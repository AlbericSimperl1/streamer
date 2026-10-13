use crate::capture;
use crate::hypr;
use crate::moonlight::server::{HostConfig, HostServer};
use crate::moonlight::{HostShared, HostStatus};
use crate::types::{LogEntry, LogLevel, MonitorConfig, MonitorJson};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, SystemTime};

use std::collections::VecDeque;

pub struct App {
    pub config: MonitorConfig,
    pub monitors: Vec<MonitorJson>,
    pub monitor_exists: bool,
    pub log_entries: Vec<LogEntry>,
    pub auto_refresh: bool,
    pub last_refresh: Option<SystemTime>,

    /// In-flight background `hyprctl monitors` query (never block the UI thread).
    refresh_rx: Option<mpsc::Receiver<Result<Vec<MonitorJson>, String>>>,

    // Moonlight host
    pub host: HostShared,
    host_server: Option<HostServer>,
    /// Name shown in Moonlight's host list.
    pub host_name: String,
    /// PIN typed into the pairing dialog.
    pub pin_input: String,

    // Capture session. Started by the /launch handler once the protocol
    // server exists; the GUI no longer starts it by hand.
    pub capture: Option<capture::CaptureSession>,
    pub capture_output_path: String,
    pub stop_result_rx: Option<mpsc::Receiver<capture::CaptureStatus>>,

    pub signal_flag: Option<Arc<AtomicBool>>,
    shutdown_done: bool,

    pub fps_history: VecDeque<f32>,
    pub current_fps: f32,
    pub stream_start_time: Option<SystemTime>,
    last_frame_count: u64,
    last_fps_calc_time: Option<SystemTime>,
}

impl App {
    pub fn with_signal_flag(flag: Arc<AtomicBool>) -> Self {
        Self::with_signal_flag_opt(Some(flag))
    }

    pub(crate) fn with_signal_flag_opt(signal_flag: Option<Arc<AtomicBool>>) -> Self {
        let mut fps_history = VecDeque::with_capacity(60);
        for _ in 0..60 {
            fps_history.push_back(0.0);
        }

        let mut app = Self {
            config: MonitorConfig::default(),
            monitors: Vec::new(),
            monitor_exists: false,
            log_entries: Vec::new(),
            auto_refresh: true,
            last_refresh: None,
            refresh_rx: None,
            host: HostShared::new(),
            host_server: None,
            host_name: default_hostname(),
            pin_input: String::new(),
            capture: None,
            capture_output_path: default_capture_path(),
            stop_result_rx: None,
            signal_flag,
            shutdown_done: false,
            fps_history,
            current_fps: 0.0,
            stream_start_time: None,
            last_frame_count: 0,
            last_fps_calc_time: None,
        };
        app.log(
            "Hyprland Virtual Display controller started.",
            LogLevel::Info,
        );
        app.refresh();
        app
    }

    fn timestamp() -> String {
        chrono::Local::now().format("%H:%M:%S").to_string()
    }

    pub fn log(&mut self, msg: impl Into<String>, level: LogLevel) {
        self.log_entries.push(LogEntry {
            time: Self::timestamp(),
            message: msg.into(),
            level,
        });
        if self.log_entries.len() > 200 {
            self.log_entries.remove(0);
        }
    }

    /// Starts a non-blocking monitor refresh. The result is applied in
    /// `poll_refresh()` (called from `tick()`).
    pub fn refresh(&mut self) {
        if self.refresh_rx.is_some() {
            return; // already in flight
        }
        let (tx, rx) = mpsc::channel();
        self.refresh_rx = Some(rx);
        let spawned = std::thread::Builder::new()
            .name("hyprpad-refresh".into())
            .spawn(move || {
                let _ = tx.send(hypr::get_monitors());
            });
        if spawned.is_err() {
            self.refresh_rx = None;
        }
    }

    fn poll_refresh(&mut self) {
        let Some(rx) = &self.refresh_rx else {
            return;
        };
        match rx.try_recv() {
            Ok(Ok(monitors)) => {
                self.refresh_rx = None;
                self.monitor_exists = monitors.iter().any(|m| m.name == self.config.name);
                self.monitors = monitors;
                self.last_refresh = Some(SystemTime::now());
            }
            Ok(Err(e)) => {
                self.refresh_rx = None;
                self.last_refresh = Some(SystemTime::now());
                self.log(format!("Failed to get monitors: {e}"), LogLevel::Error);
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                self.refresh_rx = None;
                self.last_refresh = Some(SystemTime::now());
            }
        }
    }

