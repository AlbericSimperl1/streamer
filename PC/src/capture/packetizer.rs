//! Verstuurt de rauwe H.264 Annex-B bytestream (uit ffmpeg's stdout) als
//! gesequenced UDP over het netwerk.
//!
//! Elk datagram krijgt een 4-byte, big-endian sequence-nummer vooraan. Dit is
//! bewust GEEN volledige RTP-implementatie — enkel genoeg framing zodat de
//! ontvanger (iPad) reordering en packet loss kan detecteren in plaats van
//! de bytestream blind aan elkaar te plakken (wat corrupte NAL-grenzen gaf).

use std::io::Read;
use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

/// Ruim onder de typische MTU (1500) min IP/UDP-headers (28) min onze eigen
/// 4-byte sequence-header. 1300 laat marge voor netwerken met iets kleinere
/// MTU (bv. sommige VPN's of WiFi-extenders).
const MAX_PAYLOAD: usize = 1300;

pub struct Packetizer {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Packetizer {
    /// Start een achtergrondthread die uit `reader` leest (typisch
    /// `child.stdout` van ffmpeg) en elk gelezen blok als apart UDP-datagram
    /// met sequence-header naar `dest` stuurt.
    pub fn start<R>(mut reader: R, dest: String) -> Self
    where
        R: Read + Send + 'static,
    {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = stop.clone();

        let handle = thread::Builder::new()
            .name("hyprpad-packetizer".into())
            .spawn(move || {
                let sock = match UdpSocket::bind("0.0.0.0:0") {
                    Ok(s) => s,
                    Err(e) => {
                        log::error!("packetizer: bind mislukt: {e}");
                        return;
                    }
                };
                if let Err(e) = sock.connect(&dest) {
                    log::error!("packetizer: connect naar {dest} mislukt: {e}");
                    return;
                }

                let mut seq: u32 = 0;
                let mut buf = vec![0u8; MAX_PAYLOAD];

                while !stop_clone.load(Ordering::Relaxed) {
                    let n = match reader.read(&mut buf) {
                        Ok(0) => {
                            // EOF: ffmpeg heeft stdout gesloten (proces gestopt).
                            log::info!("packetizer: EOF van ffmpeg, thread stopt");
                            break;
                        }
                        Ok(n) => n,
                        Err(e) => {
                            log::error!("packetizer: leesfout van ffmpeg stdout: {e}");
                            break;
                        }
                    };

                    let mut packet = Vec::with_capacity(4 + n);
                    packet.extend_from_slice(&seq.to_be_bytes());
                    packet.extend_from_slice(&buf[..n]);

                    if let Err(e) = sock.send(&packet) {
                        log::warn!("packetizer: send mislukt (seq {seq}): {e}");
                    }

                    seq = seq.wrapping_add(1);
                }
            })
            .expect("packetizer-thread spawn");

        Self {
            stop,
            handle: Some(handle),
        }
    }
}

impl Drop for Packetizer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        // We joinen NIET met een blocking read erin vast (reader.read() kan
        // blokkeren tot ffmpeg iets schrijft of stdout sluit). Zodra de caller
        // ook het ffmpeg-kindproces stopt (stdin dicht/kill), sluit stdout en
        // stopt read() met Ok(0), waarna de thread zelf netjes afsluit.
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}
