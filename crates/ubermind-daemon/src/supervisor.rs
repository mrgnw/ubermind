use crate::output::OutputCapture;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::AsyncReadExt;
use tokio::process::{Child, Command};
use tokio::sync::RwLock;
use ubermind_core::config::{self, GlobalConfig};
use ubermind_core::types::*;

pub struct Supervisor {
	pub services: Arc<RwLock<HashMap<String, ManagedService>>>,
	pub config: GlobalConfig,
}

pub struct ManagedService {
	pub name: String,
	pub dir: std::path::PathBuf,
	pub processes: HashMap<String, ManagedProcess>,
}

pub struct ManagedProcess {
	pub def: ProcessDef,
	pub state: ProcessState,
	pub output: OutputCapture,
	pub started_at: Option<Instant>,
	pub retry_count: u32,
	cancel: Option<tokio::sync::watch::Sender<bool>>,
}

impl Supervisor {
	pub fn new(config: GlobalConfig) -> Arc<Self> {
		Arc::new(Self {
			services: Arc::new(RwLock::new(HashMap::new())),
			config,
		})
	}

	pub async fn status(self: &Arc<Self>) -> Vec<ServiceStatus> {
		let entries = config::load_service_entries();
		let services = self.services.read().await;
		let mut result = Vec::new();

		for (name, entry) in &entries {
			if let Some(managed) = services.get(name) {
				let processes = managed
					.processes
					.iter()
					.map(|(pname, mp)| ProcessStatus {
						name: pname.clone(),
						state: mp.state.clone(),
						pid: match &mp.state {
							ProcessState::Running { pid, .. } => Some(*pid),
							_ => None,
						},
					})
					.collect();
				result.push(ServiceStatus {
					name: name.clone(),
					dir: entry.dir.clone(),
					processes,
				});
			} else {
				result.push(ServiceStatus {
					name: name.clone(),
					dir: entry.dir.clone(),
					processes: Vec::new(),
				});
			}
		}
		result
	}

	pub async fn start_service(self: &Arc<Self>, name: &str) -> Result<String, String> {
		let entries = config::load_service_entries();
		let entry = entries.get(name).ok_or_else(|| format!("unknown service: {}", name))?;

		{
			let services = self.services.read().await;
			if let Some(managed) = services.get(name) {
				if managed.processes.values().any(|p| p.state.is_running()) {
					return Ok(format!("{}: already running", name));
				}
			}
		}

		let service = config::load_service(entry, &self.config.defaults);
		if service.processes.is_empty() {
			return Err(format!("{}: no processes defined (missing Procfile?)", name));
		}

		let mut managed_processes = HashMap::new();

		for proc_def in &service.processes {
			let output = OutputCapture::new(name, &proc_def.name, self.config.logs.max_size_bytes);
			let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);

			let mp = ManagedProcess {
				def: proc_def.clone(),
				state: ProcessState::Stopped,
				output: output.clone(),
				started_at: None,
				retry_count: 0,
				cancel: Some(cancel_tx),
			};
			managed_processes.insert(proc_def.name.clone(), mp);

			let sup = Arc::clone(self);
			let service_name = name.to_string();
			let process_name = proc_def.name.clone();
			let proc_def_clone = proc_def.clone();
			let dir = entry.dir.clone();

			tokio::spawn(async move {
				run_process_loop(sup, service_name, process_name, proc_def_clone, dir, output, cancel_rx).await;
			});
		}

		{
			let mut services = self.services.write().await;
			services.insert(
				name.to_string(),
				ManagedService {
					name: name.to_string(),
					dir: entry.dir.clone(),
					processes: managed_processes,
				},
			);
		}

