use std::io::Read;
use std::net::{ToSocketAddrs, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use memchr::memmem;

const READ_CHUNK: usize = 64 * 1024;
pub const MAX_PAYLOAD: usize = 1300;
const HEADER_LEN: usize = 8;

pub struct Packetizer {
    handle: Option<JoinHandle<()>>,
    stop: Arc<AtomicBool>,
}

impl Packetizer {
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
    let mut pending: Vec<u8> = Vec::with_capacity(READ_CHUNK * 2);
    let mut read_buf = vec![0u8; READ_CHUNK];
    let mut frame_id: u32 = 0;

    loop {
        if stop.load(Ordering::Relaxed) {
            break;
        }

        let n = match reader.read(&mut read_buf) {
            Ok(0) => break,
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

fn find_start_codes(data: &[u8]) -> Vec<(usize, usize)> {
    let mut result = Vec::new();
    let finder = memmem::Finder::new(&[0, 0, 1]);
    let mut search_from = 0;

    while let Some(pos) = finder.find(&data[search_from..]) {
        let abs_pos = search_from + pos;
        let (sc_start, payload_start) = if abs_pos > 0 && data[abs_pos - 1] == 0 {
            (abs_pos - 1, abs_pos + 3)
        } else {
            (abs_pos, abs_pos + 3)
        };
        result.push((sc_start, payload_start));
        search_from = abs_pos + 3;
    }

    result
}

fn extract_complete_units(pending: &[u8]) -> (Vec<(usize, usize)>, usize) {
    let starts = find_start_codes(pending);
    if starts.len() < 2 {
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
