mod config;
mod daemon;
mod launchd;
mod logs;
mod protocol;
mod self_update;
mod types;

use std::collections::BTreeMap;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;
use config::ServiceEntry;
use protocol::{Request, Response};
use types::*;
use owo_colors::OwoColorize;

fn main() {
	let args: Vec<String> = std::env::args().skip(1).collect();

	if args.is_empty() {
		print_usage();
		if connect_daemon().is_some() {
			eprintln!();
			render_status(&[]);
		}
		check_alias_hint();
		return;
	}

	match args[0].as_str() {
		"help" | "--help" | "-h" => print_usage(),
		"version" | "--version" | "-V" => println!("ubermind {}", env!("CARGO_PKG_VERSION")),
		"init" => cmd_init(),
		"add" => cmd_add(&args[1..]),
		"status" | "st" => cmd_status(&args[1..]),
		"all" => cmd_status(&["all".to_string()]),
		"start" => cmd_start(&args[1..]),
		"stop" => cmd_stop(&args[1..]),
		"reload" => cmd_reload(&args[1..]),
		"restart" => cmd_restart(&args[1..]),
		"logs" => cmd_logs(&args[1..]),
		"tail" => cmd_tail(&args[1..]),
		"echo" => cmd_echo(&args[1..]),
		"show" => cmd_show(&args[1..]),
		"daemon" => cmd_daemon(&args[1..]),
		"serve" => cmd_serve(&args[1..]),
		"launchd" | "launch" => launchd::cmd_launchd(&args[1..]),
		"self" => {
			match args.get(1).map(|s| s.as_str()) {
				Some("update") => self_update::cmd_self_update(),
				_ => {
					eprintln!("usage: ub self update");
					std::process::exit(1);
				}
			}
		}
		name => {
			let services = config::load_service_entries();
			let base_name = name.split('.').next().unwrap_or(name);
			if services.contains_key(base_name) && args.len() > 1 {
				match args[1].as_str() {
					"start" => cmd_start(&[args[0].clone()]),
					"stop" => cmd_stop(&[args[0].clone()]),
					"reload" => cmd_reload(&[args[0].clone()]),
					"status" | "st" => cmd_status(&[args[0].clone()]),
					"logs" => cmd_logs(&args),
					"tail" => cmd_tail(&args),
					"echo" => cmd_echo(&args),
					"show" => cmd_show(&args),
					"restart" => {
						if args.len() > 2 {
							cmd_restart(&[args[0].clone(), args[2].clone()]);
						} else {
							cmd_reload(&[args[0].clone()]);
						}
					}
					_ => {
						eprintln!("unknown command: {}", args[1]);
						std::process::exit(1);
					}
				}
			} else if services.contains_key(base_name) {
				cmd_status(&[args[0].clone()]);
			} else {
				eprintln!("unknown command or service: {}", name);
				eprintln!();
				let names: Vec<&str> = services.keys().map(|s| s.as_str()).collect();
				if !names.is_empty() {
					eprintln!("registered services: {}", names.join(", "));
				}
				eprintln!("run 'ubermind help' for usage");
				std::process::exit(1);
			}
		}
	}
}

fn print_usage() {
	eprintln!("{} {} — process daemon manager", "ubermind".bold(), env!("CARGO_PKG_VERSION"));
	eprintln!();
	eprintln!("usage: {} [command] [service] [options]", "ub".bold());
	eprintln!();

	eprintln!("{}", "services".cyan().bold());
	eprintln!("  {} [name|--all]          Show status (default command)", "status".bold());
	eprintln!("  {} [name|--all]           Start service(s)", "start".bold());
	eprintln!("  {} [name|--all]            Stop service(s)", "stop".bold());
	eprintln!("  {} [name|--all]          Reload (stop + start)", "reload".bold());
	eprintln!("  {} [name] [process]     Restart a single process", "restart".bold());
	eprintln!();

	eprintln!("{}", "logs".cyan().bold());
	eprintln!("  {} <name> [process]        Last 100 lines of log file", "logs".bold());
	eprintln!("  {} <name> [process]        Follow log file (tail -f)", "tail".bold());
	eprintln!("  {} <name> [process]        Live output stream from daemon", "echo".bold());
	eprintln!();

	eprintln!("{}", "config".cyan().bold());
	eprintln!("  {} [name] [process]        Show Procfile or process command", "show".bold());
	eprintln!("  {} [name] [dir]             Register a project", "add".bold());
	eprintln!("  {}                         Create config files", "init".bold());
	eprintln!();

	eprintln!("{}", "system".cyan().bold());
	eprintln!("  {} [start|stop|status]   Manage the daemon", "daemon".bold());
	eprintln!("  {} [-d|--stop|--status]   HTTP server for web UI", "serve".bold());
	eprintln!("  {} [command]            macOS launchd agents", "launchd".bold());
	eprintln!("  {}                  Update to latest version", "self update".bold());
	eprintln!();

	eprintln!("{}", "targeting".cyan().bold());
	eprintln!("  Use {} dot syntax to target a specific process:", "name.process".bold());
	eprintln!("    ub status matrix.automation");
	eprintln!("  Context-aware: run from a project dir to auto-target it");
	eprintln!("    ub restart api             restart 'api' in current project");
	eprintln!("    ub restart appligator api  target a specific project");
	eprintln!();

	eprintln!("{}", "shortcuts".cyan().bold());
	eprintln!("    ub                         status (current project or all)");
	eprintln!("    ub all                     status --all");
	eprintln!("    ub --watch                 status --watch (live refresh)");
}

