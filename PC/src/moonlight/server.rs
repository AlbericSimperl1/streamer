//! Moonlight/GameStream protocol server.
//!
//! Start three listeners:
//!   - Plain HTTP on 47989 for /serverinfo
//!   - HTTPS on 47984 (mTLS) for /pair, /applist, /launch, /resume, /cancel
//!   - RTSP on 48010 for stream negotiation
//!
//! De HTTPS-server zelf (met de mTLS-configuratie) staat in `http.rs`
//! in de functie `run_https_server`.

use super::http::{self, HttpCtx};
use super::identity;
use super::mdns::MdnsAdvertiser;
use super::{
    unix_now, HostShared, HostStatus, PairDecision, PairedClient, DEFAULT_HTTPS_PORT,
    DEFAULT_HTTP_PORT, DEFAULT_RTSP_PORT,
};
use crate::rtsp::RtspServer;
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
    pub https_port: u16,
    pub rtsp_port: u16,
}

impl Default for HostConfig {
    fn default() -> Self {
        Self {
            hostname: "hyprstream".into(),
            http_port: DEFAULT_HTTP_PORT,
            https_port: DEFAULT_HTTPS_PORT,
            rtsp_port: DEFAULT_RTSP_PORT,
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

        let fail = |msg: String| {
            shared.set_status(HostStatus::Error(msg.clone()));
            msg
        };

        let identity = identity::load_or_create().map_err(|e| fail(format!("identity: {e}")))?;

        // --- HTTP listener (47989) ---
        let http_listener = TcpListener::bind(("0.0.0.0", cfg.http_port)).map_err(|e| {
            fail(format!(
                "cannot bind TCP {} (is Sunshine running?): {e}",
                cfg.http_port
            ))
        })?;
        http_listener
            .set_nonblocking(true)
            .map_err(|e| fail(format!("set_nonblocking: {e}")))?;

        // --- HTTPS listener (47984) ---
        let https_listener = TcpListener::bind(("0.0.0.0", cfg.https_port))
            .map_err(|e| fail(format!("cannot bind HTTPS {}: {e}", cfg.https_port)))?;
        https_listener
            .set_nonblocking(true)
            .map_err(|e| fail(format!("set_nonblocking https: {e}")))?;

        // --- RTSP listener (48010) ---
        let rtsp_listener = TcpListener::bind(("0.0.0.0", cfg.rtsp_port))
            .map_err(|e| fail(format!("cannot bind RTSP {}: {e}", cfg.rtsp_port)))?;
        rtsp_listener
            .set_nonblocking(true)
            .map_err(|e| fail(format!("set_nonblocking rtsp: {e}")))?;

        let mdns = MdnsAdvertiser::start(&cfg.hostname, cfg.http_port).map_err(|e| fail(e))?;

        let ctx = Arc::new(HttpCtx {
            hostname: cfg.hostname.clone(),
            unique_id: identity.unique_id.clone(),
            http_port: cfg.http_port,
            https_port: cfg.https_port,
            rtsp_port: cfg.rtsp_port,
            shared: shared.clone(),
        });

        let stop = Arc::new(AtomicBool::new(false));
        let stop_t = stop.clone();
        let shared_t = shared.clone();
        let fake = std::env::var_os("HYPRSTREAM_FAKE_PAIR").is_some();
        let hostname_for_log = cfg.hostname.clone();

        let handle = thread::Builder::new()
            .name("hyprstream-host".into())
            .spawn(move || {
                // Houd de mDNS-advertentie in leven zolang de thread draait.
                let _mdns = mdns;

                // Maak de tokio runtime en enter de context zodat we
                // std listeners kunnen converteren.
                let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
                let _guard = rt.enter();

                // Converteer de std HTTPS listener naar tokio.
                let https_listener = match tokio::net::TcpListener::from_std(https_listener) {
                    Ok(l) => l,
                    Err(e) => {
                        log::error!("convert https listener: {e}");
                        shared_t.set_status(HostStatus::Error(format!("tokio listener: {e}")));
                        return;
                    }
                };

                // Start RTSP server in een aparte thread.
                let rtsp_stop = stop_t.clone();
                let rtsp_shared = shared_t.clone();
                let rtsp_handle = thread::spawn(move || {
                    let server = RtspServer::new(rtsp_stop, rtsp_shared);
                    server.run(rtsp_listener);
                });

                shared_t.set_status(HostStatus::Listening);
                log::info!("host '{hostname_for_log}' listening");

                // Start HTTPS server in de tokio runtime. Dit blokkeert de
                // huidige thread; HTTP-accepts worden hieronder afgehandeld
                // in een aparte std-thread zodat ze blijven werken.
                let https_stop = stop_t.clone();
                let https_ctx = ctx.clone();
                let http_listener_for_thread = http_listener;
                let http_ctx = ctx.clone();
                let http_stop = stop_t.clone();

                // Spawn een aparte thread voor de HTTP accept loop, zodat
                // rt.block_on hieronder niet alles blokkeert.
                let http_thread = thread::spawn(move || loop {
                    if http_stop.load(Ordering::Relaxed) {
                        break;
                    }
                    match http_listener_for_thread.accept() {
                        Ok((conn, _addr)) => {
                            let _ = conn.set_nonblocking(false);
                            let ctx = http_ctx.clone();
                            let _ = thread::Builder::new()
                                .name("hyprstream-http".into())
                                .spawn(move || http::handle_plain(conn, &ctx));
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(50));
                        }
                        Err(e) => {
                            log::warn!("accept error: {e}");
                            thread::sleep(Duration::from_millis(100));
                        }
                    }
                });

                // Dev-hulp: fake pairing request na 2 seconden.
                let fake_thread = if fake {
                    let shared_for_fake = shared_t.clone();
                    let stop_for_fake = stop_t.clone();
                    Some(thread::spawn(move || {
                        let started = Instant::now();
                        let mut fake_rx: Option<mpsc::Receiver<PairDecision>> = None;
                        let mut fake_done = false;
                        while !stop_for_fake.load(Ordering::Relaxed) {
                            if !fake_done
                                && fake_rx.is_none()
                                && started.elapsed() > Duration::from_secs(2)
                            {
                                fake_rx = Some(
                                    shared_for_fake
                                        .request_pairing("Fake Moonlight client", "FAKE-ID-0001"),
                                );
                            }
                            if let Some(rx) = &fake_rx {
                                match rx.try_recv() {
                                    Ok(PairDecision::Pin(pin)) => {
                                        log::info!("fake pairing accepted with PIN {pin}");
                                        shared_for_fake.add_paired(PairedClient {
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
                            thread::sleep(Duration::from_millis(200));
                        }
                    }))
                } else {
                    None
                };

                // Blokkeer op de HTTPS-server. Dit is de hoofdloop van deze thread.
                let _ = rt.block_on(async move {
                    http::run_https_server(https_listener, https_ctx, https_stop).await
                });

                // Wacht op de andere threads.
                let _ = http_thread.join();
                let _ = rtsp_handle.join();
                if let Some(h) = fake_thread {
                    let _ = h.join();
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