    pub fn tick(&mut self) {
        self.poll_refresh();

        if self.auto_refresh && self.refresh_rx.is_none() {
            let due = self.last_refresh.map_or(true, |l| {
                l.elapsed().unwrap_or_default() >= Duration::from_secs(2)
            });
            if due {
                self.refresh();
            }
        }

        let now = SystemTime::now();
        let (current_frames, is_capturing) = match self.capture.as_ref().map(|c| c.status()) {
            Some(capture::CaptureStatus::Capturing { frames, .. }) => (frames, true),
            _ => (0, false),
        };

        if !is_capturing {
            self.stream_start_time = None;
            self.current_fps = 0.0;
        } else if self.stream_start_time.is_none() {
            self.stream_start_time = Some(now);
            self.last_frame_count = current_frames;
            self.last_fps_calc_time = Some(now);
        }

        let calc_due = self.last_fps_calc_time.map_or(true, |t| {
            t.elapsed().unwrap_or_default() >= Duration::from_millis(500)
        });

        if calc_due {
            if is_capturing {
                if let Some(last_time) = self.last_fps_calc_time {
                    let elapsed_secs = last_time.elapsed().unwrap_or_default().as_secs_f32();
                    if elapsed_secs > 0.0 {
                        let delta_frames =
                            current_frames.saturating_sub(self.last_frame_count) as f32;
                        let raw_fps = delta_frames / elapsed_secs;
                        // Smooth transition
                        self.current_fps = self.current_fps * 0.4 + raw_fps * 0.6;
                    }
                }
                self.last_frame_count = current_frames;
                self.last_fps_calc_time = Some(now);
            } else {
                self.current_fps = 0.0;
            }

            self.fps_history.pop_front();
            self.fps_history.push_back(self.current_fps);

            while self.fps_history.len() > 20 {
                self.fps_history.pop_front();
            }
        }
    }

    pub fn should_quit(&self) -> bool {
        self.signal_flag
            .as_ref()
            .map_or(false, |f| f.load(Ordering::SeqCst))
    }

    pub fn is_capturing(&self) -> bool {
        self.capture.as_ref().map_or(false, |c| c.is_running())
    }

    pub fn is_stopping(&self) -> bool {
        self.stop_result_rx.is_some()
    }

    pub fn host_running(&self) -> bool {
        self.host_server.is_some()
    }

    pub fn apply_config(&mut self) {
        if !self.monitor_exists {
            if let Err(e) = hypr::create_headless_output(&self.config.name) {
                eprintln!("Fout bij aanmaken headless monitor: {e}");
                return;
            }
            self.monitor_exists = true;
        }

        if let Err(e) = hypr::apply_monitor_keyword(&self.config) {
            eprintln!("Fout bij updaten monitor configuratie: {e}");
        }

        self.refresh();
    }

    pub fn do_remove(&mut self) {
        let name = self.config.name.clone();

        self.log(
            format!("▶ Removing virtual monitor '{name}'..."),
            LogLevel::Info,
        );
        self.log(format!("  $ hyprctl output remove {name}"), LogLevel::Info);

        match hypr::remove_monitor(&name) {
            Ok(out) => {
                self.log(
                    format!("✓ Monitor '{name}' removed. {}", out.trim()),
                    LogLevel::Success,
                );
                self.refresh();
            }
            Err(e) => {
                self.log(format!("✗ Failed to remove: {e}"), LogLevel::Error);
            }
        }
    }

    // Moonlight host

    pub fn do_start_host(&mut self) {
        if self.host_server.is_some() {
            self.log("Host already running.", LogLevel::Warning);
            return;
        }
        let cfg = HostConfig {
            hostname: self.host_name.trim().to_string(),
            ..Default::default()
        };
        self.log(
            format!("▶ Starting Moonlight host '{}'...", cfg.hostname),
            LogLevel::Info,
        );
        match HostServer::start(self.host.clone(), cfg) {
            Ok(server) => {
                self.host_server = Some(server);
                self.log(
                    "✓ Host started. Add this PC in Moonlight and pair.",
                    LogLevel::Success,
                );
            }
            Err(e) => self.log(format!("✗ Failed to start host: {e}"), LogLevel::Error),
        }
    }