// --- Config management (no daemon needed) ---

fn cmd_init() {
	let config_dir = protocol::config_dir();
	let _ = std::fs::create_dir_all(&config_dir);

	let projects_file = config_dir.join("projects");
	if !projects_file.exists() {
		let content = "# name: /path/to/project\n# myapp: ~/dev/myapp\n";
		let _ = std::fs::write(&projects_file, content);
		eprintln!("created {}", projects_file.display());
	} else {
		eprintln!("already exists: {}", projects_file.display());
	}

	let commands_file = config_dir.join("commands");
	if !commands_file.exists() {
		let content = "# name: shell command\n# tunnel: ssh -N -L 5432:localhost:5432 myserver\n";
		let _ = std::fs::write(&commands_file, content);
		eprintln!("created {}", commands_file.display());
	}

	eprintln!();
	eprintln!("getting started:");
	eprintln!("  1. add projects: ub add (from a project dir)");
	eprintln!("  2. start: ub start [name|--all]");
	eprintln!("  3. check: ub status");
}

fn cmd_add(args: &[String]) {
	let config_dir = protocol::config_dir();
	let _ = std::fs::create_dir_all(&config_dir);
	let projects_file = config_dir.join("projects");

	let (name, dir) = if args.len() >= 2 {
		(args[0].clone(), PathBuf::from(&args[1]))
	} else if args.len() == 1 {
		let dir = std::env::current_dir().unwrap();
		(args[0].clone(), dir)
	} else {
		let dir = std::env::current_dir().unwrap();
		let name = dir
			.file_name()
			.unwrap_or_default()
			.to_string_lossy()
			.to_lowercase()
			.chars()
			.map(|c| if c.is_alphanumeric() { c } else { '-' })
			.collect::<String>();
		(name, dir)
	};

	let dir = dir.canonicalize().unwrap_or(dir);

	if !dir.exists() {
		eprintln!("error: directory does not exist: {}", dir.display());
		std::process::exit(1);
	}

	if let Ok(content) = std::fs::read_to_string(&projects_file) {
		for line in content.lines() {
			let line = line.trim();
			if line.is_empty() || line.starts_with('#') {
				continue;
			}
			if let Some(pos) = line.find(':') {
				if line[..pos].trim() == name {
					eprintln!("{}: already registered", name);
					return;
				}
			}
		}
	}

	let procfile = dir.join("Procfile");
	if !procfile.exists() {
		eprintln!("note: no Procfile found in {}", dir.display());
		eprintln!("create one with process definitions, e.g.:");
		eprintln!("  web: npm run dev");
	}

	let mut file = std::fs::OpenOptions::new()
		.create(true)
		.append(true)
		.open(&projects_file)
		.unwrap();
	writeln!(file, "{}: {}", name, dir.display()).unwrap();
	eprintln!("{}: added ({})", name, dir.display());
}

// --- Daemon communication ---

fn connect_daemon() -> Option<UnixStream> {
	let socket_path = protocol::socket_path();
	UnixStream::connect(&socket_path).ok()
}

fn ensure_daemon() -> UnixStream {
	if let Some(stream) = connect_daemon() {
		return stream;
	}

	eprintln!("starting daemon...");
	let daemon_bin = find_daemon_binary();

	let mut cmd = Command::new(&daemon_bin);
	cmd.args(["daemon", "run"])
		.stdout(std::process::Stdio::null())
		.stderr(std::process::Stdio::null());

	match cmd.spawn() {
		Ok(_) => {}
		Err(e) => {
			eprintln!("error: failed to start daemon: {}", e);
			eprintln!("binary: {}", daemon_bin.display());
			std::process::exit(1);
		}
	}

	for _ in 0..50 {
		std::thread::sleep(std::time::Duration::from_millis(100));
		if let Some(stream) = connect_daemon() {
			return stream;
		}
	}

	eprintln!("error: daemon did not start in time");
	std::process::exit(1);
}

