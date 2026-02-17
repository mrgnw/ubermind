use std::future::Future;
use std::path::PathBuf;

use serde::{Serialize, de::DeserializeOwned};

use crate::paths::DaemonPaths;
use crate::server;

pub struct Daemon {
	pub paths: DaemonPaths,
}

impl Daemon {
	pub fn new(app_name: impl Into<String>) -> Self {
		Self {
			paths: DaemonPaths::new(app_name),
		}
	}

	pub async fn run<Req, Resp, F, Fut>(&self, handler: F)
	where
		Req: DeserializeOwned + Send + 'static,
		Resp: Serialize + Send + 'static,
		F: Fn(Req) -> Fut + Send + Sync + 'static,
		Fut: Future<Output = Resp> + Send,
	{
		let state_dir = self.paths.state_dir();
		let _ = std::fs::create_dir_all(&state_dir);

		let pid_path = self.paths.pid_path();
		let _ = std::fs::write(&pid_path, std::process::id().to_string());

		let socket_path = self.paths.socket_path();
		if socket_path.exists() {
			let _ = std::fs::remove_file(&socket_path);
		}

		tracing::info!("daemon started (pid {})", std::process::id());

		let paths = self.paths.clone();
		let server_handle = tokio::spawn(async move {
			server::run_socket_server(&paths, handler).await;
		});

		tokio::select! {
			_ = server_handle => {},
			_ = tokio::signal::ctrl_c() => {
				tracing::info!("shutting down");
			}
		}

		self.cleanup();
	}

	pub fn cleanup(&self) {
		let _ = std::fs::remove_file(self.paths.socket_path());
		let _ = std::fs::remove_file(self.paths.pid_path());
	}

	pub fn start_background(&self) -> Result<(), String> {
		if crate::client::is_running(&self.paths) {
			return Err("daemon already running".to_string());
		}

		let binary = find_current_binary();
		let mut cmd = std::process::Command::new(&binary);
		cmd.args(["daemon", "run"])
			.stdout(std::process::Stdio::null())
			.stderr(std::process::Stdio::null());

		cmd.spawn().map_err(|e| format!("failed to start daemon: {}", e))?;
		Ok(())
	}

	pub fn start_background_with_args(&self, args: &[&str]) -> Result<(), String> {
		if crate::client::is_running(&self.paths) {
			return Err("daemon already running".to_string());
		}

		let binary = find_current_binary();
		let mut cmd = std::process::Command::new(&binary);
		cmd.args(args)
			.stdout(std::process::Stdio::null())
			.stderr(std::process::Stdio::null());

		cmd.spawn().map_err(|e| format!("failed to start daemon: {}", e))?;
		Ok(())
	}

	pub fn stop(&self) -> Result<(), String> {
		if let Some(pid) = crate::client::read_pid(&self.paths) {
			use nix::sys::signal::{kill, Signal};
			use nix::unistd::Pid;
			let _ = kill(Pid::from_raw(pid as i32), Signal::SIGTERM);
			self.cleanup();
			Ok(())
		} else {
			Err("daemon not running".to_string())
		}
	}
}

pub fn ensure_daemon<Req, Resp>(
	paths: &DaemonPaths,
) -> Result<crate::client::DaemonClient<Req, Resp>, crate::client::ClientError>
where
	Req: Serialize,
	Resp: serde::de::DeserializeOwned,
{
	if let Ok(client) = crate::client::DaemonClient::connect(paths) {
		return Ok(client);
	}

	let binary = find_current_binary();
	let mut cmd = std::process::Command::new(&binary);
	cmd.args(["daemon", "run"])
		.stdout(std::process::Stdio::null())
		.stderr(std::process::Stdio::null());

	cmd.spawn()
		.map_err(|e| crate::client::ClientError::Io(e))?;

	for _ in 0..50 {
		std::thread::sleep(std::time::Duration::from_millis(100));
		if let Ok(client) = crate::client::DaemonClient::connect(paths) {
			return Ok(client);
		}
	}

	Err(crate::client::ClientError::NotRunning)
}

pub fn ensure_daemon_with_args<Req, Resp>(
	paths: &DaemonPaths,
	args: &[&str],
) -> Result<crate::client::DaemonClient<Req, Resp>, crate::client::ClientError>
where
	Req: Serialize,
	Resp: serde::de::DeserializeOwned,
{
	if let Ok(client) = crate::client::DaemonClient::connect(paths) {
		return Ok(client);
	}

	let binary = find_current_binary();
	let mut cmd = std::process::Command::new(&binary);
	cmd.args(args)
		.stdout(std::process::Stdio::null())
		.stderr(std::process::Stdio::null());

	cmd.spawn()
		.map_err(|e| crate::client::ClientError::Io(e))?;

	for _ in 0..50 {
		std::thread::sleep(std::time::Duration::from_millis(100));
		if let Ok(client) = crate::client::DaemonClient::connect(paths) {
			return Ok(client);
		}
	}

	Err(crate::client::ClientError::NotRunning)
}

fn find_current_binary() -> PathBuf {
	std::env::current_exe().unwrap_or_else(|_| PathBuf::from("daemon"))
}
