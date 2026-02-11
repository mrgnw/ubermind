use serde::Serialize;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct ServiceInfo {
    pub name: String,
    pub dir: String,
    pub running: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessInfo {
    pub name: String,
    pub pid: Option<u32>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServiceDetail {
    pub name: String,
    pub dir: String,
    pub running: bool,
    pub processes: Vec<ProcessInfo>,
}

fn home_dir() -> PathBuf {
    PathBuf::from(env::var("HOME").expect("HOME not set"))
}

fn expand_tilde(raw: &str) -> String {
    if let Some(rest) = raw.strip_prefix("~/") {
        format!("{}/{rest}", home_dir().display())
    } else {
        raw.to_string()
    }
}

fn config_dir() -> PathBuf {
    if let Ok(xdg) = env::var("XDG_CONFIG_HOME") {
        return Path::new(&xdg).join("ubermind");
    }
    home_dir().join(".config/ubermind")
}

fn config_path() -> PathBuf {
    let primary = config_dir().join("services");
    if primary.exists() {
        return primary;
    }
    for legacy in [
        config_dir().join("services.tsv"),
        home_dir().join(".config/dm/services.tsv"),
        home_dir().join("dev/_daemons/services.tsv"),
    ] {
        if legacy.exists() {
            return legacy;
        }
    }
    primary
}

pub struct Service {
    pub name: String,
    pub dir: PathBuf,
}

impl Service {
    pub fn socket_path(&self) -> PathBuf {
        self.dir.join(".overmind.sock")
    }

    pub fn is_running(&self) -> bool {
        self.socket_path().exists()
    }

    pub fn info(&self) -> ServiceInfo {
        ServiceInfo {
            name: self.name.clone(),
            dir: self.dir.display().to_string(),
            running: self.is_running(),
        }
    }

    pub fn overmind_output(&self, args: &[&str]) -> Result<String, String> {
        Command::new("overmind")
            .args(args)
            .current_dir(&self.dir)
            .output()
            .map_err(|e| format!("failed to run overmind: {e}"))
            .and_then(|out| {
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                if out.status.success() {
                    Ok(stdout)
                } else {
                    Err(format!("{stdout}{stderr}"))
                }
            })
    }

    pub fn overmind_run(&self, args: &[&str]) -> Result<String, String> {
        Command::new("overmind")
            .args(args)
            .current_dir(&self.dir)
            .output()
            .map_err(|e| format!("failed to run overmind: {e}"))
            .map(|out| {
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                format!("{stdout}{stderr}")
            })
    }

    pub fn detail(&self) -> ServiceDetail {
        let running = self.is_running();
        let processes = if running {
            self.parse_overmind_status()
        } else {
            vec![]
        };
        ServiceDetail {
            name: self.name.clone(),
            dir: self.dir.display().to_string(),
            running,
            processes,
        }
    }

    fn parse_overmind_status(&self) -> Vec<ProcessInfo> {
        let output = match self.overmind_output(&["status"]) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        output
            .lines()
            .filter(|line| !line.trim().is_empty())
            .skip_while(|line| {
                let first = line.split_whitespace().next().unwrap_or("");
                first == "PROCESS" || first == "Name"
            })
            .map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                let name = parts.first().unwrap_or(&"unknown").to_string();
                let pid = parts.get(1).and_then(|p| p.parse::<u32>().ok());
                let status = parts.get(2).unwrap_or(&"unknown").to_string();
                ProcessInfo { name, pid, status }
            })
            .collect()
    }
}

pub fn load_services() -> BTreeMap<String, Service> {
    let path = config_path();
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return BTreeMap::new(),
    };

    let mut services = BTreeMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let sep = if line.contains(':') { ':' } else { '\t' };
        let parts: Vec<&str> = line.splitn(2, sep).collect();
        if parts.len() != 2 {
            continue;
        }

        let name = parts[0].trim().to_string();
        let dir = PathBuf::from(expand_tilde(parts[1].trim()));

        services.insert(name.clone(), Service { name, dir });
    }

    services
}

pub fn list_services() -> Vec<ServiceInfo> {
    load_services().values().map(|s| s.info()).collect()
}

pub fn get_service_detail(name: &str) -> Result<ServiceDetail, String> {
    let services = load_services();
    match services.get(name) {
        Some(svc) => Ok(svc.detail()),
        None => Err(format!("unknown service: {name}")),
    }
}

pub fn start_service(name: &str) -> Result<String, String> {
    let services = load_services();
    let svc = services
        .get(name)
        .ok_or(format!("unknown service: {name}"))?;
    if svc.is_running() {
        return Ok(format!("{name}: already running"));
    }
    svc.overmind_run(&["start", "-D"])
        .map(|out| format!("{name}: started\n{out}"))
}

pub fn stop_service(name: &str) -> Result<String, String> {
    let services = load_services();
    let svc = services
        .get(name)
        .ok_or(format!("unknown service: {name}"))?;
    if !svc.is_running() {
        return Ok(format!("{name}: not running"));
    }
    let result = svc.overmind_run(&["quit"]);
    let _ = fs::remove_file(svc.socket_path());
    result.map(|out| format!("{name}: stopped\n{out}"))
}

pub fn reload_service(name: &str) -> Result<String, String> {
    let services = load_services();
    let svc = services
        .get(name)
        .ok_or(format!("unknown service: {name}"))?;
    if svc.is_running() {
        let _ = svc.overmind_run(&["quit"]);
        std::thread::sleep(std::time::Duration::from_secs(1));
        let _ = fs::remove_file(svc.socket_path());
    }
    svc.overmind_run(&["start", "-D"])
        .map(|out| format!("{name}: reloaded\n{out}"))
}