fn find_daemon_binary() -> PathBuf {
	std::env::current_exe().unwrap_or_else(|_| PathBuf::from("ubermind"))
}

fn send_request(request: &Request) -> Response {
	let mut stream = ensure_daemon();
	let mut data = serde_json::to_vec(request).unwrap();
	data.push(b'\n');
	stream.write_all(&data).unwrap();

	let mut reader = BufReader::new(&stream);
	let mut line = String::new();
	reader.read_line(&mut line).unwrap();

	serde_json::from_str(&line).unwrap_or(Response::Error {
		message: "failed to parse daemon response".to_string(),
	})
}

// --- Commands that talk to daemon ---

fn cmd_status(args: &[String]) {
	let (watch, rest) = parse_watch_opts(args, None);
	if watch.enabled {
		watch_status(&rest, &watch);
	} else {
		render_status(&rest);
	}
}

fn print_process_line(proc: &ProcessStatus, name_width: usize) {
	let (circle, uptime, pid, label) = match &proc.state {
		ProcessState::Running { pid, uptime_secs } => {
			("●".green().to_string(), format_uptime(*uptime_secs), format!("{}", pid), "on".green().to_string())
		}
		ProcessState::Stopped if !proc.autostart => {
			("○".dimmed().to_string(), "-".to_string(), "-".to_string(), "optional".dimmed().to_string())
		}
		ProcessState::Stopped => {
			("●".red().to_string(), "-".to_string(), "-".to_string(), "off".red().to_string())
		}
		ProcessState::Crashed { exit_code, retries } => {
			("●".yellow().to_string(), format!("exit {}", exit_code), format!("retry {}", retries), "crashed".yellow().to_string())
		}
		ProcessState::Failed { exit_code } => {
			("●".red().to_string(), format!("exit {}", exit_code), "-".to_string(), "failed".red().to_string())
		}
	};
	let ports = if proc.ports.is_empty() {
		String::new()
	} else {
		format!(" {}", proc.ports.iter().map(|p| format!(":{}", p)).collect::<Vec<_>>().join(","))
	};
	println!("{} {:<width$} {:<8} {:<8} {}{}", circle, proc.name, uptime, pid, label, ports, width = name_width);
}

fn cmd_start(args: &[String]) {
	let (mut watch, rest) = parse_watch_opts(args, Some(4));
	let entries = config::load_service_entries();

	let start_all = rest.iter().any(|a| is_all_flag(a));
	let rest: Vec<String> = rest.into_iter().filter(|a| !is_all_flag(a)).collect();

	let mut target_processes: Vec<String> = Vec::new();
	let resolved: Vec<String> = if rest.is_empty() {
		resolve_target_names(&[], &entries)
	} else {
		let mut service_names = Vec::new();
		for arg in &rest {
			let (svc, proc) = resolve_dot_target(arg, &entries);
			if let Some(p) = proc {
				if !service_names.contains(&svc) {
					service_names.push(svc);
				}
				if !target_processes.contains(&p) {
					target_processes.push(p);
				}
			} else if entries.contains_key(&svc) {
				if !service_names.contains(&svc) {
					service_names.push(svc);
				}
			} else if let Some(current) = get_current_project(&entries) {
				if !service_names.contains(&current) {
					service_names.push(current);
				}
				if !target_processes.contains(&svc) {
					target_processes.push(svc);
				}
			} else {
				eprintln!("unknown service: {}", svc);
				eprintln!("registered services: {}", entries.keys().cloned().collect::<Vec<_>>().join(", "));
				std::process::exit(1);
			}
		}
		service_names
	};

	if resolved.is_empty() {
		eprintln!("no services to start");
		std::process::exit(1);
	}

	let response = send_request(&Request::Start {
		names: resolved.clone(),
		all: start_all || !target_processes.is_empty(),
		processes: target_processes,
	});
	match response {
		Response::Ok { message } => {
			if let Some(msg) = message {
				for line in msg.lines() {
					eprintln!("{}", line);
				}
			}
			std::thread::sleep(std::time::Duration::from_millis(500));

			if !watch.enabled {
				watch.enabled = true;
				watch.duration = Some(4);
			}
			watch_status(&resolved, &watch);
		}
		Response::Error { message } => {
			eprintln!("error: {}", message);
			std::process::exit(1);
		}
		_ => {}
	}
}

