//! Controls the local Sunshine install: the systemd user service, the
//! `output_name` in `sunshine.conf`, the "follow client resolution" prep-cmd
//! in `apps.json`, and PIN pairing through Sunshine's web API.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Unit installed by the Sunshine package (`sunshine.service` is an alias
/// that only exists while the unit is enabled).
pub const UNIT: &str = "app-dev.lizardbyte.app.Sunshine.service";
pub const WEB_UI: &str = "https://localhost:47990";

/// Argument Sunshine's prep-cmd passes to us; reads `SUNSHINE_CLIENT_*`.
pub const CLIENT_MODE_ARG: &str = "--client-mode";

#[derive(Clone, Debug, PartialEq)]
pub enum ServiceState {
    Active,
    Activating,
    Inactive,
    Failed,
    Unknown(String),
}

impl ServiceState {
    pub fn label(&self) -> &str {
        match self {
            ServiceState::Active => "running",
            ServiceState::Activating => "starting",
            ServiceState::Inactive => "stopped",
            ServiceState::Failed => "failed",
            ServiceState::Unknown(_) => "unknown",
        }
    }
}

/// Everything the GUI shows about Sunshine; gathered off the UI thread.
#[derive(Clone, Debug)]
pub struct SunshineInfo {
    pub state: ServiceState,
    /// `output_name` from sunshine.conf (`None` = not set, Sunshine picks
    /// the first monitor).
    pub output_name: Option<String>,
    /// The "Desktop" app runs `streamer --client-mode` as prep-cmd.
    pub prep_installed: bool,
}

pub fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("sunshine")
}

fn conf_path() -> PathBuf {
    config_dir().join("sunshine.conf")
}

fn apps_path() -> PathBuf {
    config_dir().join("apps.json")
}

pub fn query() -> SunshineInfo {
    SunshineInfo {
        state: state(),
        output_name: read_output_name(),
        prep_installed: prep_cmd_installed(),
    }
}

fn systemctl(args: &[&str]) -> Result<String, String> {
    let out = Command::new("systemctl")
        .arg("--user")
        .args(args)
        .output()
        .map_err(|e| format!("systemctl: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if out.status.success() {
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(if stderr.is_empty() { stdout } else { stderr })
    }
}

pub fn state() -> ServiceState {
    // `is-active` exits non-zero for anything but "active"; the state is
    // still printed on stdout.
    let s = match systemctl(&["is-active", UNIT]) {
        Ok(s) | Err(s) => s,
    };
    match s.as_str() {
        "active" | "reloading" => ServiceState::Active,
        "activating" => ServiceState::Activating,
        "inactive" | "deactivating" => ServiceState::Inactive,
        "failed" => ServiceState::Failed,
        other => {
            log::warn!("sunshine: systemctl is-active: {other}");
            ServiceState::Unknown(other.to_string())
        }
    }
}

// A manual start must work even after systemd's start-limit tripped
// ("Start request repeated too quickly"), which otherwise blocks it.
pub fn start() -> Result<(), String> {
    let _ = systemctl(&["reset-failed", UNIT]);
    systemctl(&["start", "--no-block", UNIT]).map(|_| ())
}

pub fn stop() -> Result<(), String> {
    systemctl(&["stop", "--no-block", UNIT]).map(|_| ())
}

pub fn restart() -> Result<(), String> {
    let _ = systemctl(&["reset-failed", UNIT]);
    systemctl(&["restart", "--no-block", UNIT]).map(|_| ())
}

pub fn open_web_ui() -> Result<(), String> {
    Command::new("xdg-open")
        .arg(WEB_UI)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("xdg-open: {e}"))
}

// sunshine.conf

fn conf_key(line: &str) -> Option<(&str, &str)> {
    let line = line.trim();
    if line.starts_with('#') {
        return None;
    }
    let (k, v) = line.split_once('=')?;
    Some((k.trim(), v.trim()))
}

pub fn read_output_name() -> Option<String> {
    let text = std::fs::read_to_string(conf_path()).ok()?;
    text.lines()
        .filter_map(conf_key)
        .filter(|(k, _)| *k == "output_name")
        .last()
        .map(|(_, v)| v.to_string())
        .filter(|v| !v.is_empty())
}

/// Sets `output_name` in sunshine.conf, keeping every other line. Returns
/// whether the file changed (Sunshine only reads it at startup).
pub fn set_output_name(name: &str) -> Result<bool, String> {
    let path = conf_path();
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let new = replace_conf_value(&text, "output_name", name);
    if new == text {
        return Ok(false);
    }
    std::fs::create_dir_all(config_dir()).map_err(|e| e.to_string())?;
    std::fs::write(&path, new).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(true)
}