		Ok(format!("{}: starting", name))
	}

	pub async fn stop_service(self: &Arc<Self>, name: &str) -> Result<String, String> {
		let mut services = self.services.write().await;
		let managed = services.get_mut(name).ok_or_else(|| format!("{}: not running", name))?;

		let mut any_running = false;
		for (_, mp) in managed.processes.iter_mut() {
			if mp.state.is_running() {
				any_running = true;
				if let Some(cancel) = mp.cancel.take() {
					let _ = cancel.send(true);
				}
				if let ProcessState::Running { pid, .. } = &mp.state {
					kill_process_tree(*pid);
				}
				mp.state = ProcessState::Stopped;
			}
		}

		if !any_running {
			return Ok(format!("{}: already stopped", name));
		}

		services.remove(name);
		Ok(format!("{}: stopped", name))
	}

	pub async fn reload_service(self: &Arc<Self>, name: &str) -> Result<String, String> {
		let _ = self.stop_service(name).await;
		tokio::time::sleep(std::time::Duration::from_millis(200)).await;
		self.start_service(name).await
	}

	pub async fn restart_process(self: &Arc<Self>, service: &str, process: &str) -> Result<String, String> {
		let entries = config::load_service_entries();
		let entry = entries.get(service).ok_or_else(|| format!("unknown service: {}", service))?;

		let mut services = self.services.write().await;
		let managed = services.get_mut(service).ok_or_else(|| format!("{}: not running", service))?;
		let mp = managed.processes.get_mut(process).ok_or_else(|| format!("{}/{}: not found", service, process))?;

		if let Some(cancel) = mp.cancel.take() {
			let _ = cancel.send(true);
		}
		if let ProcessState::Running { pid, .. } = &mp.state {
			kill_process_tree(*pid);
		}
		mp.state = ProcessState::Stopped;
		mp.retry_count = 0;

		let output = OutputCapture::new(service, process, self.config.logs.max_size_bytes);
		let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
		mp.output = output.clone();
		mp.cancel = Some(cancel_tx);

		let sup = Arc::clone(self);
		let service_name = service.to_string();
		let process_name = process.to_string();
		let proc_def = mp.def.clone();
		let dir = entry.dir.clone();

		tokio::spawn(async move {
			run_process_loop(sup, service_name, process_name, proc_def, dir, output, cancel_rx).await;
		});

		Ok(format!("{}/{}: restarting", service, process))
	}

	pub async fn kill_process(self: &Arc<Self>, service: &str, process: &str) -> Result<String, String> {
		let mut services = self.services.write().await;
		let managed = services.get_mut(service).ok_or_else(|| format!("{}: not running", service))?;
		let mp = managed.processes.get_mut(process).ok_or_else(|| format!("{}/{}: not found", service, process))?;

		if let Some(cancel) = mp.cancel.take() {
			let _ = cancel.send(true);
		}
		if let ProcessState::Running { pid, .. } = &mp.state {
			kill_process_tree(*pid);
		}
		mp.state = ProcessState::Stopped;

		Ok(format!("{}/{}: killed", service, process))
	}

	pub async fn get_output(&self, service: &str, process: Option<&str>) -> Result<OutputCapture, String> {
		let services = self.services.read().await;
		let managed = services.get(service).ok_or_else(|| format!("{}: not found", service))?;

		if let Some(proc_name) = process {
			let mp = managed.processes.get(proc_name).ok_or_else(|| format!("{}/{}: not found", service, proc_name))?;
			Ok(mp.output.clone())
		} else {
			// Return the first process's output (or we could merge them)
			managed
				.processes
				.values()
				.next()
				.map(|mp| mp.output.clone())
				.ok_or_else(|| format!("{}: no processes", service))
		}
	}

	pub async fn get_all_outputs(&self, service: &str) -> Result<Vec<(String, OutputCapture)>, String> {
		let services = self.services.read().await;
		let managed = services.get(service).ok_or_else(|| format!("{}: not found", service))?;
		Ok(managed
			.processes
			.iter()
			.map(|(name, mp)| (name.clone(), mp.output.clone()))
			.collect())
	}
}

