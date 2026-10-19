use crate::types::{MainDisplay, MonitorConfig, MonitorJson};
use std::process::Command;

fn hyprctl(args: &[&str]) -> Result<String, String> {
    let output = Command::new("hyprctl")
        .args(args)
        .output()
        .map_err(|e| format!("Failed to execute hyprctl: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        let msg = if stderr.is_empty() { stdout } else { stderr };
        return Err(format!("hyprctl {}: {}", args.join(" "), msg.trim()));
    }

    // `hyprctl keyword` reports errors on stdout with exit code 0.
    let trimmed = stdout.trim();
    if args.first() == Some(&"keyword") && !trimmed.is_empty() && trimmed != "ok" {
        return Err(format!("hyprctl {}: {}", args.join(" "), trimmed));
    }

    Ok(stdout)
}

/// All monitors, including disabled ones.
pub fn get_monitors() -> Result<Vec<MonitorJson>, String> {
    let json = hyprctl(&["monitors", "all", "-j"])?;
    serde_json::from_str(&json).map_err(|e| format!("Failed to parse monitors JSON: {e}"))
}

/// Picks the main (physical) display: the preferred name if present, else
/// the focused physical monitor, else the first physical monitor. Headless
/// outputs and the virtual monitor itself never qualify.
pub fn pick_main(
    monitors: &[MonitorJson],
    virtual_name: &str,
    preferred: Option<&str>,
) -> Option<MainDisplay> {
    let candidates: Vec<&MonitorJson> = monitors
        .iter()
        .filter(|m| !m.disabled && m.name != virtual_name && !m.is_headless())
        .collect();

    preferred
        .and_then(|p| candidates.iter().find(|m| m.name == p))
        .or_else(|| candidates.iter().find(|m| m.focused))
        .or_else(|| candidates.iter().min_by_key(|m| m.id))
        .map(|m| MainDisplay::from_json(m))
}

pub fn create_headless_output(name: &str) -> Result<String, String> {
    hyprctl(&["output", "create", "headless", name])
}

/// Applies the monitor rule. Lua-configured Hyprland only accepts `eval`
/// (`keyword` answers "can't work with non-legacy parsers"); legacy configs
/// only accept `keyword`. Try eval first, then keyword.
pub fn apply_monitor(config: &MonitorConfig) -> Result<String, String> {
    let lua = config.to_lua();
    match hyprctl(&["eval", &lua]) {
        Ok(out) => Ok(out),
        Err(eval_err) => {
            let kw = config.to_keyword();
            hyprctl(&["keyword", "monitor", &kw])
                .map_err(|kw_err| format!("{eval_err} / {kw_err}"))
        }
    }
}

pub fn remove_monitor(name: &str) -> Result<String, String> {
    let out = hyprctl(&["output", "remove", name])?;

    let disable_kw = format!("{name},disable");
    let _ = hyprctl(&["keyword", "monitor", &disable_kw]);

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mon(id: i64, name: &str, make: &str, focused: bool) -> MonitorJson {
        MonitorJson {
            id,
            name: name.into(),
            description: String::new(),
            make: make.into(),
            model: String::new(),
            serial: String::new(),
            width: 2560,
            height: 1440,
            fps: 144.0,
            x: 0,
            y: 0,
            scale: 1.0,
            transform: 0,
            focused,
            disabled: false,
        }
    }

    #[test]
    fn main_display_skips_headless_and_virtual() {
        let ms = vec![
            mon(0, "virtual", "", true),
            mon(1, "HEADLESS-2", "", false),
            mon(2, "DP-1", "AOC", false),
            mon(3, "HDMI-A-1", "LG", true),
        ];
        assert_eq!(pick_main(&ms, "virtual", None).unwrap().name, "HDMI-A-1");
        assert_eq!(pick_main(&ms, "virtual", Some("DP-1")).unwrap().name, "DP-1");
        assert_eq!(pick_main(&ms, "virtual", Some("gone")).unwrap().name, "HDMI-A-1");
        assert!(pick_main(&ms[..2], "virtual", None).is_none());
    }
}
