//! Hyprland IPC integration for fullscreen detection.
//!
//! Connects to Hyprland's event socket to detect when a fullscreen
//! application is active on the target monitor, hiding the ticker overlay.

use std::env;
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

/// Visibility state for the ticker window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TickerVisibility {
    Visible,
    Hidden,
}

/// Watch for fullscreen events on the specified monitor.
///
/// Spawns a background thread that connects to Hyprland's event socket
/// and sends visibility updates through the provided channel.
pub fn watch_fullscreen(target_monitor: String, sender: mpsc::Sender<TickerVisibility>) {
    std::thread::spawn(move || {
        loop {
            let monitor_id = match get_monitor_id(&target_monitor) {
                Some(id) => id,
                None => {
                    std::thread::sleep(Duration::from_secs(2));
                    continue;
                }
            };

            // Check initial state
            let _ = sender.send(visibility_for_monitor(monitor_id));

            if let Err(e) = event_loop(monitor_id, &sender) {
                eprintln!("Hyprland IPC: {:?}", e);
            }

            std::thread::sleep(Duration::from_secs(2));
            let _ = sender.send(visibility_for_monitor(monitor_id));
        }
    });
}

fn visibility_for_monitor(monitor_id: i64) -> TickerVisibility {
    if is_fullscreen_on_monitor(monitor_id) {
        TickerVisibility::Hidden
    } else {
        TickerVisibility::Visible
    }
}

fn get_event_socket_path() -> Option<String> {
    let sig = env::var("HYPRLAND_INSTANCE_SIGNATURE").ok()?;
    let runtime = env::var("XDG_RUNTIME_DIR").ok()?;
    Some(format!("{}/hypr/{}/.socket2.sock", runtime, sig))
}

fn get_monitor_id(name: &str) -> Option<i64> {
    let output = Command::new("hyprctl")
        .args(["monitors", "-j"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let json: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).ok()?;

    for monitor in json {
        if monitor.get("name").and_then(|n| n.as_str()) == Some(name) {
            return monitor.get("id").and_then(|id| id.as_i64());
        }
    }

    None
}

fn get_active_monitor_id() -> Option<i64> {
    let output = Command::new("hyprctl")
        .args(["activewindow", "-j"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    json.get("monitor").and_then(|m| m.as_i64())
}

fn is_fullscreen_on_monitor(target_id: i64) -> bool {
    let output = match Command::new("hyprctl")
        .args(["activewindow", "-j"])
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return false,
    };

    let json: serde_json::Value = match serde_json::from_slice(&output.stdout) {
        Ok(j) => j,
        Err(_) => return false,
    };

    let is_fullscreen = json
        .get("fullscreen")
        .and_then(|f| f.as_i64())
        .map(|f| f > 0)
        .unwrap_or(false);

    let monitor_id = json
        .get("monitor")
        .and_then(|m| m.as_i64())
        .unwrap_or(-1);

    is_fullscreen && monitor_id == target_id
}

fn event_loop(
    target_id: i64,
    sender: &mpsc::Sender<TickerVisibility>,
) -> Result<(), Box<dyn std::error::Error>> {
    let socket_path = get_event_socket_path()
        .ok_or("Could not find Hyprland socket")?;

    let stream = UnixStream::connect(&socket_path)?;
    let reader = BufReader::new(stream);

    for line in reader.lines() {
        let line = line?;

        if line.starts_with("fullscreen>>") {
            let is_fullscreen = line.ends_with("1");
            if is_fullscreen {
                if get_active_monitor_id() == Some(target_id) {
                    let _ = sender.send(TickerVisibility::Hidden);
                }
            } else {
                let _ = sender.send(TickerVisibility::Visible);
            }
        } else if line.starts_with("activewindow>>") || line.starts_with("focusedmon>>") {
            let _ = sender.send(visibility_for_monitor(target_id));
        }
    }

    Ok(())
}