fn cmd_stop(args: &[String]) {
	let (mut watch, rest) = parse_watch_opts(args, Some(4));
	let entries = config::load_service_entries();
	let names = resolve_target_names(&rest, &entries);

	if names.is_empty() {
		eprintln!("no services to stop");
		std::process::exit(1);
	}

	let response = send_request(&Request::Stop { names: names.clone() });
	match response {
		Response::Ok { message } => {
			if let Some(msg) = message {
				for line in msg.lines() {
					eprintln!("{}", line);
				}
			}
			std::thread::sleep(std::time::Duration::from_millis(500));

			if !watch.enabled {
				watch.enabled = true;
				watch.duration = Some(4);
			}
			watch_status(&names, &watch);
		}
		Response::Error { message } => {
			eprintln!("error: {}", message);
			std::process::exit(1);
		}
		_ => {}
	}
}

fn cmd_reload(args: &[String]) {
	let (mut watch, rest) = parse_watch_opts(args, Some(4));
	let entries = config::load_service_entries();

	let reload_all = rest.iter().any(|a| is_all_flag(a));
	let rest: Vec<String> = rest.into_iter().filter(|a| !is_all_flag(a)).collect();
	let names = resolve_target_names(&rest, &entries);

	if names.is_empty() {
		eprintln!("no services to reload");
		std::process::exit(1);
	}

	let response = send_request(&Request::Reload {
		names: names.clone(),
		all: reload_all,
		processes: Vec::new(),
	});
	match response {
		Response::Ok { message } => {
			if let Some(msg) = message {
				for line in msg.lines() {
					eprintln!("{}", line);
				}
			}
			std::thread::sleep(std::time::Duration::from_millis(500));

			if !watch.enabled {
				watch.enabled = true;
				watch.duration = Some(4);
			}
			watch_status(&names, &watch);
		}
		Response::Error { message } => {
			eprintln!("error: {}", message);
			std::process::exit(1);
		}
		_ => {}
	}
}

fn cmd_restart(args: &[String]) {
	let (mut watch, rest) = parse_watch_opts(args, Some(4));
	let entries = config::load_service_entries();

	if !watch.enabled {
		watch.enabled = true;
		watch.duration = Some(4);
	}

	let mut reload_extra: Vec<String> = Vec::new();
	reload_extra.push("--watch".to_string());
	if let Some(d) = watch.duration {
		reload_extra.push(d.to_string());
	}
	if watch.interval != 1 {
		reload_extra.push("--watch-interval".to_string());
		reload_extra.push(watch.interval.to_string());
	}

	let (service, process) = if rest.is_empty() {
		if let Some(current) = get_current_project(&entries) {
			let mut reload_args = vec![current];
			reload_args.extend(reload_extra);
			return cmd_reload(&reload_args);
		} else {
			eprintln!("usage: ub restart <service> [process]");
			eprintln!("or run from a registered project directory");
			std::process::exit(1);
		}
	} else if rest.len() == 1 {
		let (svc, proc) = resolve_dot_target(&rest[0], &entries);
		if let Some(proc_name) = proc {
			(svc, Some(proc_name))
		} else if entries.contains_key(&svc) {
			let mut reload_args = vec![svc];
			reload_args.extend(reload_extra);
			return cmd_reload(&reload_args);
		} else if let Some(current) = get_current_project(&entries) {
			(current, Some(svc))
		} else {
			eprintln!("unknown service: {}", rest[0]);
			eprintln!("registered services: {}", entries.keys().cloned().collect::<Vec<_>>().join(", "));
			std::process::exit(1);
		}
	} else {
		let (svc, proc) = resolve_dot_target(&rest[0], &entries);
		(svc, proc.or_else(|| Some(rest[1].clone())))
	};

	if let Some(process_name) = process {
		let response = send_request(&Request::Restart {
			service: service.clone(),
			process: process_name.clone(),
		});
		match response {
			Response::Ok { message } => {
				if let Some(msg) = message {
					eprintln!("{}", msg);
				}
				std::thread::sleep(std::time::Duration::from_millis(500));
				watch_status(&[service], &watch);
			}
			Response::Error { message } => {
				eprintln!("error: {}", message);
				std::process::exit(1);
			}
			_ => {}
		}
	} else {
		let mut reload_args = vec![service];
		reload_args.extend(reload_extra);
		cmd_reload(&reload_args);
	}
}

