//! Persisted GUI settings (`~/.config/hyprstream/settings.json`).

use crate::types::MonitorConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    /// Last applied virtual monitor; `None` = derive from the main display.
    #[serde(default)]
    pub monitor: Option<MonitorConfig>,
    /// Physical monitor to position against; `None` = automatic.
    #[serde(default)]
    pub main_monitor: Option<String>,
    /// Resize to the Moonlight client's resolution at session start.
    #[serde(default = "default_true")]
    pub follow_client: bool,
    /// Keep Sunshine's `output_name` equal to the virtual monitor's name.
    #[serde(default = "default_true")]
    pub sync_output_name: bool,
    /// Sunshine web UI login, used to send pairing PINs (stored only when
    /// the user asks for it).
    #[serde(default)]
    pub sunshine_user: String,
    #[serde(default)]
    pub sunshine_password: Option<String>,
}

fn default_true() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        serde_json::from_str("{}").expect("defaults")
    }
}

fn dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("hyprstream")
}

fn path() -> PathBuf {
    dir().join("settings.json")
}

impl Settings {
    pub fn load() -> Self {
        std::fs::read_to_string(path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let _ = std::fs::create_dir_all(dir());
        match serde_json::to_string_pretty(self) {
            Ok(json) => {
                if let Err(e) = write_private(&path(), &json) {
                    log::warn!("settings: cannot save: {e}");
                }
            }
            Err(e) => log::warn!("settings: cannot serialize: {e}"),
        }
    }
}

/// The file may hold the Sunshine password: owner-only permissions.
fn write_private(path: &PathBuf, data: &str) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    f.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    f.write_all(data.as_bytes())
}
