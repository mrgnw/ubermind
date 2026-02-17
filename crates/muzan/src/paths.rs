use std::path::PathBuf;

/// XDG-compliant paths for daemon state and config files.
///
/// All paths are scoped by `app_name`:
/// - State: `~/.local/state/{app_name}/`
/// - Config: `~/.config/{app_name}/`
/// - Socket: `~/.local/state/{app_name}/daemon.sock`
/// - PID: `~/.local/state/{app_name}/daemon.pid`
#[derive(Debug, Clone)]
pub struct DaemonPaths {
	pub app_name: String,
}

impl DaemonPaths {
	pub fn new(app_name: impl Into<String>) -> Self {
		Self {
			app_name: app_name.into(),
		}
	}

	/// `$XDG_STATE_HOME/{app_name}` or `~/.local/state/{app_name}`
	pub fn state_dir(&self) -> PathBuf {
		if let Ok(dir) = std::env::var("XDG_STATE_HOME") {
			PathBuf::from(dir).join(&self.app_name)
		} else if let Some(home) = home_dir() {
			home.join(".local").join("state").join(&self.app_name)
		} else {
			PathBuf::from("/tmp").join(&self.app_name)
		}
	}

	/// `$XDG_CONFIG_HOME/{app_name}` or `~/.config/{app_name}`
	pub fn config_dir(&self) -> PathBuf {
		if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
			PathBuf::from(dir).join(&self.app_name)
		} else if let Some(home) = home_dir() {
			home.join(".config").join(&self.app_name)
		} else {
			PathBuf::from("/tmp").join(&self.app_name).join("config")
		}
	}

	/// Unix socket path: `{state_dir}/daemon.sock`
	pub fn socket_path(&self) -> PathBuf {
		self.state_dir().join("daemon.sock")
	}

	/// PID file path: `{state_dir}/daemon.pid`
	pub fn pid_path(&self) -> PathBuf {
		self.state_dir().join("daemon.pid")
	}
}

fn home_dir() -> Option<PathBuf> {
	std::env::var("HOME").ok().map(PathBuf::from)
}