fn cmd_logs(args: &[String]) {
	let svc_entries = config::load_service_entries();

	let (service, process) = if args.is_empty() {
		if let Some(current) = get_current_project(&svc_entries) {
			(current, None)
		} else {
			eprintln!("usage: ub logs <service> [process]");
			eprintln!("       ub logs <service.process>");
			std::process::exit(1);
		}
	} else {
		let (svc, proc) = resolve_dot_target(&args[0], &svc_entries);
		(svc, proc.or_else(|| args.get(1).map(|s| s.to_string())))
	};

	let log_dir = logs::service_log_dir(&service);
	if !log_dir.exists() {
		eprintln!("no logs for {}", service);
		std::process::exit(1);
	}

	let mut files: Vec<PathBuf> = Vec::new();
	if let Ok(dir_entries) = std::fs::read_dir(&log_dir) {
		for entry in dir_entries.flatten() {
			let path = entry.path();
			let name = path
				.file_name()
				.unwrap_or_default()
				.to_string_lossy()
				.to_string();
			if !name.ends_with(".log") {
				continue;
			}
			if let Some(ref proc_filter) = process {
				if !name.starts_with(proc_filter.as_str()) {
					continue;
				}
			}
			files.push(path);
		}
	}

	files.sort();

	if files.is_empty() {
		eprintln!("no log files found");
		std::process::exit(1);
	}

	let latest = files.last().unwrap();
	let content = std::fs::read_to_string(latest).unwrap_or_default();

	let lines: Vec<&str> = content.lines().collect();
	let start = if lines.len() > 100 {
		lines.len() - 100
	} else {
		0
	};
	for line in &lines[start..] {
		println!("{}", line);
	}
}

fn cmd_tail(args: &[String]) {
	let svc_entries = config::load_service_entries();

	let (service, process) = if args.is_empty() {
		if let Some(current) = get_current_project(&svc_entries) {
			(current, None)
		} else {
			eprintln!("usage: ub tail <service> [process]");
			eprintln!("       ub tail <service.process>");
			std::process::exit(1);
		}
	} else {
		let (svc, proc) = resolve_dot_target(&args[0], &svc_entries);
		(svc, proc.or_else(|| args.get(1).cloned()))
	};

	let log_dir = logs::service_log_dir(&service);
	if !log_dir.exists() {
		eprintln!("no logs for {}", service);
		std::process::exit(1);
	}

	let mut files: Vec<PathBuf> = Vec::new();
	if let Ok(dir_entries) = std::fs::read_dir(&log_dir) {
		for entry in dir_entries.flatten() {
			let path = entry.path();
			let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
			if !name.ends_with(".log") {
				continue;
			}
			if let Some(ref proc_filter) = process {
				if !name.starts_with(proc_filter.as_str()) {
					continue;
				}
			}
			files.push(path);
		}
	}

	files.sort();

	if files.is_empty() {
		eprintln!("no log files found");
		std::process::exit(1);
	}

	let latest = files.last().unwrap();
	let mut cmd = Command::new("tail");
	cmd.args(["-f", "-n", "100"]);
	cmd.arg(latest);
	let status = cmd.status().unwrap_or_else(|e| {
		eprintln!("error: {}", e);
		std::process::exit(1);
	});
	std::process::exit(status.code().unwrap_or(1));
}

fn cmd_echo(args: &[String]) {
	let svc_entries = config::load_service_entries();

	let (service, process) = if args.is_empty() {
		if let Some(current) = get_current_project(&svc_entries) {
			(current, None)
		} else {
			eprintln!("usage: ub echo <service> [process]");
			eprintln!("       ub echo <service.process>");
			std::process::exit(1);
		}
	} else {
		let (svc, proc) = resolve_dot_target(&args[0], &svc_entries);
		(svc, proc.or_else(|| args.get(1).cloned()))
	};

	loop {
		let response = send_request(&Request::Logs {
			service: service.clone(),
			process: process.clone(),
			follow: true,
		});

		match response {
			Response::Log { line } => {
				print!("{}", line);
				let _ = io::stdout().flush();
			}
			Response::Error { message } => {
				eprintln!("error: {}", message);
				std::process::exit(1);
			}
			_ => {}
		}

		std::thread::sleep(std::time::Duration::from_millis(100));
	}
}