fn replace_conf_value(text: &str, key: &str, value: &str) -> String {
    let mut found = false;
    let mut lines: Vec<String> = text
        .lines()
        .map(|l| match conf_key(l) {
            Some((k, _)) if k == key => {
                found = true;
                format!("{key} = {value}")
            }
            _ => l.to_string(),
        })
        .collect();
    if !found {
        lines.push(format!("{key} = {value}"));
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

// apps.json prep-cmd

fn prep_command() -> String {
    let exe = std::env::current_exe()
        .ok()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "streamer".into());
    format!("{exe} {CLIENT_MODE_ARG}")
}

fn is_our_prep(entry: &serde_json::Value) -> bool {
    entry
        .get("do")
        .and_then(|d| d.as_str())
        .is_some_and(|d| d.trim_end().ends_with(CLIENT_MODE_ARG))
}

fn desktop_app(apps: &mut serde_json::Value) -> Option<&mut serde_json::Value> {
    apps.get_mut("apps")?
        .as_array_mut()?
        .iter_mut()
        .find(|a| a.get("name").and_then(|n| n.as_str()) == Some("Desktop"))
}

pub fn prep_cmd_installed() -> bool {
    let Ok(text) = std::fs::read_to_string(apps_path()) else {
        return false;
    };
    let Ok(mut apps) = serde_json::from_str::<serde_json::Value>(&text) else {
        return false;
    };
    desktop_app(&mut apps)
        .and_then(|a| a.get("prep-cmd"))
        .and_then(|p| p.as_array())
        .is_some_and(|p| p.iter().any(is_our_prep))
}

/// Adds (or removes) `streamer --client-mode` as prep-cmd of the "Desktop"
/// app, so the virtual monitor takes the resolution Moonlight asks for
/// before Sunshine starts capturing. Keeps a backup as apps.json.bak.
pub fn set_prep_cmd(install: bool) -> Result<(), String> {
    let path = apps_path();
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut apps: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("apps.json: {e}"))?;
    let app = desktop_app(&mut apps).ok_or("apps.json has no \"Desktop\" app")?;
    let obj = app.as_object_mut().ok_or("\"Desktop\" app is not an object")?;
    let prep = obj
        .entry("prep-cmd")
        .or_insert_with(|| serde_json::Value::Array(Vec::new()))
        .as_array_mut()
        .ok_or("prep-cmd is not a list")?;
    prep.retain(|e| !is_our_prep(e));
    if install {
        prep.insert(0, serde_json::json!({ "do": prep_command(), "undo": "" }));
    }
    let _ = std::fs::write(path.with_extension("json.bak"), &text);
    let json = serde_json::to_string_pretty(&apps).map_err(|e| e.to_string())?;
    std::fs::write(&path, json + "\n").map_err(|e| format!("{}: {e}", path.display()))
}

// pairing

/// Sends the PIN Moonlight shows to Sunshine's web API. Sunshine first has to
/// hand out the id of the pending pairing request the PIN belongs to. The
/// credentials are the web UI login; they go to curl over stdin, not on the
/// command line.
pub fn send_pin(pin: &str, client_name: &str, user: &str, password: &str) -> Result<(), String> {
    let pairing_id = pending_pairing_id(user, password)?;
    let body = serde_json::json!({ "pairing_id": pairing_id, "pin": pin, "name": client_name });
    let reply = pin_api("POST", Some(&body.to_string()), user, password)?;
    if reply.contains("true") {
        Ok(())
    } else {
        Err("Sunshine rejected the PIN (wrong PIN or pairing request expired)".into())
    }
}

/// Returns the id of the one pairing request Sunshine has pending.
fn pending_pairing_id(user: &str, password: &str) -> Result<String, String> {
    let reply = pin_api("GET", None, user, password)?;
    let json: serde_json::Value =
        serde_json::from_str(&reply).map_err(|e| format!("Sunshine pairing list: {e}"))?;
    let ids: Vec<&str> = json
        .get("pairings")
        .and_then(|p| p.as_array())
        .map(|p| p.iter().filter_map(|e| e.get("id")?.as_str()).collect())
        .unwrap_or_default();
    match ids.as_slice() {
        [] => Err("no pairing request pending (start pairing in Moonlight first)".into()),
        [id] => Ok((*id).to_string()),
        _ => Err(format!(
            "{} pairing requests pending; cancel the others in Sunshine's web UI",
            ids.len()
        )),
    }
}

/// Calls `/api/pin` with the given method and returns the reply body on 200.
fn pin_api(method: &str, body: Option<&str>, user: &str, password: &str) -> Result<String, String> {
    let config = format!(
        "url = \"{WEB_UI}/api/pin\"\nuser = \"{}:{}\"\n",
        curl_escape(user),
        curl_escape(password)
    );
    let mut cmd = Command::new("curl");
    cmd.args(["-sk", "--max-time", "5", "-X", method, "-w", "\n%{http_code}"]);
    if let Some(body) = body {
        cmd.args(["-H", "Content-Type: application/json", "--data-binary", body]);
    }
    let mut child = cmd
        .args(["--config", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("curl: {e}"))?;
    child
        .stdin
        .take()
        .ok_or("curl: no stdin")?
        .write_all(config.as_bytes())
        .map_err(|e| format!("curl: {e}"))?;
    let out = child.wait_with_output().map_err(|e| format!("curl: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let (reply, code) = stdout.rsplit_once('\n').unwrap_or(("", stdout.as_ref()));
    match code.trim() {
        "000" => Err("Sunshine web API unreachable (is Sunshine running?)".into()),
        "401" => Err("wrong Sunshine username/password".into()),
        "200" => Ok(reply.to_string()),
        other => Err(format!("Sunshine answered HTTP {other}: {}", reply.trim())),
    }
}

fn curl_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conf_value_is_replaced_or_appended() {
        let text = "# output_name = old\noutput_name = vm\ncapture = wlr\n";
        assert_eq!(
            replace_conf_value(text, "output_name", "virtual"),
            "# output_name = old\noutput_name = virtual\ncapture = wlr\n"
        );
        assert_eq!(
            replace_conf_value("capture = wlr\n", "output_name", "virtual"),
            "capture = wlr\noutput_name = virtual\n"
        );
    }
}