    pub fn do_stop_host(&mut self) {
        if self.is_capturing() {
            self.do_stop_capture();
        }
        if let Some(mut server) = self.host_server.take() {
            server.stop();
            self.log("Host stopped.", LogLevel::Info);
        }
        self.pin_input.clear();
    }

    pub fn do_submit_pin(&mut self) {
        let pin = self.pin_input.clone();
        let name = self
            .host
            .pending_pairing()
            .map(|(n, _)| n)
            .unwrap_or_default();
        if self.host.submit_pin(&pin) {
            self.log(format!("Pairing PIN sent for '{name}'."), LogLevel::Info);
        } else {
            self.log(
                "Invalid PIN or no pairing request pending.",
                LogLevel::Warning,
            );
        }
        self.pin_input.clear();
    }

    pub fn do_reject_pairing(&mut self) {
        self.host.reject_pairing();
        self.pin_input.clear();
        self.log("Pairing request rejected.", LogLevel::Warning);
    }

    pub fn do_unpair(&mut self, unique_id: &str, name: &str) {
        self.host.unpair(unique_id);
        self.log(format!("Unpaired '{name}'."), LogLevel::Info);
    }

    pub fn do_stop_capture(&mut self) {
        if let Some(mut session) = self.capture.take() {
            self.log("▶ Stopping capture...", LogLevel::Info);

            let (tx, rx) = mpsc::channel();
            self.stop_result_rx = Some(rx);

            std::thread::spawn(move || {
                session.stop();
                let final_status = session.status();
                let _ = tx.send(final_status);
            });
        }
    }

    pub fn poll_stop_result(&mut self) {
        let rx = match &self.stop_result_rx {
            Some(rx) => rx,
            None => return,
        };
        match rx.try_recv() {
            Ok(status) => {
                self.stop_result_rx = None;
                match status {
                    capture::CaptureStatus::Finished { path, frames } => {
                        self.log(
                            format!("✓ Capture ended ({frames} frames)"),
                            LogLevel::Success,
                        );
                    }
                    capture::CaptureStatus::Error(e) => {
                        self.log(format!("✗ Capture error: {e}"), LogLevel::Error);
                    }
                    _ => {
                        self.log("Capture stopped.", LogLevel::Info);
                    }
                }
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                self.stop_result_rx = None;
                self.log("✗ Capture stop thread ended unexpectedly.", LogLevel::Error);
            }
        }
    }

    pub fn shutdown(&mut self) {
        if self.shutdown_done {
            return;
        }
        self.shutdown_done = true;

        if let Some(mut server) = self.host_server.take() {
            server.stop();
        }
        if let Some(mut session) = self.capture.take() {
            session.stop();
            println!("✓ Capture sessie gestopt bij afsluiten.");
        }
        if let Some(rx) = self.stop_result_rx.take() {
            let _ = rx.recv_timeout(std::time::Duration::from_secs(5));
        }
        if self.monitor_exists {
            let name = self.config.name.clone();
            match hypr::remove_monitor(&name) {
                Ok(_) => println!("✓ Monitor succesvol opgeruimd."),
                Err(e) => eprintln!("✗ Fout bij opruimen monitor bij afsluiten: {e}"),
            }
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.host_server.take();
        if self.capture.is_some() {
            self.capture.take();
        }
        if self.monitor_exists {
            let name = self.config.name.clone();
            let _ = hypr::remove_monitor(&name);
        }
    }
}

fn default_hostname() -> String {
    std::fs::read_to_string("/etc/hostname")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("HOSTNAME").ok())
        .unwrap_or_else(|| "hyprstream".into())
}

fn default_capture_path() -> String {
    let mut p = dirs_or_tmp();
    p.push("hyprpad_capture.mp4");
    p.to_string_lossy().into_owned()
}

fn dirs_or_tmp() -> std::path::PathBuf {
    if let Some(d) = std::env::var_os("XDG_VIDEOS_DIR") {
        return std::path::PathBuf::from(d);
    }
    if let Some(home) = std::env::var_os("HOME") {
        let mut p = std::path::PathBuf::from(home);
        p.push("Videos");
        if p.exists() {
            return p;
        }
    }
    std::path::PathBuf::from("/tmp")
}
