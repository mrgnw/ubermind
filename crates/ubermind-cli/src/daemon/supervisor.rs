use std::collections::HashMap;
use std::sync::Arc;

use crate::config::{self, GlobalConfig};
use crate::types::*;

pub struct Supervisor {
	pub inner: Arc<kagaya::Supervisor>,
	pub config: GlobalConfig,
	pub http_port: Option<u16>,
}

impl Supervisor {
	pub fn new(config: GlobalConfig, http_port: Option<u16>) -> Arc<Self> {
		let log_dir = crate::logs::log_dir();
		let inner = kagaya::Supervisor::new(kagaya::SupervisorConfig {
			log_dir,
			max_log_size: config.logs.max_size_bytes,
		});
		Arc::new(Self {
			inner,
			config,
			http_port,
		})
	}

	pub async fn status(self: &Arc<Self>) -> Vec<ServiceStatus> {
		let entries = config::load_service_entries();
		let services = self.inner.services.read().await;
		let running_pids: Vec<u32> = services
			.values()
			.flat_map(|s| s.processes.values())
			.filter_map(|mp| match &mp.state {
				ProcessState::Running { pid, .. } => Some(*pid),
				_ => None,
			})
			.collect();
		let pid_ports = listening_ports_for_pids(&running_pids);
		let mut result = Vec::new();

		for (name, entry) in &entries {
			if let Some(managed) = services.get(name) {
				let processes = managed
					.processes
					.iter()
					.map(|(pname, mp)| {
						let pid = match &mp.state {
							ProcessState::Running { pid, .. } => Some(*pid),
							_ => None,
						};
						let ports = pid
							.and_then(|p| pid_ports.get(&p))
							.cloned()
							.unwrap_or_default();
						ProcessStatus {
							name: pname.clone(),
							state: mp.state.clone(),
							pid,
							autostart: mp.def.autostart,
							service_type: mp.def.service_type.clone(),
							ports,
						}
					})
					.collect();
				result.push(ServiceStatus {
					name: name.clone(),
					dir: entry.dir.clone(),
					processes,
				});
			} else {
				let service = config::load_service(entry, &self.config.defaults);
				let processes = service
					.processes
					.iter()
					.map(|p| ProcessStatus {
						name: p.name.clone(),
						state: ProcessState::Stopped,
						pid: None,
						autostart: p.autostart,
						service_type: p.service_type.clone(),
						ports: vec![],
					})
					.collect();
				result.push(ServiceStatus {
					name: name.clone(),
					dir: entry.dir.clone(),
					processes,
				});
			}
		}
		result
	}

	pub async fn start_service_filtered(
		self: &Arc<Self>,
		name: &str,
		all: bool,
		processes: &[String],
	) -> Result<String, String> {
		let entries = config::load_service_entries();
		let entry = entries
			.get(name)
			.ok_or_else(|| format!("unknown service: {}", name))?;

		let service = config::load_service(entry, &self.config.defaults);
		if service.processes.is_empty() {
			return Err(format!(
				"{}: no processes defined (missing services.toml?)",
				name
			));
		}

		self.inner
			.start_service(name, &entry.dir, &service.processes, all, processes)
			.await
	}

	pub async fn stop_service(self: &Arc<Self>, name: &str) -> Result<String, String> {
		self.inner.stop_service(name).await
	}

	pub async fn reload_service_filtered(
		self: &Arc<Self>,
		name: &str,
		all: bool,
		processes: &[String],
	) -> Result<String, String> {
		let entries = config::load_service_entries();
		let entry = entries
			.get(name)
			.ok_or_else(|| format!("unknown service: {}", name))?;

		let service = config::load_service(entry, &self.config.defaults);
		self.inner
			.reload_service(name, &entry.dir, &service.processes, all, processes)
			.await
	}

	pub async fn restart_process(
		self: &Arc<Self>,
		service: &str,
		process: &str,
	) -> Result<String, String> {
		let entries = config::load_service_entries();
		let entry = entries
			.get(service)
			.ok_or_else(|| format!("unknown service: {}", service))?;

		self.inner.restart_process(service, process, &entry.dir).await
	}

	pub async fn kill_process(
		self: &Arc<Self>,
		service: &str,
		process: &str,
	) -> Result<String, String> {
		self.inner.kill_process(service, process).await
	}

	pub async fn get_output(
		&self,
		service: &str,
		process: Option<&str>,
	) -> Result<kagaya::OutputCapture, String> {
		self.inner.get_output(service, process).await
	}

	pub async fn get_all_outputs(
		&self,
		service: &str,
	) -> Result<Vec<(String, kagaya::OutputCapture)>, String> {
		self.inner.get_all_outputs(service).await
	}
}

#[cfg(target_os = "macos")]
fn listening_ports_for_pids(target_pids: &[u32]) -> HashMap<u32, Vec<u16>> {
	use libproc::processes::{pids_by_type, ProcFilter};
	use netstat2::*;

	let af = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
	let proto = ProtocolFlags::TCP;
	let sockets = match get_sockets_info(af, proto) {
		Ok(s) => s,
		Err(_) => return HashMap::new(),
	};

	let mut all_ports: HashMap<u32, Vec<u16>> = HashMap::new();
	for si in &sockets {
		if let ProtocolSocketInfo::Tcp(ref tcp) = si.protocol_socket_info {
			if tcp.state == TcpState::Listen {
				for pid in &si.associated_pids {
					let ports = all_ports.entry(*pid).or_default();
					if !ports.contains(&tcp.local_port) {
						ports.push(tcp.local_port);
					}
				}
			}
		}
	}

	let mut result: HashMap<u32, Vec<u16>> = HashMap::new();
	for &pid in target_pids {
		if let Some(ports) = all_ports.get(&pid) {
			result.insert(pid, ports.clone());
			continue;
		}
		let group_pids = pids_by_type(ProcFilter::ByProgramGroup { pgrpid: pid }).unwrap_or_default();
		let mut ports: Vec<u16> = Vec::new();
		for gpid in &group_pids {
			if let Some(p) = all_ports.get(gpid) {
				for port in p {
					if !ports.contains(port) {
						ports.push(*port);
					}
				}
			}
		}
		if !ports.is_empty() {
			ports.sort();
			result.insert(pid, ports);
		}
	}
	result
}

#[cfg(not(target_os = "macos"))]
fn listening_ports_for_pids(_target_pids: &[u32]) -> HashMap<u32, Vec<u16>> {
	HashMap::new()
}
