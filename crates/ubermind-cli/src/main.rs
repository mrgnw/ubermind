use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::Command;
use ubermind_core::config::{self, ServiceEntry};
use ubermind_core::protocol::{self, Request, Response};
use ubermind_core::types::*;
use owo_colors::OwoColorize;

fn main() {
	let args: Vec<String> = std::env::args().skip(1).collect();

	if args.is_empty() {
		print_usage();
		check_alias_hint();
		return;
	}

	match args[0].as_str() {
		"help" | "--help" | "-h" => print_usage(),
		"version" | "--version" | "-V" => println!("ubermind {}", env!("CARGO_PKG_VERSION")),
		"init" => cmd_init(),
		"add" => cmd_add(&args[1..]),
		"status" | "st" => cmd_status(&args[1..]),
		"start" => cmd_start(&args[1..]),
		"stop" => cmd_stop(&args[1..]),
		"reload" => cmd_reload(&args[1..]),
		"restart" => cmd_restart(&args[1..]),
		"logs" => cmd_logs(&args[1..]),
		"echo" => cmd_echo(&args[1..]),
		"show" => cmd_show(&args[1..]),
		"daemon" => cmd_daemon(&args[1..]),
		"serve" => cmd_serve(&args[1..]),
		name => {
			// Flexible arg ordering: treat first arg as service name
			let services = config::load_service_entries();
			if services.contains_key(name) && args.len() > 1 {
				match args[1].as_str() {
					"start" => cmd_start(&[args[0].clone()]),
					"stop" => cmd_stop(&[args[0].clone()]),
					"reload" => cmd_reload(&[args[0].clone()]),
					"status" | "st" => cmd_status(&[args[0].clone()]),
					"logs" => cmd_logs(&args),
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
			} else if services.contains_key(name) {
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
	eprintln!("ubermind {} — process daemon manager", env!("CARGO_PKG_VERSION"));
	eprintln!();
	eprintln!("usage: ub [command] [service] [options]");
	eprintln!();
	eprintln!("commands:");
	eprintln!("  status [name|--all]          Show service status");
	eprintln!("  start [name|--all]           Start service(s)");
	eprintln!("  stop [name|--all]            Stop service(s)");
	eprintln!("  reload [name|--all]          Reload service(s) (stop + start)");
	eprintln!("  restart [name] [process]     Restart a process within a service");
	eprintln!("  logs <name> [process]        Tail log files");
	eprintln!("  echo <name> [process]        Live output stream");
	eprintln!("  show [name] [process]        Show Procfile or process command");
	eprintln!("  add [name] [dir]             Register a project");
	eprintln!("  init                         Create config files");
	eprintln!("  daemon [start|stop|status]   Manage the daemon");
	eprintln!("  serve [-d|--stop|--status]   Manage HTTP server for UI");
	eprintln!();
	eprintln!("context-aware: run from a project directory to auto-target it");
	eprintln!();
	eprintln!("examples:");
	eprintln!("  ub restart api               (from within project) restart 'api' process");
	eprintln!("  ub restart appligator api    (from anywhere) restart appligator's 'api' process");
	eprintln!("  ub show                      (from within project) show Procfile");
	eprintln!("  ub show api                  (from within project) show 'api' command");
	eprintln!("  ub show appligator           show appligator's Procfile");
	eprintln!("  ub show appligator api       show appligator's 'api' command");
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

	// Check for duplicate
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

	// Check for Procfile
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

	// Auto-start daemon
	eprintln!("starting daemon...");
	let daemon_bin = find_daemon_binary();

	let mut cmd = Command::new(&daemon_bin);
	cmd.stdout(std::process::Stdio::null())
		.stderr(std::process::Stdio::null());

	match cmd.spawn() {
		Ok(_) => {}
		Err(e) => {
			eprintln!("error: failed to start daemon: {}", e);
			eprintln!("binary: {}", daemon_bin.display());
			std::process::exit(1);
		}
	}

	// Wait for socket
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
	// Look next to the CLI binary first
	if let Ok(exe) = std::env::current_exe() {
		if let Some(dir) = exe.parent() {
			let daemon = dir.join("ubermind-daemon");
			if daemon.exists() {
				return daemon;
			}
		}
	}
	// Fall back to PATH
	PathBuf::from("ubermind-daemon")
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
	let response = send_request(&Request::Status);
	let services = match response {
		Response::Status { services } => services,
		Response::Error { message } => {
			eprintln!("error: {}", message);
			std::process::exit(1);
		}
		_ => {
			eprintln!("unexpected response from daemon");
			std::process::exit(1);
		}
	};

	let entries = config::load_service_entries();
	
	// Determine if we should show all or just current project
	let show_all = !args.is_empty() && (args.len() == 1 && is_all_flag(&args[0]) || args.iter().any(|a| is_all_flag(a)));
	let current_project = get_current_project(&entries);
	
	let filter = if args.is_empty() {
		// No args: show only current project if in one, otherwise show all
		if let Some(ref current) = current_project {
			vec![current.clone()]
		} else {
			entries.keys().cloned().collect()
		}
	} else if show_all {
		entries.keys().cloned().collect()
	} else {
		resolve_target_names(args, &entries)
	};

	let mut status_map: std::collections::HashMap<String, &ServiceStatus> =
		std::collections::HashMap::new();
	for s in &services {
		status_map.insert(s.name.clone(), s);
	}

	// Sort filter so current project comes first
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

	// Calculate column widths for alignment
	let max_name_width = sorted_filter.iter().map(|n| n.len()).max().unwrap_or(0);
	let max_proc_name_width = sorted_filter
		.iter()
		.filter_map(|name| status_map.get(name))
		.flat_map(|s| s.processes.iter().map(|p| p.name.len()))
		.max()
		.unwrap_or(0);

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

		if let Some(status) = status {
			for proc in &status.processes {
				let (circle, uptime, pid) = match &proc.state {
					ProcessState::Running { pid, uptime_secs } => {
						("●".green().to_string(), format!("{}s", uptime_secs), format!("{}", pid))
					}
					ProcessState::Stopped => ("●".red().to_string(), "stopped".to_string(), "-".to_string()),
					ProcessState::Crashed { exit_code, retries } => {
						("●".red().to_string(), format!("crashed (exit {}, retry {})", exit_code, retries), "-".to_string())
					}
					ProcessState::Failed { exit_code } => {
						("●".red().to_string(), format!("failed (exit {})", exit_code), "-".to_string())
					}
				};
				println!("   └ {} {:<pwidth$} {:<8} {}", circle, proc.name, uptime, pid, pwidth = max_proc_name_width);
			}
		}
	}
}

fn cmd_start(args: &[String]) {
	let entries = config::load_service_entries();
	let names = resolve_target_names(args, &entries);

	if names.is_empty() {
		eprintln!("no services to start");
		std::process::exit(1);
	}

	let response = send_request(&Request::Start { names: names.clone() });
	match response {
		Response::Ok { message } => {
			if let Some(msg) = message {
				for line in msg.lines() {
					eprintln!("{}", line);
				}
			}
			std::thread::sleep(std::time::Duration::from_millis(500));
			cmd_status(&names);
		}
		Response::Error { message } => {
			eprintln!("error: {}", message);
			std::process::exit(1);
		}
		_ => {}
	}
}

fn cmd_stop(args: &[String]) {
	let entries = config::load_service_entries();
	let names = resolve_target_names(args, &entries);

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
		}
		Response::Error { message } => {
			eprintln!("error: {}", message);
			std::process::exit(1);
		}
		_ => {}
	}
}

fn cmd_reload(args: &[String]) {
	let entries = config::load_service_entries();
	let names = resolve_target_names(args, &entries);

	if names.is_empty() {
		eprintln!("no services to reload");
		std::process::exit(1);
	}

	let response = send_request(&Request::Reload { names: names.clone() });
	match response {
		Response::Ok { message } => {
			if let Some(msg) = message {
				for line in msg.lines() {
					eprintln!("{}", line);
				}
			}
			std::thread::sleep(std::time::Duration::from_millis(500));
			cmd_status(&names);
		}
		Response::Error { message } => {
			eprintln!("error: {}", message);
			std::process::exit(1);
		}
		_ => {}
	}
}

fn cmd_restart(args: &[String]) {
	let entries = config::load_service_entries();
	
	// Context-aware: if no args or first arg is not a service, check if we're in a project
	let (service, process) = if args.is_empty() {
		// No args - reload current service
		if let Some(current) = get_current_project(&entries) {
			return cmd_reload(&[current]);
		} else {
			eprintln!("usage: ub restart <service> [process]");
			eprintln!("or run from a registered project directory");
			std::process::exit(1);
		}
	} else if args.len() == 1 {
		// One arg - could be service name or process name in current service
		if entries.contains_key(&args[0]) {
			// It's a service name - reload it
			return cmd_reload(&[args[0].clone()]);
		} else if let Some(current) = get_current_project(&entries) {
			// Treat arg as process name in current service
			(current, Some(args[0].clone()))
		} else {
			eprintln!("unknown service: {}", args[0]);
			eprintln!("registered services: {}", entries.keys().cloned().collect::<Vec<_>>().join(", "));
			std::process::exit(1);
		}
	} else {
		// Two or more args - first is service, second is process
		(args[0].clone(), Some(args[1].clone()))
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
			}
			Response::Error { message } => {
				eprintln!("error: {}", message);
				std::process::exit(1);
			}
			_ => {}
		}
	} else {
		cmd_reload(&[service.clone()]);
	}
}

