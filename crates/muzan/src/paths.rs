use std::path::PathBuf;

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

	pub fn state_dir(&self) -> PathBuf {
		if let Ok(dir) = std::env::var("XDG_STATE_HOME") {
			PathBuf::from(dir).join(&self.app_name)
		} else if let Some(home) = home_dir() {
			home.join(".local").join("state").join(&self.app_name)
		} else {
			PathBuf::from("/tmp").join(&self.app_name)
		}
	}

	pub fn config_dir(&self) -> PathBuf {
		if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
			PathBuf::from(dir).join(&self.app_name)
		} else if let Some(home) = home_dir() {
			home.join(".config").join(&self.app_name)
		} else {
			PathBuf::from("/tmp").join(&self.app_name).join("config")
		}
	}

	pub fn socket_path(&self) -> PathBuf {
		self.state_dir().join("daemon.sock")
	}

	pub fn pid_path(&self) -> PathBuf {
		self.state_dir().join("daemon.pid")
	}
}

fn home_dir() -> Option<PathBuf> {
	std::env::var("HOME").ok().map(PathBuf::from)
}
