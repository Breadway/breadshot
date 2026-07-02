use anyhow::{bail, Context, Result};
use clap::ValueEnum;
use serde_json::Value;
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

use crate::config::Config;

#[derive(Debug, Clone, ValueEnum)]
pub enum Mode {
    /// Select a region interactively
    Region,
    /// Click to select a window
    Window,
    /// Click to select a monitor
    Output,
    /// Capture the active window
    #[value(name = "active-window")]
    ActiveWindow,
    /// Capture the active monitor
    #[value(name = "active-output")]
    ActiveOutput,
}

pub struct Overrides {
    pub clipboard_only: bool,
    pub silent: bool,
    pub freeze: bool,
    pub output_dir: Option<PathBuf>,
    pub filename: Option<String>,
}

pub fn run(mode: &Mode, config: &Config, overrides: Overrides) -> Result<()> {
    check_deps()?;

    let save_dir = overrides.output_dir.as_ref().unwrap_or(&config.save_dir);
    let filename = overrides.filename.unwrap_or_else(|| {
        format!(
            "{}_breadshot.png",
            chrono::Local::now().format(&config.date_format)
        )
    });
    let save_path = save_dir.join(&filename);

    let silent = overrides.silent || config.silent;
    let freeze = overrides.freeze || config.freeze;
    let clipboard_only = overrides.clipboard_only;

    let _freeze_guard = if freeze {
        FreezeGuard::try_spawn()
            .map_err(|e| tracing::warn!("freeze: {e}"))
            .ok()
    } else {
        None
    };

    let geometry = geometry_for_mode(mode)?;
    tracing::debug!("geometry: {geometry}");

    if clipboard_only {
        copy_only(&geometry)?;
    } else {
        std::fs::create_dir_all(save_dir)
            .with_context(|| format!("creating {}", save_dir.display()))?;
        save_and_copy(&geometry, &save_path)?;
    }

    if !silent {
        let msg = if clipboard_only {
            "Copied to clipboard".to_string()
        } else {
            format!("Saved to <i>{}</i>", save_path.display())
        };
        send_notification("Screenshot", &msg, config.notif_timeout, &save_path);
    }

    Ok(())
}

// --- geometry ---

fn geometry_for_mode(mode: &Mode) -> Result<String> {
    match mode {
        Mode::Region => geometry_region(),
        Mode::Window => geometry_window().and_then(|g| trim_geometry(&g)),
        Mode::Output => geometry_output(),
        Mode::ActiveWindow => geometry_active_window().and_then(|g| trim_geometry(&g)),
        Mode::ActiveOutput => geometry_active_output(),
    }
}

fn geometry_region() -> Result<String> {
    slurp(&["-d"])
}

fn geometry_output() -> Result<String> {
    slurp(&["-or"])
}

fn geometry_window() -> Result<String> {
    let monitors = hyprctl_json("monitors")?;
    let active_ids: Vec<i64> = monitors
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|m| m["activeWorkspace"]["id"].as_i64())
        .collect();

    let clients = hyprctl_json("clients")?;
    let boxes: Vec<String> = clients
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter(|c| {
            c["workspace"]["id"]
                .as_i64()
                .is_some_and(|id| active_ids.contains(&id))
        })
        .map(|c| {
            format!(
                "{},{} {}x{}",
                c["at"][0].as_i64().unwrap_or(0),
                c["at"][1].as_i64().unwrap_or(0),
                c["size"][0].as_i64().unwrap_or(0),
                c["size"][1].as_i64().unwrap_or(0),
            )
        })
        .collect();

    slurp_piped(&["-r"], &boxes.join("\n"))
}

fn geometry_active_window() -> Result<String> {
    let w = hyprctl_json("activewindow")?;
    Ok(format!(
        "{},{} {}x{}",
        w["at"][0].as_i64().context("missing at[0]")?,
        w["at"][1].as_i64().context("missing at[1]")?,
        w["size"][0].as_i64().context("missing size[0]")?,
        w["size"][1].as_i64().context("missing size[1]")?,
    ))
}

fn geometry_active_output() -> Result<String> {
    let active_id = hyprctl_json("activeworkspace")?["id"]
        .as_i64()
        .context("no active workspace id")?;

    let monitors = hyprctl_json("monitors")?;
    let m = monitors
        .as_array()
        .and_then(|arr| {
            arr.iter()
                .find(|m| m["activeWorkspace"]["id"].as_i64() == Some(active_id))
        })
        .context("no monitor for active workspace")?;

    monitor_geometry(m)
}

fn monitor_geometry(m: &Value) -> Result<String> {
    let x = m["x"].as_i64().unwrap_or(0);
    let y = m["y"].as_i64().unwrap_or(0);
    let w = m["width"].as_f64().context("missing width")?;
    let h = m["height"].as_f64().context("missing height")?;
    let scale = m["scale"].as_f64().unwrap_or(1.0);
    Ok(format!(
        "{x},{y} {}x{}",
        (w / scale).round() as i64,
        (h / scale).round() as i64
    ))
}

