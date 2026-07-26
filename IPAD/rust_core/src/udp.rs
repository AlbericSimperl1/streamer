//! UDP-listener + NAL-reassemblage voor poort 5000.
//!
//! **Waarom dit anders is dan de vorige versie:** de vorige `udp.rs` schreef
//! ruwe binnenkomende UDP-bytes direct in een lock-free ring, waarna een
//! aparte `parser`-thread die ring aftastte op Annex-B-startcodes
//! (`00 00 01`) om NAL-units te vinden. Dat werkte alleen zolang de bytes
//! die binnenkwamen ook echt een kale Annex-B elementary stream waren.
//!
//! Sinds de PC-kant nu zelf een NAL-aware packetizer heeft
//! (`PC/src/capture/packetizer.rs`) die ffmpeg's rauwe H.264-stdout al
//! opknipt in chunks mét een expliciete header
//! (`frame_id`/`chunk_index`/`total_chunks`), hoeft deze kant geen Annex-B
//! meer te scannen: elk UDP-datagram draagt exact genoeg informatie om het
//! weer op de juiste plek te leggen. Dat maakt de ring en de losse
//! parser-thread overbodig — de reassemblage gebeurt nu direct op de
//! UDP-thread zelf (goedkoop genoeg: een paar HashMap-operaties per
//! pakketje, geen SIMD-scan meer nodig aan deze kant).
//!
//! Dit is ook meteen de fix voor "absurd veel frames": de oude scanner zag
//! valse startcodes in MPEG-TS-overhead (sync-bytes, adaptation fields,
//! PSI/PAT/PMT-tabellen) omdat ffmpeg destijds via `-f mpegts udp://...`
//! verzond. Nu ffmpeg alleen nog een kale Annex-B-stream op stdout zet — die
//! de packetizer zelf en éénmalig opdeelt — kan de ontvanger nooit meer een
//! NAL-grens "verzinnen" die er niet is.

use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Instant;

use crate::nal::{NAL_IDR, NAL_NON_IDR};
use crate::reassembler::Reassembler;
use crate::stats::{State, Stats};

const READ_BUF_SIZE: usize = 65_536; // ruim boven de 1300+8 byte packetizer-chunks

pub struct UdpReceiver {
    handle: Option<JoinHandle<()>>,
    stop: Arc<AtomicBool>,
}

impl UdpReceiver {
    /// Start het luisteren op `port`. Elk binnenkomend datagram wordt direct
    /// door de `Reassembler` gehaald; zodra die een complete NAL-unit
    /// teruggeeft, wordt `on_nalu` aangeroepen (zelfde contract als
    /// voorheen: de pointer is alleen geldig tijdens de call).
    pub fn start<L, N>(
        port: u16,
        stats: Arc<Stats>,
        mut on_log: L,
        mut on_nalu: N,
    ) -> std::io::Result<Self>
    where
        L: FnMut(u8, &str) + Send + 'static,
        N: FnMut(*const u8, u32, u8) + Send + 'static,
    {
        let socket = UdpSocket::bind(("0.0.0.0", port))?;
        // 250ms timeout zodat we de stop-flag regelmatig checken, ook als er
        // (nog) geen data binnenkomt.
        socket.set_read_timeout(Some(std::time::Duration::from_millis(250)))?;
        socket.set_broadcast(true)?;

        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = stop.clone();

        let handle = thread::Builder::new()
            .name("hyprpad-udp".into())
            .spawn(move || {
                on_log(0, &format!("UDP listening on 0.0.0.0:{}", port));
                stats.set_state(State::Listening);

                let mut buf = [0u8; READ_BUF_SIZE];
                let mut reassembler = Reassembler::new();

                let mut frame_count: u32 = 0;
                let mut last_stats = Instant::now();

                while !stop_clone.load(Ordering::Relaxed) {
                    match socket.recv_from(&mut buf) {
                        Ok((n, _peer)) => {
                            stats.add_bytes(n as u64);
                            if stats.state() == State::Listening as u8 {
                                stats.set_state(State::Receiving);
                            }

                            if let Some(nal) = reassembler.accept(&buf[..n]) {
                                if let Some(&first_byte) = nal.first() {
                                    let nal_type = first_byte & 0x1F;
                                    if nal_type == NAL_IDR || nal_type == NAL_NON_IDR {
                                        frame_count += 1;
                                    }
                                    on_nalu(nal.as_ptr(), nal.len() as u32, nal_type);
                                }
                            }
                        }
                        Err(ref e)
                            if e.kind() == std::io::ErrorKind::WouldBlock
                                || e.kind() == std::io::ErrorKind::TimedOut =>
                        {
                            continue;
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {
                            continue;
                        }
                        Err(ref e) => {
                            on_log(2, &format!("UDP fout: {}", e));
                            stats.set_state(State::Error);
                            break;
                        }
                    }

                    // FPS ~1x/seconde — geteld op basis van daadwerkelijk
                    // gereassembleerde IDR/non-IDR NAL-units, niet op ruwe
                    // startcode-hits (dat was precies de bron van de
                    // "absurd veel frames"-bug).
                    let elapsed = last_stats.elapsed();
                    if elapsed >= std::time::Duration::from_secs(1) {
                        let fps = (frame_count as f64 / elapsed.as_secs_f64()).round() as u32;
                        stats.set_fps(fps);
                        frame_count = 0;
                        last_stats = Instant::now();
                    }
                }

                stats.set_state(State::Idle);
            })?;

        Ok(Self {
            handle: Some(handle),
            stop,
        })
    }
}

impl Drop for UdpReceiver {
    fn drop(&mut self) {
        // Voorheen ontbrak deze stop-signalering: de thread stopte alleen op
        // een echte socket-fout, dus `hyprpad_stop()` kon in theorie
        // blijven hangen op de `join()`. Nu zet Drop expliciet de vlag,
        // waarna de volgende recv_timeout de loop netjes laat afsluiten.
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}
