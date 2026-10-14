//! State shared between the egui GUI and the Moonlight/GameStream protocol
//! server.
//!
//! The GUI never talks to the protocol code directly. It only reads/writes a
//! `HostShared`, and the protocol server does the same from its own threads.
//!
//! Pairing flow (this is what replaces Sunshine's web UI):
//!   1. Moonlight starts pairing and shows a 4-digit PIN to the user.
//!   2. The HTTP handler for `/pair?phrase=getservercert` calls
//!      `HostShared::request_pairing(name, unique_id)` and blocks on the
//!      returned receiver.
//!   3. The GUI shows a dialog, the user types the PIN shown in Moonlight.
//!   4. The GUI calls `submit_pin`, the handler receives `PairDecision::Pin`,
//!      derives the AES key from salt+PIN and continues the handshake.
//!   5. On success the handler calls `add_paired` with the client certificate.

pub mod http;
pub mod identity;
pub mod mdns;
pub mod server;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

pub const DEFAULT_HTTP_PORT: u16 = 47989;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PairedClient {
    pub unique_id: String,
    pub name: String,
    /// PEM of the client certificate, used to pin the HTTPS (47984) connection.
    pub cert_pem: String,
    /// Unix seconds.
    pub paired_at: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum HostStatus {
    Stopped,
    Starting,
    Listening,
    Error(String),
}

#[derive(Clone, Debug)]
pub struct SessionInfo {
    pub client_name: String,
    pub client_addr: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bitrate_kbps: u32,
    pub started: SystemTime,
}

#[derive(Debug, PartialEq)]
pub enum PairDecision {
    Pin(String),
    Reject,
}

struct Pending {
    client_name: String,
    unique_id: String,
    reply: mpsc::Sender<PairDecision>,
}

struct Inner {
    status: HostStatus,
    paired: Vec<PairedClient>,
    pending: Option<Pending>,
    session: Option<SessionInfo>,
}

#[derive(Clone)]
pub struct HostShared {
    inner: Arc<Mutex<Inner>>,
}

impl HostShared {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                status: HostStatus::Stopped,
                paired: load_paired(),
                pending: None,
                session: None,
            })),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    // status / session

    pub fn status(&self) -> HostStatus {
        self.lock().status.clone()
    }

    pub fn set_status(&self, s: HostStatus) {
        self.lock().status = s;
    }

    pub fn session(&self) -> Option<SessionInfo> {
        self.lock().session.clone()
    }

    pub fn set_session(&self, s: Option<SessionInfo>) {
        self.lock().session = s;
    }

    // pairing: server side

    /// Called by the protocol server when a client starts pairing.
    /// Any older pending request is rejected.
    pub fn request_pairing(
        &self,
        client_name: &str,
        unique_id: &str,
    ) -> mpsc::Receiver<PairDecision> {
        let (tx, rx) = mpsc::channel();
        let mut g = self.lock();
        if let Some(old) = g.pending.take() {
            let _ = old.reply.send(PairDecision::Reject);
        }
        g.pending = Some(Pending {
            client_name: client_name.to_string(),
            unique_id: unique_id.to_string(),
            reply: tx,
        });
        rx
    }

    /// Called by the server when the pairing attempt times out or fails.
    pub fn cancel_pairing(&self, unique_id: &str) {
        let mut g = self.lock();
        if g.pending
            .as_ref()
            .map_or(false, |p| p.unique_id == unique_id)
        {
            g.pending = None;
        }
    }

    pub fn add_paired(&self, client: PairedClient) {
        let mut g = self.lock();
        g.paired.retain(|c| c.unique_id != client.unique_id);
        g.paired.push(client);
        save_paired(&g.paired);
    }

    pub fn is_paired(&self, unique_id: &str) -> bool {
        self.lock().paired.iter().any(|c| c.unique_id == unique_id)
    }

    // pairing: GUI side

    /// `(client_name, unique_id)` of the request waiting for a PIN, if any.
    pub fn pending_pairing(&self) -> Option<(String, String)> {
        self.lock()
            .pending
            .as_ref()
            .map(|p| (p.client_name.clone(), p.unique_id.clone()))
    }

    /// Returns false if the PIN is malformed or nothing is pending.
    pub fn submit_pin(&self, pin: &str) -> bool {
        if pin.len() != 4 || !pin.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
        match self.lock().pending.take() {
            Some(p) => p.reply.send(PairDecision::Pin(pin.to_string())).is_ok(),
            None => false,
        }
    }

    pub fn reject_pairing(&self) {
        if let Some(p) = self.lock().pending.take() {
            let _ = p.reply.send(PairDecision::Reject);
        }
    }

    pub fn paired_clients(&self) -> Vec<PairedClient> {
        self.lock().paired.clone()
    }

    pub fn unpair(&self, unique_id: &str) {
        let mut g = self.lock();
        g.paired.retain(|c| c.unique_id != unique_id);
        save_paired(&g.paired);
    }
}

impl Default for HostShared {
    fn default() -> Self {
        Self::new()
    }
}

// persistence

pub fn config_dir() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| {
                let mut p = PathBuf::from(h);
                p.push(".config");
                p
            })
        })
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    base.join("hyprstream")
}

fn paired_path() -> PathBuf {
    config_dir().join("paired_clients.json")
}

fn load_paired() -> Vec<PairedClient> {
    std::fs::read_to_string(paired_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_paired(clients: &[PairedClient]) {
    let dir = config_dir();
    if let Err(e) = std::fs::create_dir_all(&dir) {
        log::warn!("could not create {dir:?}: {e}");
        return;
    }
    match serde_json::to_string_pretty(clients) {
        Ok(json) => {
            if let Err(e) = std::fs::write(paired_path(), json) {
                log::warn!("could not save paired clients: {e}");
            }
        }
        Err(e) => log::warn!("could not serialize paired clients: {e}"),
    }
}

pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pin_roundtrip() {
        let h = HostShared::new();
        let rx = h.request_pairing("iPad", "abc");
        assert_eq!(h.pending_pairing(), Some(("iPad".into(), "abc".into())));
        assert!(!h.submit_pin("12a4"));
        assert!(h.submit_pin("1234"));
        assert_eq!(rx.recv().unwrap(), PairDecision::Pin("1234".into()));
        assert!(h.pending_pairing().is_none());
    }

    #[test]
    fn new_request_rejects_old() {
        let h = HostShared::new();
        let rx1 = h.request_pairing("A", "1");
        let _rx2 = h.request_pairing("B", "2");
        assert_eq!(rx1.recv().unwrap(), PairDecision::Reject);
    }
}
