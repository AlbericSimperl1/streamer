//! NAL-aware packetizer: leest ffmpeg's rauwe Annex-B H.264-stdout,
//! herkent NAL-unit-grenzen via een SIMD subslice-zoektocht (`memchr`), en
//! verstuurt elke NAL-unit als één of meer UDP-datagrammen met een 8-byte
//! framing-header zodat de ontvanger (`IPAD/rust_core/src/udp.rs`) ze exact
//! kan reassembleren — zonder zelf nog naar startcodes te hoeven zoeken.
//!
//! **Waarom dit nodig is:** voorheen liet ffmpeg zelf de UDP-verzending doen
//! via `-f mpegts udp://...`. MPEG-TS voegt containeroverhead toe (188-byte
//! TS-packets, PAT/PMT, sync-bytes) die de Annex-B-scanner aan de
//! ontvangende kant liet struikelen over toevallige `00 00 01`-patronen in
//! die overhead — vandaar eerst de crash (kapotte SPS/PPS) en daarna de
//! "absurd veel frames"-bug (elke valse match werd als NAL-unit geteld).
//! Door ffmpeg een kale Annex-B-stream op stdout te laten zetten
//! (`-f h264 -`, zie `encoder.rs`) en zelf de framing te doen, weet de
//! ontvanger exact waar elke NAL-unit begint en eindigt.
//!
//! Wire-formaat per UDP-pakketje (alle velden big-endian):
//! ```text
//! [frame_id: u32][chunk_index: u16][total_chunks: u16][payload bytes...]
//! ```
//! `frame_id` is een oplopende teller per NAL-unit (niet per access-unit/
//! frame — SPS, PPS en elke slice krijgen elk hun eigen id). Payload is max.
//! `MAX_PAYLOAD` bytes, dus elk pakketje (incl. header) blijft ruim onder
//! een normale Ethernet-MTU (geen IP-fragmentatie).

use std::io::Read;
use std::net::{ToSocketAddrs, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use memchr::memmem;

const READ_CHUNK: usize = 64 * 1024;
pub const MAX_PAYLOAD: usize = 1300;
const HEADER_LEN: usize = 8;

/// Vast IP:poort van de iPad-ontvanger. Zelfde adres als voorheen in
/// `encoder.rs` stond ingebakken — nu is de packetizer verantwoordelijk voor
/// de verzending in plaats van ffmpeg zelf.
pub const IPAD_ADDR: &str = "192.168.0.119:5000";

pub struct Packetizer {
    handle: Option<JoinHandle<()>>,
    stop: Arc<AtomicBool>,
}

impl Packetizer {
    /// Start de packetizer-thread. Leest uit `reader` (typisch ffmpeg's
    /// `ChildStdout`) tot EOF of tot `stop()` aangeroepen wordt.
    pub fn start<R, A>(
        mut reader: R,
        target_addr: A,
        stop_flag: Arc<AtomicBool>,
    ) -> Result<Self, String>
    where
        R: Read + Send + 'static,
        A: ToSocketAddrs,
    {
        let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| format!("UDP bind failed: {e}"))?;
        socket
            .connect(target_addr)
            .map_err(|e| format!("UDP connect failed: {e}"))?;

        let stop = stop_flag;
        let stop_clone = stop.clone();

        let handle = thread::Builder::new()
            .name("hyprpad-packetizer".into())
            .spawn(move || packetize_loop(&mut reader, &socket, &stop_clone))
            .map_err(|e| format!("Failed to spawn packetizer thread: {e}"))?;

        Ok(Self {
            handle: Some(handle),
            stop,
        })
    }

    /// Signaleer de thread te stoppen en wacht tot hij afgesloten is. Roep
    /// dit pas aan NADAT `Encoder::finish()` ffmpeg's stdin gesloten heeft
    /// (ffmpeg flusht dan en sluit stdout, waarna de leeslus vanzelf al op
    /// EOF stopt — deze call is vooral om netjes te joinen).
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for Packetizer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

fn packetize_loop<R: Read>(reader: &mut R, socket: &UdpSocket, stop: &Arc<AtomicBool>) {
    // Accumulerende buffer voor bytes die nog niet tot een complete NAL-unit
    // hebben geleid — bewaart overblijfselen van de vorige read, net zoals
    // de oude Annex-B parser dat aan de ontvangst-kant deed.
    let mut pending: Vec<u8> = Vec::with_capacity(READ_CHUNK * 2);
    let mut read_buf = vec![0u8; READ_CHUNK];
    let mut frame_id: u32 = 0;

    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }

        let n = match reader.read(&mut read_buf) {
            Ok(0) => break, // EOF — ffmpeg is gestopt (stdin gesloten via finish()).
            Ok(n) => n,
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => {
                log::warn!("packetizer: stdout read error: {e}");
                break;
            }
        };

        pending.extend_from_slice(&read_buf[..n]);

        let (units, drain_upto) = extract_complete_units(&pending);
        for (start, end) in units {
            send_nal(socket, &pending[start..end], &mut frame_id);
        }
        if drain_upto > 0 {
            pending.drain(..drain_upto);
        }
    }
}