fn cmd_show(args: &[String]) {
	let entries = config::load_service_entries();

	let filtered_args: Vec<String> = if args.len() >= 2 && args[1] == "show" {
		let mut new_args = vec![args[0].clone()];
		new_args.extend_from_slice(&args[2..]);
		new_args
	} else {
		args.to_vec()
	};

	let (service_name, process_name) = if filtered_args.is_empty() {
		if let Some(current) = get_current_project(&entries) {
			(current, None)
		} else {
			eprintln!("usage: ub show [service] [process]");
			eprintln!("or run from a registered project directory");
			std::process::exit(1);
		}
	} else if filtered_args.len() == 1 {
		if entries.contains_key(&filtered_args[0]) {
			(filtered_args[0].clone(), None)
		} else if let Some(current) = get_current_project(&entries) {
			(current, Some(filtered_args[0].clone()))
		} else {
			eprintln!("unknown service: {}", filtered_args[0]);
			eprintln!("registered services: {}", entries.keys().cloned().collect::<Vec<_>>().join(", "));
			std::process::exit(1);
		}
	} else {
		(filtered_args[0].clone(), Some(filtered_args[1].clone()))
	};

	let service_entry = match entries.get(&service_name) {
		Some(entry) => entry,
		None => {
			eprintln!("unknown service: {}", service_name);
			std::process::exit(1);
		}
	};

	let procfile_path = service_entry.dir.join("Procfile");

	if !procfile_path.exists() {
		eprintln!("Procfile not found: {}", procfile_path.display());
		std::process::exit(1);
	}

	let content = match std::fs::read_to_string(&procfile_path) {
		Ok(c) => c,
		Err(e) => {
			eprintln!("failed to read Procfile: {}", e);
			std::process::exit(1);
		}
	};

	if let Some(proc_name) = process_name {
		let mut found = false;
		for line in content.lines() {
			let line = line.trim();
			if line.is_empty() || line.starts_with('#') {
				continue;
			}
			if let Some((name, cmd)) = line.split_once(':') {
				if name.trim() == proc_name {
					println!("{}", cmd.trim());
					found = true;
					break;
				}
			}
		}
		if !found {
			eprintln!("process '{}' not found in {}", proc_name, procfile_path.display());
			std::process::exit(1);
		}
	} else {
		println!("{}", procfile_path.display().dimmed());
		println!();
		for line in content.lines() {
			let line_trimmed = line.trim();
			if line_trimmed.is_empty() {
				println!();
			} else if line_trimmed.starts_with('#') {
				let after_hash = line_trimmed[1..].trim_start();
				if let Some(rest) = after_hash.strip_prefix('~') {
					let rest = rest.trim();
					if let Some((name, cmd)) = rest.split_once(':') {
						println!("{} {}:{}", "~".dimmed(), name.cyan().dimmed(), cmd.dimmed());
					} else {
						println!("{}", line.dimmed());
					}
				} else {
					println!("{}", line.dimmed());
				}
			} else if let Some((name, cmd)) = line.split_once(':') {
				println!("{}:{}", name.cyan(), cmd);
			} else {
				println!("{}", line);
			}
		}
	}
}

fn cmd_daemon(args: &[String]) {
	let subcmd = args.first().map(|s| s.as_str()).unwrap_or("status");

	match subcmd {
		"run" => {
			// Run the daemon in-process (this is the actual daemon entry point)
			let daemon_args: Vec<String> = args[1..].to_vec();
			tokio::runtime::Runtime::new()
				.unwrap()
				.block_on(daemon::run(&daemon_args));
		}
		"start" => {
			if connect_daemon().is_some() {
				eprintln!("daemon already running");
				return;
			}
			let extra_args: Vec<String> = args[1..].iter().cloned().collect();
			let daemon_bin = find_daemon_binary();
			let mut cmd = Command::new(&daemon_bin);
			let mut spawn_args = vec!["daemon".to_string(), "run".to_string()];
			spawn_args.extend(extra_args);
			cmd.args(&spawn_args)
				.stdout(std::process::Stdio::null())
				.stderr(std::process::Stdio::null());
			match cmd.spawn() {
				Ok(_) => eprintln!("daemon started"),
				Err(e) => {
					eprintln!("error: {}", e);
					std::process::exit(1);
				}
			}
		}
		"stop" => {
			let response = send_request(&Request::Shutdown);
			match response {
				Response::Ok { message } => {
					eprintln!("daemon: {}", message.unwrap_or_default());
				}
				_ => eprintln!("daemon not running"),
			}
		}
		"status" => {
			if connect_daemon().is_some() {
				let pid = std::fs::read_to_string(protocol::pid_path()).unwrap_or_default();
				eprintln!("daemon running (pid {})", pid.trim());
			} else {
				eprintln!("daemon not running");
			}
		}
		_ => {
			eprintln!("usage: ub daemon [start|stop|status|run]");
		}
	}
}

fn cmd_serve(args: &[String]) {
	let has_stop = args.iter().any(|a| a == "--stop");
	let has_status = args.iter().any(|a| a == "--status");
	let has_daemon = args.iter().any(|a| a == "-d" || a == "--daemon");

	if has_stop {
		cmd_daemon(&["stop".to_string()].to_vec());
	} else if has_status {
		cmd_daemon(&["status".to_string()].to_vec());
	} else if has_daemon {
		cmd_daemon(&vec!["start".to_string(), "--http".to_string()]);
	} else {
		// Foreground: run daemon in-process with --http
		cmd_daemon(&vec!["run".to_string(), "--foreground".to_string(), "--http".to_string()]);
	}
}