// Clips window geometry to the logical bounds of the monitor layout.
// hyprctl clients returns logical (scaled) coordinates, but a window
// can technically extend outside the visible area.
fn trim_geometry(geometry: &str) -> Result<String> {
    let (xy, wh) = geometry.split_once(' ').context("invalid geometry")?;
    let (xs, ys) = xy.split_once(',').context("invalid xy")?;
    let (ws, hs) = wh.split_once('x').context("invalid wh")?;
    let x: i64 = xs.trim().parse().context("invalid x")?;
    let y: i64 = ys.trim().parse().context("invalid y")?;
    let w: i64 = ws.trim().parse().context("invalid w")?;
    let h: i64 = hs.trim().parse().context("invalid h")?;

    let monitors = hyprctl_json("monitors")?;
    let monitors = monitors.as_array().context("monitors not an array")?;

    let (max_x, max_y, min_x, min_y) =
        monitors
            .iter()
            .fold((i64::MIN, i64::MIN, i64::MAX, i64::MAX), |acc, m| {
                let mx = m["x"].as_i64().unwrap_or(0);
                let my = m["y"].as_i64().unwrap_or(0);
                let mw = m["width"].as_f64().unwrap_or(0.0);
                let mh = m["height"].as_f64().unwrap_or(0.0);
                let scale = m["scale"].as_f64().unwrap_or(1.0);
                let transform = m["transform"].as_i64().unwrap_or(0);
                let (lw, lh) = if transform % 2 == 0 {
                    ((mw / scale).round() as i64, (mh / scale).round() as i64)
                } else {
                    ((mh / scale).round() as i64, (mw / scale).round() as i64)
                };
                (
                    acc.0.max(mx + lw),
                    acc.1.max(my + lh),
                    acc.2.min(mx),
                    acc.3.min(my),
                )
            });

    let mut cx = x;
    let mut cy = y;
    let mut cw = w;
    let mut ch = h;
    if x + w > max_x {
        cw = max_x - x;
    }
    if y + h > max_y {
        ch = max_y - y;
    }
    if x < min_x {
        cx = min_x;
        cw += x - min_x;
    }
    if y < min_y {
        cy = min_y;
        ch += y - min_y;
    }

    Ok(format!("{cx},{cy} {cw}x{ch}"))
}

// --- capture ---

fn copy_only(geometry: &str) -> Result<()> {
    let mut grim = Command::new("grim")
        .args(["-g", geometry, "-"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .context("spawning grim")?;

    let stdout = grim.stdout.take().context("grim stdout")?;

    let mut wl_copy = Command::new("wl-copy")
        .args(["--type", "image/png"])
        .stdin(stdout)
        .spawn()
        .context("spawning wl-copy")?;

    grim.wait().context("waiting for grim")?;
    wl_copy.wait().context("waiting for wl-copy")?;

    Ok(())
}

fn save_and_copy(geometry: &str, path: &Path) -> Result<()> {
    let status = Command::new("grim")
        .args(["-g", geometry, path.to_str().unwrap_or("")])
        .status()
        .context("running grim")?;

    if !status.success() {
        bail!("grim exited with {status}");
    }

    let data = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;

    let mut wl_copy = Command::new("wl-copy")
        .args(["--type", "image/png"])
        .stdin(Stdio::piped())
        .spawn()
        .context("spawning wl-copy")?;

    wl_copy
        .stdin
        .take()
        .context("wl-copy stdin")?
        .write_all(&data)
        .context("piping to wl-copy")?;

    wl_copy.wait().context("waiting for wl-copy")?;

    Ok(())
}

fn send_notification(title: &str, msg: &str, timeout: u32, path: &Path) {
    let mut cmd = Command::new("notify-send");
    cmd.args([title, msg, "-t", &timeout.to_string(), "-a", "breadshot"]);
    if path.exists() {
        cmd.args(["-i", path.to_str().unwrap_or("")]);
    }
    if let Err(e) = cmd.status() {
        tracing::warn!("notify-send: {e}");
    }
}

// --- helpers ---

fn hyprctl_json(subcmd: &str) -> Result<Value> {
    let out = Command::new("hyprctl")
        .args(["-j", subcmd])
        .output()
        .context("running hyprctl")?;
    serde_json::from_slice(&out.stdout)
        .with_context(|| format!("parsing hyprctl {subcmd} output"))
}

fn slurp(args: &[&str]) -> Result<String> {
    let out = Command::new("slurp")
        .args(args)
        .output()
        .context("running slurp")?;
    if !out.status.success() {
        bail!("selection cancelled");
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn slurp_piped(args: &[&str], stdin_data: &str) -> Result<String> {
    let mut child = Command::new("slurp")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("spawning slurp")?;

    child
        .stdin
        .take()
        .context("slurp stdin")?
        .write_all(stdin_data.as_bytes())
        .context("writing to slurp")?;

    let out = child.wait_with_output().context("waiting for slurp")?;
    if !out.status.success() {
        bail!("selection cancelled");
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn in_path(cmd: &str) -> bool {
    std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).any(|dir| dir.join(cmd).exists()))
        .unwrap_or(false)
}

fn check_deps() -> Result<()> {
    let required = ["grim", "slurp", "wl-copy", "hyprctl"];
    let missing: Vec<_> = required.iter().copied().filter(|&d| !in_path(d)).collect();
    if !missing.is_empty() {
        bail!("missing required tools: {}", missing.join(", "));
    }
    Ok(())
}

// --- freeze ---

struct FreezeGuard {
    child: std::process::Child,
}

impl FreezeGuard {
    fn try_spawn() -> Result<Self> {
        if !in_path("hyprpicker") {
            bail!("hyprpicker not installed");
        }
        let child = Command::new("hyprpicker")
            .args(["-r", "-z"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("spawning hyprpicker")?;
        std::thread::sleep(Duration::from_millis(200));
        Ok(Self { child })
    }
}

impl Drop for FreezeGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