/// Vind alle Annex-B startcodes (3- of 4-byte variant) in `data` via een
/// snelle (SIMD) subslice-zoektocht. Retourneert per gevonden startcode
/// `(sc_start, payload_start)`:
/// - `sc_start`: index van het eerste `0x00` van de startcode zelf.
/// - `payload_start`: index van het eerste byte ná de startcode (de
///   NAL-header-byte, begin van de eigenlijke NAL-unit-payload).
fn find_start_codes(data: &[u8]) -> Vec<(usize, usize)> {
    let mut result = Vec::new();
    let finder = memmem::Finder::new(&[0, 0, 1]);
    let mut search_from = 0;

    while let Some(pos) = finder.find(&data[search_from..]) {
        let abs_pos = search_from + pos;
        let (sc_start, payload_start) = if abs_pos > 0 && data[abs_pos - 1] == 0 {
            (abs_pos - 1, abs_pos + 3) // 4-byte variant: 00 00 00 01
        } else {
            (abs_pos, abs_pos + 3) // 3-byte variant: 00 00 01
        };
        result.push((sc_start, payload_start));
        search_from = abs_pos + 3;
    }

    result
}

/// Geef de complete NAL-unit-ranges terug die uit `pending` gehaald kunnen
/// worden, plus de offset waarop `pending` gedraineerd moet worden vóór de
/// volgende read. De laatste (mogelijk nog incomplete) unit blijft altijd
/// bewaard voor de volgende iteratie — precies zoals de oude Rust-kant
/// parser dat deed, alleen dan hier aan de zender-kant.
fn extract_complete_units(pending: &[u8]) -> (Vec<(usize, usize)>, usize) {
    let starts = find_start_codes(pending);
    if starts.len() < 2 {
        // Nog geen enkele complete NAL-unit — gewoon meer lezen.
        return (Vec::new(), 0);
    }

    let mut units = Vec::with_capacity(starts.len() - 1);
    for i in 0..starts.len() - 1 {
        let payload_start = starts[i].1;
        let payload_end = starts[i + 1].0;
        if payload_end > payload_start {
            units.push((payload_start, payload_end));
        }
    }

    (units, starts[starts.len() - 1].0)
}