// --- Watch support ---

struct WatchOpts {
	duration: Option<u64>,
	interval: u64,
	enabled: bool,
}

fn parse_watch_opts(args: &[String], default_duration: Option<u64>) -> (WatchOpts, Vec<String>) {
	let mut opts = WatchOpts {
		duration: None,
		interval: 1,
		enabled: false,
	};
	let mut rest = Vec::new();
	let mut i = 0;
	while i < args.len() {
		match args[i].as_str() {
			"--watch" | "-w" => {
				opts.enabled = true;
				if i + 1 < args.len() {
					if let Ok(n) = args[i + 1].parse::<u64>() {
						opts.duration = Some(n);
						i += 1;
					}
				}
				if opts.duration.is_none() {
					opts.duration = default_duration;
				}
			}
			"--watch-interval" => {
				if i + 1 < args.len() {
					if let Ok(n) = args[i + 1].parse::<u64>() {
						opts.interval = n.max(1);
						i += 1;
					}
				}
			}
			_ => rest.push(args[i].clone()),
		}
		i += 1;
	}
	(opts, rest)
}

fn fetch_status() -> (Vec<ServiceStatus>, Option<u16>) {
	let response = send_request(&Request::Status);
	match response {
		Response::Status { services, http_port } => (services, http_port),
		Response::Error { message } => {
			eprintln!("error: {}", message);
			std::process::exit(1);
		}
		_ => {
			eprintln!("unexpected response from daemon");
			std::process::exit(1);
		}
	}
}

fn render_status(args: &[String]) -> usize {
	let (services, http_port) = fetch_status();
	let entries = config::load_service_entries();

	let (process_filter, resolved_args) = if let Some(first) = args.first() {
		let (svc, proc) = resolve_dot_target(first, &entries);
		let rest: Vec<String> = std::iter::once(svc).chain(args[1..].iter().cloned()).collect();
		(proc, rest)
	} else {
		(None, args.to_vec())
	};

	let show_all = !resolved_args.is_empty() && (resolved_args.len() == 1 && is_all_flag(&resolved_args[0]) || resolved_args.iter().any(|a| is_all_flag(a)));
	let current_project = get_current_project(&entries);

	let filter = if resolved_args.is_empty() {
		if let Some(ref current) = current_project {
			vec![current.clone()]
		} else {
			entries.keys().cloned().collect()
		}
	} else if show_all {
		entries.keys().cloned().collect()
	} else {
		resolve_target_names(&resolved_args, &entries)
	};

	let mut status_map: std::collections::HashMap<String, &ServiceStatus> =
		std::collections::HashMap::new();
	for s in &services {
		status_map.insert(s.name.clone(), s);
	}

	let mut sorted_filter = filter.clone();
	if let Some(ref current) = current_project {
		sorted_filter.sort_by(|a, b| {
			if a == current {
				std::cmp::Ordering::Less
			} else if b == current {
				std::cmp::Ordering::Greater
			} else {
				a.cmp(b)
			}
		});
	}

	if let Some(ref proc_name) = process_filter {
		for name in &sorted_filter {
			if let Some(status) = status_map.get(name) {
				for proc in &status.processes {
					if proc.name == *proc_name {
						print_process_line(proc, proc.name.len());
						return 1;
					}
				}
				eprintln!("process '{}' not found in {}", proc_name, name);
				std::process::exit(1);
			} else {
				eprintln!("service '{}' not running", name);
				std::process::exit(1);
			}
		}
		return 0;
	}

	let max_name_width = sorted_filter.iter().map(|n| n.len()).max().unwrap_or(0);
	let max_proc_name_width = sorted_filter
		.iter()
		.filter_map(|name| status_map.get(name))
		.flat_map(|s| s.processes.iter().map(|p| p.name.len()))
		.max()
		.unwrap_or(0);

	let mut lines = 0usize;
	for name in &sorted_filter {
		let entry = entries.get(name);
		let status = status_map.get(name);
		let running = status.map(|s| s.is_running()).unwrap_or(false);

		let detail = if let Some(entry) = entry {
			if let Some(ref cmd) = entry.command {
				cmd.clone()
			} else {
				entry.dir.to_string_lossy().to_string()
			}
		} else {
			String::new()
		};

		let circle = if running { "●".green().to_string() } else { "●".red().to_string() };
		println!(" {} {:<width$} {}", circle, name, detail, width = max_name_width);
		lines += 1;

		if let Some(status) = status {
			for proc in &status.processes {
				print!("   └ ");
				print_process_line(proc, max_proc_name_width);
				lines += 1;
			}
		}
	}

	if show_all || (resolved_args.is_empty() && current_project.is_none()) {
		println!();
		lines += 1;
		if let Some(port) = http_port {
			println!(" {} {:<width$} http://127.0.0.1:{}", "●".green(), "serve", port, width = max_name_width);
		} else {
			println!(" {} {:<width$} not running", "○".dimmed(), "serve", width = max_name_width);
		}
		lines += 1;
	}

	lines
}

