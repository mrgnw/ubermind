use serde::Serialize;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct TmuxPane {
    pub session: String,
    pub window: u32,
    pub pane: u32,
    pub command: String,
    pub pid: u32,
}

fn tmux_socket_dir() -> PathBuf {
    let uid = unsafe { libc::getuid() };
    PathBuf::from(format!("/private/tmp/tmux-{uid}"))
}

pub fn find_overmind_socket(service_name: &str) -> Option<PathBuf> {
    let dir = tmux_socket_dir();
    let entries = fs::read_dir(&dir).ok()?;

    let prefix = format!("overmind-{service_name}-");

    let mut best: Option<(PathBuf, std::time::SystemTime)> = None;

    for entry in entries.flatten() {
        let fname = entry.file_name().to_string_lossy().to_string();
        if !fname.starts_with(&prefix) {
            continue;
        }
        let meta = entry.metadata().ok()?;
        let mode = meta.permissions().mode();
        // Active sockets have execute permission
        if mode & 0o100 == 0 {
            continue;
        }
        let mtime = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
        if best.as_ref().map_or(true, |(_, t)| mtime > *t) {
            best = Some((entry.path(), mtime));
        }
    }

    best.map(|(p, _)| p)
}

pub fn list_panes(service_name: &str) -> Vec<TmuxPane> {
    let socket = match find_overmind_socket(service_name) {
        Some(s) => s,
        None => return vec![],
    };

    let output = Command::new("tmux")
        .args([
            "-S",
            &socket.display().to_string(),
            "list-panes",
            "-a",
            "-F",
            "#{session_name} #{window_index} #{pane_index} #{pane_current_command} #{pane_pid}",
        ])
        .output();

    match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 5 {
                    Some(TmuxPane {
                        session: parts[0].to_string(),
                        window: parts[1].parse().unwrap_or(0),
                        pane: parts[2].parse().unwrap_or(0),
                        command: parts[3].to_string(),
                        pid: parts[4].parse().unwrap_or(0),
                    })
                } else {
                    None
                }
            })
            .collect(),
        _ => vec![],
    }
}

pub fn capture_pane(service_name: &str, window: u32, pane: u32) -> Result<String, String> {
    let socket = find_overmind_socket(service_name)
        .ok_or_else(|| format!("no tmux socket found for {service_name}"))?;

    let target = format!("{service_name}:{window}.{pane}");

    let output = Command::new("tmux")
        .args([
            "-S",
            &socket.display().to_string(),
            "capture-pane",
            "-e",
            "-p",
            "-t",
            &target,
            "-S",
            "-",
        ])
        .output()
        .map_err(|e| format!("tmux error: {e}"))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("tmux capture-pane failed: {stderr}"))
    }
}

pub fn capture_all_panes(service_name: &str) -> Result<String, String> {
    let panes = list_panes(service_name);
    if panes.is_empty() {
        return Err(format!("no panes found for {service_name}"));
    }

    let mut output = String::new();
    for pane in &panes {
        match capture_pane(service_name, pane.window, pane.pane) {
            Ok(content) => {
                if panes.len() > 1 {
                    output.push_str(&format!(
                        "\x1b[1m--- {}:{}.{} ({}) ---\x1b[0m\n",
                        service_name, pane.window, pane.pane, pane.command
                    ));
                }
                output.push_str(&content);
            }
            Err(e) => {
                output.push_str(&format!("Error capturing pane: {e}\n"));
            }
        }
    }
    Ok(output)
}
