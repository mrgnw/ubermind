use crate::types::ServiceStatus;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Request {
	Start { names: Vec<String> },
	Stop { names: Vec<String> },
	Reload { names: Vec<String> },
	Restart { service: String, process: String },
	Kill { service: String, process: String },
	Status,
	Logs { service: String, process: Option<String>, follow: bool },
	Ping,
	Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
	Ok { message: Option<String> },
	Status { services: Vec<ServiceStatus> },
	Log { line: String },
	Error { message: String },
	Progress { service: String, message: String },
	Pong,
}

pub const SOCKET_NAME: &str = "daemon.sock";

pub fn socket_path() -> std::path::PathBuf {
	state_dir().join(SOCKET_NAME)
}

pub fn pid_path() -> std::path::PathBuf {
	state_dir().join("daemon.pid")
}

pub fn state_dir() -> std::path::PathBuf {
	if let Ok(dir) = std::env::var("XDG_STATE_HOME") {
		std::path::PathBuf::from(dir).join("ubermind")
	} else if let Some(home) = home_dir() {
		home.join(".local").join("state").join("ubermind")
	} else {
		std::path::PathBuf::from("/tmp/ubermind")
	}
}

pub fn config_dir() -> std::path::PathBuf {
	if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
		std::path::PathBuf::from(dir).join("ubermind")
	} else if let Some(home) = home_dir() {
		home.join(".config").join("ubermind")
	} else {
		std::path::PathBuf::from("/tmp/ubermind/config")
	}
}

fn home_dir() -> Option<std::path::PathBuf> {
	std::env::var("HOME").ok().map(std::path::PathBuf::from)
}
