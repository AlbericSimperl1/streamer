//! RTSP server voor GameStream stream negotiation op poort 48010.
//!
//! Moonlight stuurt hier RTSP-berichten na /launch. De server moet
//! antwoorden met de juiste SDP (Session Description Protocol) en
//! vervolgens de UDP-streams opzetten.

use crate::moonlight::{HostShared, HostStatus};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub struct RtspServer {
    stop: Arc<AtomicBool>,
    shared: HostShared,
}

impl RtspServer {
    pub fn new(stop: Arc<AtomicBool>, shared: HostShared) -> Self {
        Self { stop, shared }
    }

    pub fn run(&self, listener: TcpListener) {
        let _ = listener.set_nonblocking(true);

        loop {
            if self.stop.load(Ordering::Relaxed) {
                break;
            }

            match listener.accept() {
                Ok((conn, _addr)) => {
                    let _ = conn.set_nonblocking(false);
                    let shared = self.shared.clone();
                    std::thread::spawn(move || {
                        handle_rtsp_client(conn, shared);
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(e) => {
                    log::warn!("RTSP accept error: {e}");
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        }
    }
}

fn handle_rtsp_client(mut conn: TcpStream, shared: HostShared) {
    let _ = conn.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = conn.set_write_timeout(Some(Duration::from_secs(5)));

    let mut buf = [0u8; 4096];
    let n = match conn.read(&mut buf) {
        Ok(n) => n,
        Err(_) => return,
    };

    let request = String::from_utf8_lossy(&buf[..n]);
    log::info!("RTSP request:\n{request}");

    // Bepaal de methode
    let method = request
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().next())
        .unwrap_or("");

    match method {
        "OPTIONS" => {
            let resp = "RTSP/1.0 200 OK\r\n\
                        CSeq: 1\r\n\
                        Public: DESCRIBE, SETUP, PLAY, TEARDOWN\r\n\
                        \r\n";
            let _ = conn.write_all(resp.as_bytes());
        }
        "DESCRIBE" => {
            // Stuur een minimale SDP met H.264 video en Opus audio.
            let sdp = "v=0\r\n\
                       o=- 0 0 IN IP4 0.0.0.0\r\n\
                       s=HyprStream\r\n\
                       t=0 0\r\n\
                       m=video 47998 RTP/AVP 96\r\n\
                       a=rtpmap:96 H264/90000\r\n\
                       a=fmtp:96 packetization-mode=1\r\n\
                       m=audio 48000 RTP/AVP 97\r\n\
                       a=rtpmap:97 OPUS/48000/2\r\n";
            let resp = format!(
                "RTSP/1.0 200 OK\r\n\
                 CSeq: 1\r\n\
                 Content-Type: application/sdp\r\n\
                 Content-Length: {}\r\n\
                 \r\n{}",
                sdp.len(),
                sdp
            );
            let _ = conn.write_all(resp.as_bytes());
        }
        "SETUP" => {
            // Client wil een stream opzetten. We antwoorden met de
            // UDP-poorten die we gaan gebruiken.
            let resp = "RTSP/1.0 200 OK\r\n\
                        CSeq: 2\r\n\
                        Session: 1\r\n\
                        Transport: RTP/AVP/UDP;unicast;client_port=47998-47999\r\n\
                        \r\n";
            let _ = conn.write_all(resp.as_bytes());
        }
        "PLAY" => {
            let resp = "RTSP/1.0 200 OK\r\n\
                        CSeq: 3\r\n\
                        Session: 1\r\n\
                        \r\n";
            let _ = conn.write_all(resp.as_bytes());

            // Start de daadwerkelijke capture en stuur frames naar de client.
            // Dit is al geïmplementeerd in de capture-module, maar moet hier
            // worden getriggerd met het juiste doeladres.
            log::info!("RTSP PLAY: stream should start now");
        }
        "TEARDOWN" => {
            let resp = "RTSP/1.0 200 OK\r\n\
                        CSeq: 4\r\n\
                        Session: 1\r\n\
                        \r\n";
            let _ = conn.write_all(resp.as_bytes());
            shared.set_session(None);
        }
        _ => {
            let resp = "RTSP/1.0 400 Bad Request\r\n\r\n";
            let _ = conn.write_all(resp.as_bytes());
        }
    }
}