fn watch_status(args: &[String], opts: &WatchOpts) {
	let start = Instant::now();
	let mut prev_lines = 0usize;
	let stdout = io::stdout();

	loop {
		if prev_lines > 0 {
			print!("\x1b[{}A\x1b[J", prev_lines);
			let _ = stdout.lock().flush();
		}

		prev_lines = render_status(args);
		let _ = stdout.lock().flush();

		if let Some(duration) = opts.duration {
			if start.elapsed().as_secs() >= duration {
				return;
			}
		}

		std::thread::sleep(std::time::Duration::from_secs(opts.interval));
	}
}

// --- Formatting helpers ---

fn format_uptime(secs: u64) -> String {
	if secs < 60 {
		format!("{}s", secs)
	} else if secs < 3600 {
		let m = secs / 60;
		let s = secs % 60;
		if s == 0 { format!("{}m", m) } else { format!("{}m{}s", m, s) }
	} else if secs < 86400 {
		let h = secs / 3600;
		let m = (secs % 3600) / 60;
		if m == 0 { format!("{}h", h) } else { format!("{}h{}m", h, m) }
	} else {
		let d = secs / 86400;
		let h = (secs % 86400) / 3600;
		if h == 0 { format!("{}d", d) } else { format!("{}d{}h", d, h) }
	}
}

fn parse_dot_target(name: &str) -> (&str, Option<&str>) {
	if let Some(dot) = name.find('.') {
		(&name[..dot], Some(&name[dot + 1..]))
	} else {
		(name, None)
	}
}

fn resolve_dot_target(name: &str, entries: &BTreeMap<String, ServiceEntry>) -> (String, Option<String>) {
	let (svc, proc) = parse_dot_target(name);
	if svc.is_empty() {
		if let Some(current) = get_current_project(entries) {
			(current, proc.map(|s| s.to_string()))
		} else {
			eprintln!("not in a registered project directory; use service.process syntax");
			std::process::exit(1);
		}
	} else {
		(svc.to_string(), proc.map(|s| s.to_string()))
	}
}

// --- Target resolution ---

fn is_all_flag(s: &str) -> bool {
	matches!(s, "--all" | "-a" | "all")
}

fn get_current_project(entries: &BTreeMap<String, ServiceEntry>) -> Option<String> {
	if let Ok(cwd) = std::env::current_dir() {
		let cwd = cwd.canonicalize().unwrap_or(cwd);
		for (name, entry) in entries {
			let entry_dir = entry.dir.canonicalize().unwrap_or(entry.dir.clone());
			if cwd == entry_dir {
				return Some(name.clone());
			}
		}
	}
	None
}

fn resolve_target_names(args: &[String], entries: &BTreeMap<String, ServiceEntry>) -> Vec<String> {
	if args.is_empty() {
		if let Ok(cwd) = std::env::current_dir() {
			let cwd = cwd.canonicalize().unwrap_or(cwd);
			for (name, entry) in entries {
				let entry_dir = entry.dir.canonicalize().unwrap_or(entry.dir.clone());
				if cwd == entry_dir {
					return vec![name.clone()];
				}
			}
		}
		eprintln!("no service specified and not in a registered project directory");
		eprintln!("use --all to target all services, or specify a name");
		if !entries.is_empty() {
			let names: Vec<&str> = entries.keys().map(|s| s.as_str()).collect();
			eprintln!("registered: {}", names.join(", "));
		}
		std::process::exit(1);
	}

	if args.len() == 1 && is_all_flag(&args[0]) {
		return entries.keys().cloned().collect();
	}

	args.iter().filter(|a| !is_all_flag(a)).cloned().collect()
}

fn check_alias_hint() {
	let exe = std::env::current_exe().unwrap_or_default();
	let name = exe.file_name().unwrap_or_default().to_string_lossy();
	if name == "ubermind" {
		if let Some(dir) = exe.parent() {
			let ub = dir.join("ub");
			if !ub.exists() {
				eprintln!();
				eprintln!("tip: create a shorter alias:");
				eprintln!("ln -s {} {}", exe.display(), ub.display());
			}
		}
	}
}