fn cmd_logs(args: &[String]) {
	if args.is_empty() {
		eprintln!("usage: ub logs <service> [process]");
		std::process::exit(1);
	}

	let service = &args[0];
	let process = args.get(1).map(|s| s.as_str());

	let log_dir = ubermind_core::logs::service_log_dir(service);
	if !log_dir.exists() {
		eprintln!("no logs for {}", service);
		std::process::exit(1);
	}

	let mut files: Vec<PathBuf> = Vec::new();
	if let Ok(entries) = std::fs::read_dir(&log_dir) {
		for entry in entries.flatten() {
			let path = entry.path();
			let name = path
				.file_name()
				.unwrap_or_default()
				.to_string_lossy()
				.to_string();
			if !name.ends_with(".log") {
				continue;
			}
			if let Some(proc_filter) = process {
				if !name.starts_with(proc_filter) {
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

fn cmd_echo(args: &[String]) {
	if args.is_empty() {
		eprintln!("usage: ub echo <service> [process]");
		std::process::exit(1);
	}

	let service = &args[0];
	let process = args.get(1).cloned();

	let response = send_request(&Request::Logs {
		service: service.clone(),
		process,
		follow: true,
	});

	match response {
		Response::Log { line } => print!("{}", line),
		Response::Error { message } => {
			eprintln!("error: {}", message);
			std::process::exit(1);
		}
		_ => {}
	}
}

fn cmd_show(args: &[String]) {
	let entries = config::load_service_entries();
	
	// Handle "ub appligator show api" - skip "show" if it's in args[1]
	let filtered_args: Vec<String> = if args.len() >= 2 && args[1] == "show" {
		// Skip the "show" command: ["appligator", "show", "api"] -> ["appligator", "api"]
		let mut new_args = vec![args[0].clone()];
		new_args.extend_from_slice(&args[2..]);
		new_args
	} else {
		args.to_vec()
	};
	
	// Context-aware: if no args, show current service's Procfile
	let (service_name, process_name) = if filtered_args.is_empty() {
		if let Some(current) = get_current_project(&entries) {
			(current, None)
		} else {
			eprintln!("usage: ub show [service] [process]");
			eprintln!("or run from a registered project directory");
			std::process::exit(1);
		}
	} else if filtered_args.len() == 1 {
		// One arg - could be service name or process name in current service
		if entries.contains_key(&filtered_args[0]) {
			// It's a service name - show its Procfile
			(filtered_args[0].clone(), None)
		} else if let Some(current) = get_current_project(&entries) {
			// Treat arg as process name in current service
			(current, Some(filtered_args[0].clone()))
		} else {
			eprintln!("unknown service: {}", filtered_args[0]);
			eprintln!("registered services: {}", entries.keys().cloned().collect::<Vec<_>>().join(", "));
			std::process::exit(1);
		}
	} else {
		// Two or more args - first is service, second is process
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
		// Show specific process command
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
		// Show entire Procfile with syntax highlighting
		println!("{}", procfile_path.display().dimmed());
		println!();
		for line in content.lines() {
			let line_trimmed = line.trim();
			if line_trimmed.is_empty() {
				println!();
			} else if line_trimmed.starts_with('#') {
				// Comments in dim
				println!("{}", line.dimmed());
			} else if let Some((name, cmd)) = line.split_once(':') {
				// Process name in cyan, command in default color
				println!("{}:{}", name.cyan(), cmd);
			} else {
				// Fallback for malformed lines
				println!("{}", line);
			}
		}
	}
}

fn cmd_daemon(args: &[String]) {
	let subcmd = args.first().map(|s| s.as_str()).unwrap_or("status");

	match subcmd {
		"start" => {
			if connect_daemon().is_some() {
				eprintln!("daemon already running");
				return;
			}
			let extra_args: Vec<&str> = args[1..].iter().map(|s| s.as_str()).collect();
			let daemon_bin = find_daemon_binary();
			let mut cmd = Command::new(&daemon_bin);
			cmd.args(&extra_args)
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
			eprintln!("usage: ub daemon [start|stop|status]");
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
		// Foreground
		let daemon_bin = find_daemon_binary();
		let mut cmd = Command::new(&daemon_bin);
		cmd.args(["--foreground", "--http"]);
		let status = cmd.status().unwrap_or_else(|e| {
			eprintln!("error: {}", e);
			std::process::exit(1);
		});
		std::process::exit(status.code().unwrap_or(1));
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
		// Context-aware: check if cwd matches a registered service
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