/// Knip één NAL-unit in chunks van max. `MAX_PAYLOAD` bytes en verstuur elk
/// stuk met zijn framing-header.
fn send_nal(socket: &UdpSocket, nal: &[u8], frame_id: &mut u32) {
    if nal.is_empty() {
        return;
    }
    let id = *frame_id;
    *frame_id = frame_id.wrapping_add(1);

    let total_chunks = nal.chunks(MAX_PAYLOAD).count() as u16;
    for (idx, chunk) in nal.chunks(MAX_PAYLOAD).enumerate() {
        let mut packet = Vec::with_capacity(HEADER_LEN + chunk.len());
        packet.extend_from_slice(&id.to_be_bytes());
        packet.extend_from_slice(&(idx as u16).to_be_bytes());
        packet.extend_from_slice(&total_chunks.to_be_bytes());
        packet.extend_from_slice(chunk);

        if let Err(e) = socket.send(&packet) {
            log::warn!("packetizer: send failed: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::net::UdpSocket as StdUdpSocket;
    use std::time::Duration;

    fn sc3(payload: &[u8]) -> Vec<u8> {
        let mut v = vec![0, 0, 1];
        v.extend_from_slice(payload);
        v
    }

    fn sc4(payload: &[u8]) -> Vec<u8> {
        let mut v = vec![0, 0, 0, 1];
        v.extend_from_slice(payload);
        v
    }

    #[test]
    fn finds_3_and_4byte_startcodes() {
        let mut data = Vec::new();
        data.extend(sc4(b"SPS"));
        data.extend(sc3(b"PPS"));
        let starts = find_start_codes(&data);
        assert_eq!(starts, vec![(0, 4), (7, 10)]);
    }

    #[test]
    fn streaming_across_fragmented_reads() {
        // Simuleer ffmpeg dat in extreem kleine brokken naar stdout schrijft.
        let mut full = Vec::new();
        full.extend(sc4(b"SPSDATA"));
        full.extend(sc3(b"PPSDATA"));
        full.extend(sc3(b"IDRDATA-LONGER-SLICE"));
        full.extend(sc3(b"PSLICE"));

        let mut pending: Vec<u8> = Vec::new();
        let mut emitted: Vec<Vec<u8>> = Vec::new();

        for chunk in full.chunks(3) {
            pending.extend_from_slice(chunk);
            let (units, drain_upto) = extract_complete_units(&pending);
            for (s, e) in &units {
                emitted.push(pending[*s..*e].to_vec());
            }
            if drain_upto > 0 {
                pending.drain(..drain_upto);
            }
        }

        assert_eq!(emitted.len(), 3); // laatste NAL (PSLICE) blijft bewust "hangen"
        assert_eq!(emitted[0], b"SPSDATA");
        assert_eq!(emitted[1], b"PPSDATA");
        assert_eq!(emitted[2], b"IDRDATA-LONGER-SLICE");
    }

    #[test]
    fn large_nal_is_chunked_under_mtu() {
        let nal: Vec<u8> = (0..3500u32).map(|i| (i % 256) as u8).collect();
        let receiver = StdUdpSocket::bind("127.0.0.1:0").unwrap();
        receiver
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let addr = receiver.local_addr().unwrap();
        let sender = StdUdpSocket::bind("127.0.0.1:0").unwrap();
        sender.connect(addr).unwrap();

        let mut frame_id = 0u32;
        send_nal(&sender, &nal, &mut frame_id);

        let mut reassembled = Vec::new();
        let mut buf = [0u8; 65536];
        let mut total_chunks_seen = None;
        for _ in 0..3 {
            let n = receiver.recv(&mut buf).unwrap();
            assert!(n <= HEADER_LEN + MAX_PAYLOAD);
            let total_chunks = u16::from_be_bytes([buf[6], buf[7]]);
            total_chunks_seen = Some(total_chunks);
            reassembled.extend_from_slice(&buf[HEADER_LEN..n]);
        }
        assert_eq!(total_chunks_seen, Some(3));
        assert_eq!(reassembled, nal);
    }

    #[test]
    fn end_to_end_over_real_loopback_socket() {
        let receiver = StdUdpSocket::bind("127.0.0.1:0").unwrap();
        receiver
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let recv_addr = receiver.local_addr().unwrap();

        let mut fake_stream = Vec::new();
        fake_stream.extend(sc3(&[0x67, b'S', b'P', b'S'])); // SPS
        fake_stream.extend(sc3(&[0x68, b'P', b'P', b'S'])); // PPS
        let mut big_slice = vec![0x65u8]; // IDR
        big_slice.extend(std::iter::repeat(0xAB).take(3000));
        fake_stream.extend(sc3(&big_slice));
        fake_stream.extend(sc3(b"X")); // sluit de vorige (grote) unit af

        let reader = Cursor::new(fake_stream);
        let stop = Arc::new(AtomicBool::new(false));
        let mut packetizer = Packetizer::start(reader, recv_addr, stop).expect("start");

        use std::collections::HashMap;
        let mut by_frame: HashMap<u32, Vec<(u16, u16, Vec<u8>)>> = HashMap::new();
        let mut buf = [0u8; 65536];
        loop {
            match receiver.recv(&mut buf) {
                Ok(n) => {
                    let frame_id = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
                    let chunk_index = u16::from_be_bytes([buf[4], buf[5]]);
                    let total_chunks = u16::from_be_bytes([buf[6], buf[7]]);
                    by_frame.entry(frame_id).or_default().push((
                        chunk_index,
                        total_chunks,
                        buf[HEADER_LEN..n].to_vec(),
                    ));
                }
                Err(_) => break,
            }
        }
        packetizer.stop();

        assert_eq!(by_frame.len(), 3);
        assert_eq!(by_frame[&0][0].2, vec![0x67, b'S', b'P', b'S']);
        let mut idr = by_frame[&2].clone();
        idr.sort_by_key(|(idx, _, _)| *idx);
        let reassembled: Vec<u8> = idr.iter().flat_map(|(_, _, p)| p.clone()).collect();
        assert_eq!(reassembled, big_slice);
    }
}
