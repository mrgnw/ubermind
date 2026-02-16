use crate::protocol::config_dir;
use crate::types::{ProcessDef, Service};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct GlobalConfig {
	#[serde(default)]
	pub daemon: DaemonConfig,
	#[serde(default)]
	pub logs: LogsConfig,
	#[serde(default)]
	pub defaults: DefaultsConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DaemonConfig {
	#[serde(default = "default_idle_timeout")]
	pub idle_timeout: u64,
	pub log_dir: Option<String>,
	#[serde(default = "default_port")]
	pub port: u16,
}

impl Default for DaemonConfig {
	fn default() -> Self {
		Self {
			idle_timeout: default_idle_timeout(),
			log_dir: None,
			port: default_port(),
		}
	}
}

fn default_idle_timeout() -> u64 {
	300
}
fn default_port() -> u16 {
	13369
}

#[derive(Debug, Clone, Deserialize)]
pub struct LogsConfig {
	#[serde(default = "default_max_size")]
	pub max_size_bytes: u64,
	#[serde(default = "default_max_age_days")]
	pub max_age_days: u32,
	#[serde(default = "default_max_files")]
	pub max_files: u32,
}

impl Default for LogsConfig {
	fn default() -> Self {
		Self {
			max_size_bytes: default_max_size(),
			max_age_days: default_max_age_days(),
			max_files: default_max_files(),
		}
	}
}

fn default_max_size() -> u64 {
	10 * 1024 * 1024
}
fn default_max_age_days() -> u32 {
	7
}
fn default_max_files() -> u32 {
	5
}

#[derive(Debug, Clone, Deserialize)]
pub struct DefaultsConfig {
	#[serde(default = "default_true")]
	pub restart: bool,
	#[serde(default = "default_max_retries")]
	pub max_retries: u32,
	#[serde(default = "default_restart_delay")]
	pub restart_delay: u64,
	#[serde(default = "default_env")]
	pub env: HashMap<String, String>,
}

impl Default for DefaultsConfig {
	fn default() -> Self {
		Self {
			restart: true,
			max_retries: default_max_retries(),
			restart_delay: default_restart_delay(),
			env: default_env(),
		}
	}
}

fn default_true() -> bool {
	true
}
fn default_max_retries() -> u32 {
	3
}
fn default_restart_delay() -> u64 {
	1
}
fn default_env() -> HashMap<String, String> {
	let mut env = HashMap::new();
	env.insert("FORCE_COLOR".into(), "1".into());
	env.insert("CLICOLOR_FORCE".into(), "1".into());
	env
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ProjectToml {
	#[serde(default)]
	pub processes: HashMap<String, ProcessOverride>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ProcessOverride {
	pub command: Option<String>,
	pub restart: Option<bool>,
	pub max_retries: Option<u32>,
	pub restart_delay: Option<u64>,
	pub autostart: Option<bool>,
	#[serde(default)]
	pub env: HashMap<String, String>,
}

pub fn load_global_config() -> GlobalConfig {
	let path = config_dir().join("config.toml");
	if path.exists() {
		match std::fs::read_to_string(&path) {
			Ok(content) => match toml::from_str(&content) {
				Ok(config) => return config,
				Err(e) => eprintln!("warning: failed to parse {}: {}", path.display(), e),
			},
			Err(e) => eprintln!("warning: failed to read {}: {}", path.display(), e),
		}
	}
	GlobalConfig::default()
}

pub struct ServiceEntry {
	pub name: String,
	pub dir: PathBuf,
	pub command: Option<String>,
}

pub fn load_projects() -> BTreeMap<String, ServiceEntry> {
	let path = config_dir().join("projects");
	let mut services = BTreeMap::new();
	let content = match std::fs::read_to_string(&path) {
		Ok(c) => c,
		Err(_) => return services,
	};
	for line in content.lines() {
		let line = line.trim();
		if line.is_empty() || line.starts_with('#') {
			continue;
		}
		let (name, dir_str) = if let Some(pos) = line.find(':') {
			(line[..pos].trim().to_string(), line[pos + 1..].trim().to_string())
		} else if let Some(pos) = line.find('\t') {
			(line[..pos].trim().to_string(), line[pos + 1..].trim().to_string())
		} else {
			continue;
		};
		let dir = expand_tilde(&dir_str);
		if !dir.exists() {
			eprintln!("warning: directory does not exist for {}: {}", name, dir.display());
			continue;
		}
		services.insert(name.clone(), ServiceEntry { name, dir, command: None });
	}
	services
}

pub fn load_commands() -> BTreeMap<String, ServiceEntry> {
	let path = config_dir().join("commands");
	let mut services = BTreeMap::new();
	let content = match std::fs::read_to_string(&path) {
		Ok(c) => c,
		Err(_) => return services,
	};
	let commands_dir = config_dir().join("_commands");
	for line in content.lines() {
		let line = line.trim();
		if line.is_empty() || line.starts_with('#') {
			continue;
		}
		let (name, command) = if let Some(pos) = line.find(':') {
			(line[..pos].trim().to_string(), line[pos + 1..].trim().to_string())
		} else {
			continue;
		};
		let dir = commands_dir.join(&name);
		let _ = std::fs::create_dir_all(&dir);
		let procfile = dir.join("Procfile");
		let procfile_content = format!("{}: {}\n", name, command);
		let needs_write = match std::fs::read_to_string(&procfile) {
			Ok(existing) => existing != procfile_content,
			Err(_) => true,
		};
		if needs_write {
			let _ = std::fs::write(&procfile, &procfile_content);
		}
		services.insert(
			name.clone(),
			ServiceEntry { name, dir, command: Some(command) },
		);
	}
	services
}

pub fn load_service_entries() -> BTreeMap<String, ServiceEntry> {
	let mut services = load_projects();
	services.extend(load_commands());
	services
}

pub fn load_service(entry: &ServiceEntry, defaults: &DefaultsConfig) -> Service {
	let mut processes = Vec::new();
	let procfile_path = entry.dir.join("Procfile");
	if let Ok(content) = std::fs::read_to_string(&procfile_path) {
		for line in content.lines() {
			let line = line.trim();
			if line.is_empty() {
				continue;
			}

			let (proc_line, autostart) = if line.starts_with('#') {
				let after_hash = line[1..].trim_start();
				if let Some(rest) = after_hash.strip_prefix('~') {
					(rest.trim(), false)
				} else {
					continue;
				}
			} else {
				(line, true)
			};

			if let Some(pos) = proc_line.find(':') {
				let name = proc_line[..pos].trim().to_string();
				let command = proc_line[pos + 1..].trim().to_string();
				if name.is_empty() || command.is_empty() {
					continue;
				}
				processes.push(ProcessDef {
					name,
					command,
					restart: defaults.restart,
					max_retries: defaults.max_retries,
					restart_delay_secs: defaults.restart_delay,
					env: defaults.env.clone(),
					autostart,
				});
			}
		}
	}

	let toml_path = entry.dir.join(".ubermind.toml");
	if let Ok(content) = std::fs::read_to_string(&toml_path) {
		if let Ok(overrides) = toml::from_str::<ProjectToml>(&content) {
			for proc in &mut processes {
				if let Some(ov) = overrides.processes.get(&proc.name) {
					if let Some(ref cmd) = ov.command {
						proc.command = cmd.clone();
					}
					if let Some(r) = ov.restart {
						proc.restart = r;
					}
					if let Some(r) = ov.max_retries {
						proc.max_retries = r;
					}
					if let Some(r) = ov.restart_delay {
						proc.restart_delay_secs = r;
					}
					if let Some(a) = ov.autostart {
						proc.autostart = a;
					}
					proc.env.extend(ov.env.clone());
				}
			}
			for (name, ov) in &overrides.processes {
				if !processes.iter().any(|p| &p.name == name) {
					if let Some(ref cmd) = ov.command {
						processes.push(ProcessDef {
							name: name.clone(),
							command: cmd.clone(),
							restart: ov.restart.unwrap_or(defaults.restart),
							max_retries: ov.max_retries.unwrap_or(defaults.max_retries),
							restart_delay_secs: ov.restart_delay.unwrap_or(defaults.restart_delay),
							env: {
								let mut e = defaults.env.clone();
								e.extend(ov.env.clone());
								e
							},
							autostart: ov.autostart.unwrap_or(true),
						});
					}
				}
			}
		}
	}

	Service {
		name: entry.name.clone(),
		dir: entry.dir.clone(),
		processes,
	}
}

fn expand_tilde(path: &str) -> PathBuf {
	if let Some(rest) = path.strip_prefix("~/") {
		if let Some(home) = std::env::var("HOME").ok() {
			return PathBuf::from(home).join(rest);
		}
	}
	PathBuf::from(path)
}