async fn run_process_loop(
	supervisor: Arc<Supervisor>,
	service: String,
	process: String,
	def: ProcessDef,
	dir: std::path::PathBuf,
	output: OutputCapture,
	mut cancel: tokio::sync::watch::Receiver<bool>,
) {
	let mut retry_count: u32 = 0;

	loop {
		if *cancel.borrow() {
			return;
		}

		let child = spawn_process(&def, &dir).await;
		let mut child = match child {
			Ok(c) => c,
			Err(e) => {
				let msg = format!("[ubermind] failed to spawn {}/{}: {}\n", service, process, e);
				output.write(msg.as_bytes()).await;
				update_state(&supervisor, &service, &process, ProcessState::Failed { exit_code: -1 }).await;
				return;
			}
		};

		let pid = child.id().unwrap_or(0) as u32;
		let started_at = Instant::now();
		update_state(
			&supervisor,
			&service,
			&process,
			ProcessState::Running {
				pid,
				uptime_secs: 0,
			},
		)
		.await;

		if let Some(stdout) = child.stdout.take() {
			let out = output.clone();
			tokio::spawn(async move {
				pipe_output(stdout, out).await;
			});
		}
		if let Some(stderr) = child.stderr.take() {
			let out = output.clone();
			tokio::spawn(async move {
				pipe_output(stderr, out).await;
			});
		}

		// Also spawn an uptime updater
		let sup_clone = Arc::clone(&supervisor);
		let svc = service.clone();
		let proc_name = process.clone();
		let cancel_clone = cancel.clone();
		let uptime_handle = tokio::spawn(async move {
			loop {
				tokio::time::sleep(std::time::Duration::from_secs(5)).await;
				if *cancel_clone.borrow() {
					return;
				}
				let uptime = started_at.elapsed().as_secs();
				update_state(
					&sup_clone,
					&svc,
					&proc_name,
					ProcessState::Running { pid, uptime_secs: uptime },
				)
				.await;
			}
		});

		let exit_result = tokio::select! {
			status = child.wait() => status,
			_ = cancel.changed() => {
				let _ = child.kill().await;
				uptime_handle.abort();
				return;
			}
		};

		// Process exited, stop the uptime updater
		uptime_handle.abort();

		match exit_result {
			Ok(exit) if exit.success() => {
				let msg = format!("[ubermind] {}/{} exited cleanly\n", service, process);
				output.write(msg.as_bytes()).await;
				update_state(&supervisor, &service, &process, ProcessState::Stopped).await;
				return;
			}
			Ok(exit) => {
				let code = exit.code().unwrap_or(-1);
				retry_count += 1;

				if def.restart && retry_count <= def.max_retries {
					let msg = format!(
						"[ubermind] {}/{} crashed (exit {}), restarting ({}/{})\n",
						service, process, code, retry_count, def.max_retries
					);
					output.write(msg.as_bytes()).await;
					update_state(
						&supervisor,
						&service,
						&process,
						ProcessState::Crashed { exit_code: code, retries: retry_count },
					)
					.await;
					tokio::time::sleep(std::time::Duration::from_secs(def.restart_delay_secs)).await;
					continue;
				} else {
					let msg = format!(
						"[ubermind] {}/{} failed (exit {}), max retries exceeded\n",
						service, process, code
					);
					output.write(msg.as_bytes()).await;
					update_state(
						&supervisor,
						&service,
						&process,
						ProcessState::Failed { exit_code: code },
					)
					.await;
					return;
				}
			}
			Err(e) => {
				let msg = format!("[ubermind] {}/{} error: {}\n", service, process, e);
				output.write(msg.as_bytes()).await;
				update_state(&supervisor, &service, &process, ProcessState::Failed { exit_code: -1 }).await;
				return;
			}
		}
	}
}

async fn spawn_process(def: &ProcessDef, dir: &std::path::Path) -> Result<Child, String> {
	let mut cmd = Command::new("sh");
	cmd.args(["-c", &def.command])
		.current_dir(dir)
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		// Create a new process group so we can kill the tree
		.process_group(0);

	for (key, val) in &def.env {
		cmd.env(key, val);
	}

	cmd.spawn().map_err(|e| format!("spawn failed: {}", e))
}

async fn pipe_output<R: tokio::io::AsyncRead + Unpin>(mut reader: R, output: OutputCapture) {
	let mut buf = [0u8; 4096];
	loop {
		match reader.read(&mut buf).await {
			Ok(0) => break,
			Ok(n) => output.write(&buf[..n]).await,
			Err(_) => break,
		}
	}
}

async fn update_state(supervisor: &Arc<Supervisor>, service: &str, process: &str, state: ProcessState) {
	let mut services = supervisor.services.write().await;
	if let Some(managed) = services.get_mut(service) {
		if let Some(mp) = managed.processes.get_mut(process) {
			mp.state = state;
		}
	}
}

fn kill_process_tree(pid: u32) {
	use nix::sys::signal::{killpg, Signal};
	use nix::unistd::Pid;
	let pgid = Pid::from_raw(pid as i32);
	let _ = killpg(pgid, Signal::SIGTERM);
	// Give processes a moment, then force kill
	std::thread::spawn(move || {
		std::thread::sleep(std::time::Duration::from_secs(3));
		let _ = killpg(pgid, Signal::SIGKILL);
	});
}
