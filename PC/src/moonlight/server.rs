//! Moonlight/GameStream protocol server: STUB.
//!
//! Only binds the HTTP port so port conflicts (e.g. a running Sunshine) show
//! up in the GUI. Every connection is dropped. The real implementation goes
//! where the TODOs are:
//!
//!   TODO(identity)  generate/load RSA-2048 cert + unique id in `config_dir()`
//!   TODO(mdns)      advertise `_nvstream._tcp` on `http_port`
//!   TODO(http)      /serverinfo, /applist, /launch, /resume, /cancel, /unpair
//!   TODO(pair)      /pair: call `shared.request_pairing(..)`, block on the
//!                   receiver, derive the AES key from salt + PIN, finish the
//!                   handshake, then `shared.add_paired(..)`
//!   TODO(https)     rustls on 47984 pinned to `shared.paired_clients()`
//!   TODO(rtsp)      TCP 48010
//!   TODO(session)   on /launch: `shared.set_session(Some(..))`, create the
//!                   headless monitor, start capture; on /cancel clear it
//!
//! Dev aid: run with `HYPRSTREAM_FAKE_PAIR=1` to make the host raise a fake
//! pairing request 2 s after start, so the PIN dialog can be tested.

use super::{unix_now, HostShared, HostStatus, PairDecision, PairedClient};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct HostConfig {
    /// Name shown in Moonlight's host list.
    pub hostname: String,
    pub http_port: u16,
}

impl Default for HostConfig {
    fn default() -> Self {
        Self {
            hostname: "hyprstream".into(),
            http_port: super::DEFAULT_HTTP_PORT,
        }
    }
}

pub struct HostServer {
    shared: HostShared,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl HostServer {
    pub fn start(shared: HostShared, cfg: HostConfig) -> Result<Self, String> {
        shared.set_status(HostStatus::Starting);

        let listener = TcpListener::bind(("0.0.0.0", cfg.http_port)).map_err(|e| {
            let msg = format!(
                "cannot bind TCP {} (is Sunshine running?): {e}",
                cfg.http_port
            );
            shared.set_status(HostStatus::Error(msg.clone()));
            msg
        })?;
        listener
            .set_nonblocking(true)
            .map_err(|e| format!("set_nonblocking: {e}"))?;

        let stop = Arc::new(AtomicBool::new(false));
        let stop_t = stop.clone();
        let shared_t = shared.clone();
        let fake = std::env::var_os("HYPRSTREAM_FAKE_PAIR").is_some();

        let handle = thread::Builder::new()
            .name("hyprstream-host".into())
            .spawn(move || {
                shared_t.set_status(HostStatus::Listening);
                log::info!("host '{}' listening on TCP {}", cfg.hostname, cfg.http_port);

                let started = Instant::now();
                let mut fake_rx: Option<mpsc::Receiver<PairDecision>> = None;
                let mut fake_done = false;

                while !stop_t.load(Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((conn, addr)) => {
                            log::warn!("connection from {addr}: protocol handlers not implemented");
                            drop(conn);
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(100));
                        }
                        Err(e) => {
                            log::warn!("accept error: {e}");
                            thread::sleep(Duration::from_millis(100));
                        }
                    }

                    if fake && !fake_done {
                        if fake_rx.is_none() && started.elapsed() > Duration::from_secs(2) {
                            fake_rx = Some(
                                shared_t.request_pairing("Fake Moonlight client", "FAKE-ID-0001"),
                            );
                        }
                        if let Some(rx) = &fake_rx {
                            match rx.try_recv() {
                                Ok(PairDecision::Pin(pin)) => {
                                    log::info!("fake pairing accepted with PIN {pin}");
                                    shared_t.add_paired(PairedClient {
                                        unique_id: "FAKE-ID-0001".into(),
                                        name: "Fake Moonlight client".into(),
                                        cert_pem: String::new(),
                                        paired_at: unix_now(),
                                    });
                                    fake_done = true;
                                }
                                Ok(PairDecision::Reject) => fake_done = true,
                                Err(mpsc::TryRecvError::Empty) => {}
                                Err(mpsc::TryRecvError::Disconnected) => fake_done = true,
                            }
                        }
                    }
                }

                shared_t.set_status(HostStatus::Stopped);
            })
            .map_err(|e| format!("spawn host thread: {e}"))?;

        Ok(Self {
            shared,
            stop,
            handle: Some(handle),
        })
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
        self.shared.reject_pairing();
        self.shared.set_session(None);
        self.shared.set_status(HostStatus::Stopped);
    }
}

impl Drop for HostServer {
    fn drop(&mut self) {
        self.stop();
    }
}
