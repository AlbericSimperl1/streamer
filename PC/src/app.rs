use crate::capture;
use crate::hypr;
use crate::types::{LogEntry, LogLevel, MonitorConfig, MonitorJson};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, SystemTime};

/// Centrale applicatie-state. Bevat alle data en business logic.
/// De GUI (zie `gui.rs`) leest en schrijft via pub(crate) velden en
/// roept pub-methodes aan om acties te triggeren.
pub struct App {
    pub config: MonitorConfig,
    pub monitors: Vec<MonitorJson>,
    pub monitor_exists: bool,
    pub log_entries: Vec<LogEntry>,
    pub auto_refresh: bool,
    pub last_refresh: Option<SystemTime>,

    // capture
    pub capture: Option<capture::CaptureSession>,
    pub capture_output_path: String,
    /// Receiver voor de status nadat een achtergrond-stop klaar is.
    pub stop_result_rx: Option<mpsc::Receiver<capture::CaptureStatus>>,

    /// Wordt gezet door de ctrlc signal handler in main.rs.
    /// Wanneer true, triggert de GUI-loop een graceful shutdown.
    pub signal_flag: Option<Arc<AtomicBool>>,
    /// Voorkomt dat shutdown meerdere keren wordt aangeroepen.
    shutdown_done: bool,
}

impl App {
    pub fn new() -> Self {
        Self::with_signal_flag_opt(None)
    }

    /// Constructor met een optionele signal flag (gezet door ctrlc handler).
    pub fn with_signal_flag(flag: Arc<AtomicBool>) -> Self {
        Self::with_signal_flag_opt(Some(flag))
    }

    fn with_signal_flag_opt(signal_flag: Option<Arc<AtomicBool>>) -> Self {
        let mut app = Self {
            config: MonitorConfig::default(),
            monitors: Vec::new(),
            monitor_exists: false,
            log_entries: Vec::new(),
            auto_refresh: true,
            last_refresh: None,
            capture: None,
            capture_output_path: default_capture_path(),
            stop_result_rx: None,
            signal_flag,
            shutdown_done: false,
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

    /// Monitor-lijst ophalen van Hyprland.
    pub fn refresh(&mut self) {
        match hypr::get_monitors() {
            Ok(monitors) => {
                let count = monitors.len();
                self.monitor_exists = monitors.iter().any(|m| m.name == self.config.name);
                self.monitors = monitors;
                self.last_refresh = Some(SystemTime::now());
                self.log(
                    format!("Refreshed — {count} monitor(s) active."),
                    LogLevel::Info,
                );
            }
            Err(e) => {
                self.log(format!("Failed to get monitors: {e}"), LogLevel::Error);
            }
        }
    }

    /// Wordt elke GUI-frame aangeroepen; voert auto-refresh uit indien nodig.
    pub fn tick(&mut self) {
        if !self.auto_refresh {
            return;
        }
        if let Some(last) = self.last_refresh {
            if last.elapsed().unwrap_or_default() >= Duration::from_secs(2) {
                self.refresh();
            }
        }
    }

    /// Checkt of een SIGINT/SIGTERM is ontvangen.
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

    pub fn do_create(&mut self) {
        let name = self.config.name.clone();
        let kw = self.config.to_keyword();

        self.log(
            format!("▶ Creating virtual monitor '{name}'..."),
            LogLevel::Info,
        );
        self.log(format!("  $ hyprctl keyword monitor {kw}"), LogLevel::Info);
        self.log(
            format!("  $ hyprctl output create headless {name}"),
            LogLevel::Info,
        );

        match hypr::create_monitor(&self.config) {
            Ok(out) => {
                self.log(
                    format!("✓ Monitor '{name}' created. {}", out.trim()),
                    LogLevel::Success,
                );
                self.refresh();
            }
            Err(e) => {
                self.log(format!("⚠ {e}"), LogLevel::Warning);
                self.refresh();
            }
        }
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

    pub fn do_start_capture(&mut self) {
        if self.is_capturing() {
            self.log("Capture already running.", LogLevel::Warning);
            return;
        }
        if !self.monitor_exists {
            self.log(
                "Create the virtual monitor first before capturing.",
                LogLevel::Warning,
            );
            return;
        }

        let path = self.capture_output_path.clone();
        self.log(format!("▶ Starting capture → {path}"), LogLevel::Info);
        self.log(
            "  A portal popup will appear — pick the virtual monitor.",
            LogLevel::Info,
        );

        match capture::CaptureSession::start(path.clone()) {
            Ok(session) => {
                self.capture = Some(session);
                self.log(
                    "✓ Capture session started. Select the monitor in the popup.",
                    LogLevel::Success,
                );
            }
            Err(e) => {
                self.log(format!("✗ Failed to start capture: {e}"), LogLevel::Error);
            }
        }
    }

    pub fn do_stop_capture(&mut self) {
        if let Some(mut session) = self.capture.take() {
            self.log(
                "▶ Stopping capture (finalizing MP4 in background)...",
                LogLevel::Info,
            );

            let (tx, rx) = mpsc::channel();
            self.stop_result_rx = Some(rx);

            // Zet het stop+finalize werk OFF de GUI-thread. Eerder blokkeerde
            // session.stop() de GUI-thread via h.join() → feeder.join() →
            // child.wait(), wat de hele applicatie liet hangen.
            std::thread::spawn(move || {
                session.stop();
                let final_status = session.status();
                let _ = tx.send(final_status);
                // session wordt hier gedropt — alle threads zijn al joined.
            });
        }
    }

    /// Poll het achtergrond-stopresultaat. Niet-blokkerend — elke GUI-frame
    /// aangeroepen. Logt het resultaat zodra de stop-thread klaar is.
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
                            format!("✓ Capture saved: {path} ({frames} frames)"),
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
            Err(mpsc::TryRecvError::Empty) => {
                // Nog bezig — wachten.
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.stop_result_rx = None;
                self.log("✗ Capture stop thread ended unexpectedly.", LogLevel::Error);
            }
        }
    }

    /// Cleanup bij afsluiten: capture stoppen + virtuele monitor verwijderen.
    pub fn shutdown(&mut self) {
        if self.shutdown_done {
            return;
        }
        self.shutdown_done = true;

        if let Some(mut session) = self.capture.take() {
            session.stop();
            println!("✓ Capture sessie gestopt bij afsluiten.");
        }
        // Wacht ook eventuele achtergrond stop-thread af.
        if let Some(rx) = self.stop_result_rx.take() {
            // Geef de stop-thread max 5 seconden om te finaliseren.
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

/// Vangnet: als shutdown() niet is aangeroepen (bv. bij panic of onverwacht
/// afsluiten), probeer alsnog netjes op te ruimen.
impl Drop for App {
    fn drop(&mut self) {
        // capture session dropt zichzelf (CaptureSession::drop zet stop_flag
        // en joint threads), maar we willen ook de monitor opruimen.
        if self.capture.is_some() {
            // CaptureSession::drop handelt het stoppen af.
            self.capture.take();
        }
        if self.monitor_exists {
            let name = self.config.name.clone();
            let _ = hypr::remove_monitor(&name);
        }
    }
}

// ─── pad-helpers ──────────────────────────────────────────────

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

