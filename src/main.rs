use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

const BIN: &str = "ubermind";

const OVERMIND_COMMANDS: &[&str] = &["kill", "echo", "restart", "connect", "quit", "run"];

// --- Service ---

struct Service {
    name: String,
    dir: PathBuf,
    command: Option<String>,
}

impl Service {
    fn socket_path(&self) -> PathBuf {
        self.dir.join(".overmind.sock")
    }

    fn is_running(&self) -> bool {
        self.socket_path().exists()
    }

    fn has_procfile(&self) -> bool {
        self.dir.join("Procfile").exists()
    }

    fn overmind(&self, args: &[&str]) -> std::io::Result<std::process::ExitStatus> {
        Command::new("overmind")
            .args(args)
            .current_dir(&self.dir)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
    }

    fn run(&self, args: &[&str]) -> bool {
        match self.overmind(args) {
            Ok(s) if s.success() => true,
            Ok(s) => {
                eprintln!("failed (exit {})", s.code().unwrap_or(-1));
                false
            }
            Err(e) => {
                eprintln!("error: {e}");
                false
            }
        }
    }

    fn run_quiet(&self, args: &[&str]) -> bool {
        let result = Command::new("overmind")
            .args(args)
            .current_dir(&self.dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        match result {
            Ok(s) => s.success(),
            Err(_) => false,
        }
    }
}

// --- Config ---

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

fn projects_config_path() -> PathBuf {
    config_dir().join("projects")
}

fn detect_overmind_asset() -> Option<(&'static str, &'static str)> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    let os_part = match os {
        "macos" => "macos",
        "linux" => "linux",
        _ => return None,
    };
    let arch_part = match arch {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        _ => return None,
    };
    Some((os_part, arch_part))
}

fn install_overmind() -> bool {
    let (os_part, arch_part) = match detect_overmind_asset() {
        Some(v) => v,
        None => {
            eprintln!("unsupported platform for auto-install");
            return false;
        }
    };

    // Resolve install directory: same dir as current exe, fallback to ~/.local/bin
    let install_dir = env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| home_dir().join(".local/bin"));

    let dest = install_dir.join("overmind");

    eprintln!("installing overmind to {}", dest.display());

    // Fetch latest tag from GitHub API
    let tag_output = Command::new("curl")
        .args([
            "-fsSL",
            "https://api.github.com/repos/DarthSim/overmind/releases/latest",
        ])
        .output();

    let tag = match tag_output {
        Ok(out) => {
            let body = String::from_utf8_lossy(&out.stdout);
            body.lines()
                .find(|l| l.contains("\"tag_name\""))
                .and_then(|l| {
                    let parts: Vec<&str> = l.split('"').collect();
                    // "tag_name": "v2.5.1" -> splits to [..., "tag_name", ": ", "v2.5.1", ...]
                    let idx = parts.iter().position(|&p| p == "tag_name")?;
                    Some(parts.get(idx + 2)?.to_string())
                })
                .unwrap_or_else(|| "v2.5.1".to_string())
        }
        Err(_) => "v2.5.1".to_string(),
    };

    let url = format!(
        "https://github.com/DarthSim/overmind/releases/download/{tag}/overmind-{tag}-{os_part}-{arch_part}.gz"
    );

    eprintln!("downloading {url}");

    // Download and decompress: curl | gunzip > dest
    let curl = Command::new("curl")
        .args(["-fsSL", &url])
        .stdout(Stdio::piped())
        .spawn();

    let curl = match curl {
        Ok(c) => c,
        Err(e) => {
            eprintln!("failed to run curl: {e}");
            return false;
        }
    };

    let gunzip = Command::new("gunzip")
        .arg("-c")
        .stdin(curl.stdout.unwrap())
        .stdout(Stdio::piped())
        .output();

    match gunzip {
        Ok(out) if out.status.success() && !out.stdout.is_empty() => {
            if let Err(e) = fs::write(&dest, &out.stdout) {
                eprintln!("failed to write {}: {e}", dest.display());
                return false;
            }

            // chmod +x
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&dest, fs::Permissions::from_mode(0o755));
            }

            eprintln!("installed overmind {tag} to {}", dest.display());
            true
        }
        Ok(out) => {
            eprintln!("gunzip failed: {}", String::from_utf8_lossy(&out.stderr));
            false
        }
        Err(e) => {
            eprintln!("failed to run gunzip: {e}");
            false
        }
    }
}

fn check_tmux() {
    if Command::new("tmux")
        .arg("-V")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
    {
        return;
    }

    eprintln!("tmux not found (required by overmind)");

    // Try to install via system package manager
    let installed = if cfg!(target_os = "macos") {
        eprintln!("installing tmux via brew...");
        Command::new("brew")
            .args(["install", "tmux"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    } else {
        // Linux: try apt, then yum/dnf, then apk
        if Command::new("apt-get")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
        {
            eprintln!("installing tmux via apt-get...");
            Command::new("sudo")
                .args(["apt-get", "install", "-y", "tmux"])
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        } else if Command::new("dnf")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
        {
            eprintln!("installing tmux via dnf...");
            Command::new("sudo")
                .args(["dnf", "install", "-y", "tmux"])
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        } else if Command::new("yum")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
        {
            eprintln!("installing tmux via yum...");
            Command::new("sudo")
                .args(["yum", "install", "-y", "tmux"])
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        } else if Command::new("apk")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
        {
            eprintln!("installing tmux via apk...");
            Command::new("apk")
                .args(["add", "tmux"])
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        } else {
            false
        }
    };

    if installed {
        if Command::new("tmux")
            .arg("-V")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
        {
            eprintln!("tmux installed successfully");
            return;
        }
        eprintln!("tmux was installed but still not found in PATH");
    }

    eprintln!("please install tmux manually:");
    eprintln!("  macOS:  brew install tmux");
    eprintln!("  Ubuntu: sudo apt install tmux");
    eprintln!("  Fedora: sudo dnf install tmux");
    std::process::exit(1);
}

fn check_overmind() {
    if Command::new("overmind")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
    {
        return;
    }

    eprintln!("overmind not found in PATH");

    if install_overmind() {
        // Verify it works now
        if Command::new("overmind")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
        {
            return;
        }
        eprintln!("overmind was installed but still not found in PATH");
    }

    eprintln!("install manually: https://github.com/DarthSim/overmind");
    std::process::exit(1);
}

fn load_projects() -> BTreeMap<String, Service> {
    let path = projects_config_path();
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
            eprintln!("bad config line (expected name: dir): {line}");
            continue;
        }

        let name = parts[0].trim().to_string();
        let dir_str = expand_tilde(parts[1].trim());
        let dir = PathBuf::from(&dir_str);

        if !dir.exists() {
            eprintln!("warning: dir does not exist for {name}: {dir_str}");
        }

        services.insert(
            name.clone(),
            Service {
                name,
                dir,
                command: None,
            },
        );
    }

    services
}

fn load_commands() -> BTreeMap<String, Service> {
    let path = config_dir().join("commands");
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

        if let Some((name, cmd)) = line.split_once(':') {
            let name = name.trim().to_string();
            let cmd = cmd.trim().to_string();
            let svc_dir = config_dir().join("_commands").join(&name);
            let _ = fs::create_dir_all(&svc_dir);
            let procfile = svc_dir.join("Procfile");
            let procfile_content = format!("{name}: {cmd}\n");
            if fs::read_to_string(&procfile).ok().as_deref() != Some(&procfile_content) {
                let _ = fs::write(&procfile, &procfile_content);
            }
            services.insert(
                name.clone(),
                Service {
                    name,
                    dir: svc_dir,
                    command: Some(cmd),
                },
            );
        }
    }

    services
}

fn load_services() -> BTreeMap<String, Service> {
    let mut services = load_projects();
    services.extend(load_commands());
    services
}

fn require_services() -> BTreeMap<String, Service> {
    check_tmux();
    check_overmind();
    let services = load_services();
    if services.is_empty() {
        let path = projects_config_path();
        if path.exists() {
            eprintln!("no projects configured");
            eprintln!();
            eprintln!("add a project:");
            eprintln!("  {BIN} add myapp ~/dev/myapp");
            eprintln!();
            eprintln!("or edit the config directly:");
            eprintln!("  {}", path.display());
            eprintln!();
            eprintln!("each project directory needs a Procfile with processes to run:");
            eprintln!("  web: npm run dev");
            eprintln!("  api: python server.py");
            eprintln!("  worker: ruby worker.rb");
        } else {
            eprintln!("no projects configured");
            eprintln!("run '{BIN} init' to get started");
        }
        eprintln!();
        eprintln!("docs: https://github.com/mrgnw/ubermind#quick-start");
        std::process::exit(1);
    }
    services
}

// --- Utilities ---

fn exit_code(failed: bool) -> ExitCode {
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn resolve_targets<'a>(
    services: &'a BTreeMap<String, Service>,
    name: Option<&str>,
) -> Option<Vec<&'a Service>> {
    match name {
        Some(n) => match services.get(n) {
            Some(svc) => Some(vec![svc]),
            None => {
                eprintln!("unknown project: {n}");
                eprintln!(
                    "available: {}",
                    services.keys().cloned().collect::<Vec<_>>().join(", ")
                );
                None
            }
        },
        None => Some(services.values().collect()),
    }
}

fn is_all_flag(s: &str) -> bool {
    matches!(s, "--all" | "-a" | "all")
}

fn resolve_targets_context_aware<'a>(
    services: &'a BTreeMap<String, Service>,
    name: Option<&str>,
) -> Option<Vec<&'a Service>> {
    match name {
        Some(n) if is_all_flag(n) => {
            let cwd = env::current_dir().ok();
            let mut targets: Vec<&Service> = Vec::with_capacity(services.len());
            let mut rest: Vec<&Service> = Vec::new();
            for svc in services.values() {
                if cwd.as_ref() == Some(&svc.dir) {
                    targets.insert(0, svc);
                } else {
                    rest.push(svc);
                }
            }
            targets.extend(rest);
            Some(targets)
        }
        Some(n) => match services.get(n) {
            Some(svc) => Some(vec![svc]),
            None => {
                eprintln!("unknown project: {n}");
                eprintln!(
                    "available: {}",
                    services.keys().cloned().collect::<Vec<_>>().join(", ")
                );
                None
            }
        },
        None => {
            let cwd = match env::current_dir() {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("error: failed to get current directory: {e}");
                    return None;
                }
            };

            for svc in services.values() {
                if cwd == svc.dir {
                    return Some(vec![svc]);
                }
            }

            eprintln!("error: not in a registered project directory");
            eprintln!();
            eprintln!("current directory: {}", cwd.display());
            eprintln!();
            eprintln!("to target all projects, run:");
            eprintln!("  {BIN} <command> --all");
            eprintln!();
            eprintln!("registered projects:");
            for (name, svc) in services {
                eprintln!("  {} -> {}", name, svc.dir.display());
            }
            None
        }
    }
}

fn get_process_status(svc: &Service, process_name: &str) -> Option<String> {
    let output = Command::new("overmind")
        .args(["status"])
        .current_dir(&svc.dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 && parts[0] == process_name {
            return Some(parts[1].to_string());
        }
    }
    None
}

fn await_process_status(
    svc: &Service,
    process_name: &str,
    want_running: bool,
    max_secs: u8,
) -> bool {
    for _ in 0..max_secs {
        std::thread::sleep(std::time::Duration::from_secs(1));
        eprint!(".");
        std::io::stderr().flush().ok();

        if let Some(status) = get_process_status(svc, process_name) {
            let is_running = status != "exited" && status != "stopped";
            if is_running == want_running {
                return true;
            }
        } else if !want_running {
            return true;
        }
    }
    false
}

fn await_socket_gone(socket_path: &PathBuf, max_secs: u8) -> bool {
    for _ in 0..max_secs {
        std::thread::sleep(std::time::Duration::from_secs(1));
        eprint!(".");
        std::io::stderr().flush().ok();

        if !socket_path.exists() {
            return true;
        }
    }
    false
}

fn await_socket_exists(socket_path: &PathBuf, max_secs: u8) -> bool {
    for _ in 0..max_secs {
        std::thread::sleep(std::time::Duration::from_secs(1));
        eprint!(".");
        std::io::stderr().flush().ok();

        if socket_path.exists() {
            return true;
        }
    }
    false
}

// --- Commands ---

fn cmd_init() -> ExitCode {
    let dir = config_dir();
    let path = dir.join("projects");
    if path.exists() {
        eprintln!("config already exists: {}", path.display());
        return ExitCode::SUCCESS;
    }
    if let Err(e) = fs::create_dir_all(&dir) {
        eprintln!("failed to create {}: {e}", dir.display());
        return ExitCode::FAILURE;
    }
    let content = "# name: dir\n# myapp: ~/dev/myapp\n";
    match fs::write(&path, content) {
        Ok(_) => {
            eprintln!("created {}", path.display());
            eprintln!();
            eprintln!("getting started:");
            eprintln!();
            eprintln!("  1. add a project that has a Procfile:");
            eprintln!("     {BIN} add myapp ~/dev/myapp");
            eprintln!();
            eprintln!("  2. if your project doesn't have a Procfile yet, create one:");
            eprintln!("     echo 'web: npm run dev' > ~/dev/myapp/Procfile");
            eprintln!();
            eprintln!("     a Procfile lists the processes to run (one per line):");
            eprintln!("       web: npm run dev");
            eprintln!("       api: python server.py");
            eprintln!("       worker: ruby worker.rb");
            eprintln!();
            eprintln!("  3. start your projects:");
            eprintln!("     {BIN} start");
            eprintln!();
            eprintln!("docs: https://github.com/mrgnw/ubermind#quick-start");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("failed to write {}: {e}", path.display());
            ExitCode::FAILURE
        }
    }
}

fn sanitize_service_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

fn cmd_add(name: Option<&str>, dir: Option<&str>) -> ExitCode {
    let path = projects_config_path();
    if !path.exists() {
        eprintln!("no config file found. run '{BIN} init' first");
        return ExitCode::FAILURE;
    }

    // Handle `uv add` with no arguments - use current directory
    let (name, dir) = match (name, dir) {
        (Some(n), Some(d)) => (n.to_string(), d.to_string()),
        (None, None) => {
            // Get current directory
            let cwd = match env::current_dir() {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("failed to get current directory: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let dir_name = match cwd.file_name() {
                Some(n) => n.to_string_lossy().to_string(),
                None => {
                    eprintln!("failed to get directory name");
                    return ExitCode::FAILURE;
                }
            };
            let sanitized_name = sanitize_service_name(&dir_name);
            let cwd_str = cwd.display().to_string();

            // Check if Procfile exists, if not create one
            let procfile_path = cwd.join("Procfile");
            if !procfile_path.exists() {
                eprintln!("no Procfile found in current directory");
                eprintln!("creating {}", procfile_path.display());
                let default_content =
                    format!("{sanitized_name}: echo 'configure your command in Procfile'\n");
                if let Err(e) = fs::write(&procfile_path, default_content) {
                    eprintln!("failed to create Procfile: {e}");
                    return ExitCode::FAILURE;
                }
                eprintln!("created Procfile - edit it to set your command");
            }

            (sanitized_name, cwd_str)
        }
        _ => {
            eprintln!("usage: {BIN} add [<name> <dir>]");
            eprintln!("  {BIN} add              add current directory");
            eprintln!("  {BIN} add <name> <dir> add specific directory");
            return ExitCode::FAILURE;
        }
    };

    let expanded = expand_tilde(&dir);
    if !Path::new(&expanded).exists() {
        eprintln!("warning: directory does not exist: {expanded}");
    }
    if !Path::new(&expanded).join("Procfile").exists() {
        eprintln!("warning: no Procfile in {expanded}");
    }

    let existing = fs::read_to_string(&path).unwrap_or_default();
    for line in existing.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let sep = if line.contains(':') { ':' } else { '\t' };
        if let Some(existing_name) = line.split(sep).next() {
            if existing_name.trim() == name {
                eprintln!("project '{name}' already exists in {}", path.display());
                return ExitCode::FAILURE;
            }
        }
    }

    let mut file = match fs::OpenOptions::new().append(true).open(&path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("failed to open {}: {e}", path.display());
            return ExitCode::FAILURE;
        }
    };

    if let Err(e) = writeln!(file, "{name}: {dir}") {
        eprintln!("failed to write: {e}");
        return ExitCode::FAILURE;
    }

    eprintln!("added {name} -> {dir}");
    ExitCode::SUCCESS
}

fn cmd_start(services: &BTreeMap<String, Service>, name: Option<&str>) -> ExitCode {
    let targets = match resolve_targets_context_aware(services, name) {
        Some(t) => t,
        None => return ExitCode::FAILURE,
    };

    let mut failed = false;
    for svc in targets {
        if svc.is_running() {
            eprintln!("{}: already running", svc.name);
            continue;
        }
        if !svc.has_procfile() {
            eprintln!("{}: no Procfile in {}", svc.name, svc.dir.display());
            failed = true;
            continue;
        }
        eprint!("{}: starting", svc.name);
        if !svc.run_quiet(&["start", "-D"]) {
            eprintln!(" failed");
            failed = true;
            continue;
        }
        if await_socket_exists(&svc.socket_path(), 5) {
            eprintln!(" running");
        } else {
            eprintln!(" failed");
            failed = true;
        }
    }

    exit_code(failed)
}

fn cmd_stop(services: &BTreeMap<String, Service>, name: Option<&str>) -> ExitCode {
    let targets = match resolve_targets_context_aware(services, name) {
        Some(t) => t,
        None => return ExitCode::FAILURE,
    };

    let mut failed = false;
    for svc in targets {
        if !svc.is_running() {
            eprintln!("{}: not running", svc.name);
            continue;
        }
        eprint!("{}: stopping", svc.name);
        if !svc.run(&["quit"]) {
            eprintln!(" failed");
            failed = true;
            continue;
        }
        if await_socket_gone(&svc.socket_path(), 5) {
            let _ = fs::remove_file(svc.socket_path());
            eprintln!(" stopped");
        } else {
            eprintln!(" failed");
            failed = true;
        }
    }

    exit_code(failed)
}

fn cmd_reload(services: &BTreeMap<String, Service>, name: Option<&str>) -> ExitCode {
    let targets = match resolve_targets_context_aware(services, name) {
        Some(t) => t,
        None => return ExitCode::FAILURE,
    };

    let mut failed = false;
    for svc in targets {
        eprint!("{}: reloading", svc.name);
        if svc.is_running() {
            let _ = svc.overmind(&["quit"]);
            std::thread::sleep(std::time::Duration::from_secs(1));
            let _ = fs::remove_file(svc.socket_path());
        }
        if !svc.has_procfile() {
            eprintln!(" no Procfile");
            failed = true;
            continue;
        }
        if !svc.run_quiet(&["start", "-D"]) {
            eprintln!(" failed");
            failed = true;
            continue;
        }
        if await_socket_exists(&svc.socket_path(), 5) {
            eprintln!(" running");
        } else {
            eprintln!(" failed");
            failed = true;
        }
    }

    exit_code(failed)
}

fn cmd_status(services: &BTreeMap<String, Service>, name: Option<&str>) -> ExitCode {
    let targets = match resolve_targets_context_aware(services, name) {
        Some(t) => t,
        None => return ExitCode::FAILURE,
    };

    const GREEN: &str = "\x1b[32m";
    const RED: &str = "\x1b[31m";
    const RESET: &str = "\x1b[0m";

    if name.is_some_and(|n| is_all_flag(n)) {
        match serve_running_pid() {
            Some(pid) => println!(" {}●{}\tubermind-serve\tPID {}", GREEN, RESET, pid),
            None => println!(" {}●{}\tubermind-serve\tstopped", RED, RESET),
        }
    }

    for svc in targets {
        let (circle, color) = if svc.is_running() {
            ("●", GREEN)
        } else {
            ("●", RED)
        };
        if let Some(cmd) = &svc.command {
            println!(" {}{}{}\t{}\t{}", color, circle, RESET, svc.name, cmd);
        } else {
            println!(
                " {}{}{}\t{}\t{}",
                color,
                circle,
                RESET,
                svc.name,
                svc.dir.display()
            );
        }
    }
    ExitCode::SUCCESS
}

const INTERACTIVE_COMMANDS: &[&str] = &["connect", "echo", "run"];

fn cmd_passthrough(svc: &Service, args: &[String]) -> ExitCode {
    if !svc.is_running() && !args.first().is_some_and(|a| a == "start") {
        eprintln!("{}: not running", svc.name);
        return ExitCode::FAILURE;
    }

    let cmd_name = args.first().map(|s| s.as_str()).unwrap_or("?");
    let quiet = INTERACTIVE_COMMANDS.contains(&cmd_name);
    if !quiet {
        eprint!("{}: {}", svc.name, cmd_name);
    }
    let str_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    let result = svc.overmind(&str_args);

    if quiet {
        return match result {
            Ok(s) => exit_code(!s.success()),
            Err(_) => ExitCode::FAILURE,
        };
    }

    match result {
        Ok(s) if !s.success() => {
            eprintln!(" failed (exit {})", s.code().unwrap_or(-1));
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!(" error: {e}");
            ExitCode::FAILURE
        }
        Ok(_) => {
            let verified = match cmd_name {
                "restart" => await_process_status(svc, &svc.name, true, 5),
                "quit" => await_socket_gone(&svc.socket_path(), 5),
                _ => {
                    eprintln!(" sent");
                    return ExitCode::SUCCESS;
                }
            };
            if verified {
                match cmd_name {
                    "restart" => eprintln!(" running"),
                    "quit" => eprintln!(" stopped"),
                    _ => eprintln!(" ok"),
                }
                ExitCode::SUCCESS
            } else {
                eprintln!(" failed");
                ExitCode::FAILURE
            }
        }
    }
}

fn cmd_passthrough_all(
    services: &BTreeMap<String, Service>,
    cmd: &str,
    name: Option<&str>,
    extra: &[String],
) -> ExitCode {
    let targets = match resolve_targets_context_aware(services, name) {
        Some(t) => t,
        None => return ExitCode::FAILURE,
    };

    let quiet = INTERACTIVE_COMMANDS.contains(&cmd);
    let mut failed = false;
    for svc in targets {
        if !svc.is_running() {
            eprintln!("{}: not running", svc.name);
            continue;
        }
        if !quiet {
            eprint!("{}: {}", svc.name, cmd);
        }
        let mut args = vec![cmd.to_string()];
        args.extend(extra.iter().cloned());
        let str_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let result = svc.overmind(&str_args);

        if quiet {
            match result {
                Ok(s) if !s.success() => failed = true,
                Err(_) => failed = true,
                _ => {}
            }
            continue;
        }

        match result {
            Ok(s) if !s.success() => {
                eprintln!(" failed (exit {})", s.code().unwrap_or(-1));
                failed = true;
            }
            Err(e) => {
                eprintln!(" error: {e}");
                failed = true;
            }
            Ok(_) => {
                let verified = match cmd {
                    "restart" => await_process_status(svc, &svc.name, true, 5),
                    "kill" => {
                        std::thread::sleep(std::time::Duration::from_secs(1));
                        eprint!(".");
                        true
                    }
                    "quit" => await_socket_gone(&svc.socket_path(), 5),
                    _ => {
                        eprintln!(" sent");
                        continue;
                    }
                };
                if verified {
                    match cmd {
                        "restart" => eprintln!(" running"),
                        "kill" => eprintln!(" killed"),
                        "quit" => eprintln!(" stopped"),
                        _ => eprintln!(" ok"),
                    }
                } else {
                    eprintln!(" failed");
                    failed = true;
                }
            }
        }
    }
    exit_code(failed)
}

fn strip_ansi(s: &str) -> String {
    let mut result = String::new();
    let mut in_escape = false;
    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if c.is_ascii_alphabetic() {
                in_escape = false;
            }
        } else {
            result.push(c);
        }
    }
    result
}

fn cmd_echo(
    services: &BTreeMap<String, Service>,
    name: Option<&str>,
    filters: &[String],
) -> ExitCode {
    let targets = match resolve_targets(services, name) {
        Some(t) => t,
        None => return ExitCode::FAILURE,
    };

    if filters.is_empty() {
        return cmd_passthrough_all(services, "echo", name, &[]);
    }

    let mut failed = false;

    for svc in targets {
        if !svc.is_running() {
            eprintln!("{}: not running", svc.name);
            continue;
        }

        let mut child = match Command::new("overmind")
            .args(["echo"])
            .current_dir(&svc.dir)
            .env("CLICOLOR_FORCE", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{}: failed to start echo: {}", svc.name, e);
                failed = true;
                continue;
            }
        };

        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(l) => {
                        let clean = strip_ansi(&l);
                        if let Some((prefix, _)) = clean.split_once(" | ") {
                            let prefix = prefix.trim();
                            if filters.iter().any(|f| prefix == f) {
                                println!("{}", l);
                            }
                        } else {
                            println!("{}", l);
                        }
                    }
                    Err(_) => break,
                }
            }
        }

        let _ = child.wait();
    }

    exit_code(failed)
}

fn serve_state_file() -> PathBuf {
    home_dir().join(".local/share/ubermind/serve-state")
}

fn serve_log_file() -> Option<PathBuf> {
    fs::read_to_string(serve_state_file())
        .ok()
        .and_then(|s| s.lines().next().map(|l| PathBuf::from(l)))
}

fn save_serve_state(log_path: &PathBuf) {
    let _ = fs::write(serve_state_file(), log_path.display().to_string());
}

fn serve_running_pid() -> Option<u32> {
    Command::new("lsof")
        .args(["-ti", ":13369", "-sTCP:LISTEN"])
        .output()
        .ok()
        .and_then(|out| {
            if out.stdout.is_empty() {
                None
            } else {
                let pid_str = String::from_utf8_lossy(&out.stdout);
                pid_str.trim().parse::<u32>().ok()
            }
        })
}

fn cmd_serve(args: &[String]) -> ExitCode {
    let serve_bin = "ubermind-serve";
    let daemon = args.iter().any(|a| a == "-d" || a == "--daemon");
    let stop = args.iter().any(|a| a == "--stop");
    let echo = args.iter().any(|a| a == "--echo");
    let restart = args.iter().any(|a| a == "--restart");
    let status = args.iter().any(|a| a == "--status");
    let extra: Vec<&str> = args
        .iter()
        .filter(|a| {
            !matches!(
                a.as_str(),
                "-d" | "--daemon" | "--stop" | "--echo" | "--restart" | "--status"
            )
        })
        .map(|s| s.as_str())
        .collect();

    if status {
        match serve_running_pid() {
            Some(pid) => {
                println!("ubermind-serve\trunning (PID {pid})");
                if let Some(log_file) = serve_log_file() {
                    println!("  {}", log_file.display());
                }
                ExitCode::SUCCESS
            }
            None => {
                println!("ubermind-serve\tnot running");
                ExitCode::FAILURE
            }
        }
    } else if echo {
        match serve_log_file() {
            Some(log_file) if log_file.exists() => {
                let status = Command::new("tail")
                    .args(["-f", log_file.to_str().unwrap()])
                    .stdin(Stdio::inherit())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .status();
                match status {
                    Ok(_) => ExitCode::SUCCESS,
                    Err(e) => {
                        eprintln!("failed to tail log: {e}");
                        ExitCode::FAILURE
                    }
                }
            }
            Some(log_file) => {
                eprintln!("log file not found: {}", log_file.display());
                ExitCode::FAILURE
            }
            None => {
                eprintln!("serve not running or no log file recorded");
                eprintln!("run 'ub serve -d' first");
                ExitCode::FAILURE
            }
        }
    } else if restart {
        eprint!("restarting ubermind-serve");
        let stop_result = serve_running_pid()
            .and_then(|pid| Command::new("kill").arg(pid.to_string()).status().ok());

        if stop_result.is_some() {
            eprint!(".");
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        let log_dir = home_dir().join(".local/share/ubermind/log");
        let _ = fs::create_dir_all(&log_dir);
        let log_file = log_dir.join(format!(
            "serve-{}.log",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        ));

        let log = match fs::File::create(&log_file) {
            Ok(f) => f,
            Err(e) => {
                eprintln!(" failed to create log file: {e}");
                return ExitCode::FAILURE;
            }
        };

        match Command::new(serve_bin)
            .args(&extra)
            .stdin(Stdio::null())
            .stdout(log.try_clone().unwrap())
            .stderr(log)
            .spawn()
        {
            Ok(_) => {
                save_serve_state(&log_file);
                eprintln!(" running");
                eprintln!("  {}", log_file.display());
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!(" failed to start {serve_bin}: {e}");
                ExitCode::FAILURE
            }
        }
    } else if stop {
        match serve_running_pid() {
            Some(pid) => match Command::new("kill").arg(pid.to_string()).status() {
                Ok(_) => {
                    eprintln!("stopped ubermind-serve (PID {pid})");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("failed to kill process: {e}");
                    ExitCode::FAILURE
                }
            },
            None => {
                eprintln!("ubermind-serve not running (port 13369 not in use)");
                ExitCode::FAILURE
            }
        }
    } else if daemon {
        let log_dir = home_dir().join(".local/share/ubermind/log");
        let _ = fs::create_dir_all(&log_dir);
        let log_file = log_dir.join(format!(
            "serve-{}.log",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        ));

        let log = match fs::File::create(&log_file) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("failed to create log file: {e}");
                return ExitCode::FAILURE;
            }
        };

        match Command::new(serve_bin)
            .args(&extra)
            .stdin(Stdio::null())
            .stdout(log.try_clone().unwrap())
            .stderr(log)
            .spawn()
        {
            Ok(_) => {
                save_serve_state(&log_file);
                eprintln!("ubermind-serve started in background");
                eprintln!("  {}", log_file.display());
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("failed to start {serve_bin}: {e}");
                ExitCode::FAILURE
            }
        }
    } else {
        match Command::new(serve_bin)
            .args(&extra)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
        {
            Ok(s) if s.success() => ExitCode::SUCCESS,
            Ok(s) => {
                eprintln!("{serve_bin} exited with code {}", s.code().unwrap_or(-1));
                ExitCode::FAILURE
            }
            Err(_) => {
                eprintln!("{serve_bin} not found in PATH");
                eprintln!("build it from ubermind/ui/src-tauri:");
                eprintln!("  cargo build --release --bin serve");
                ExitCode::FAILURE
            }
        }
    }
}

// --- CLI ---

fn print_usage() {
    let v = env!("CARGO_PKG_VERSION");
    eprintln!("{BIN} {v} - manage multiple overmind instances");
    eprintln!();
    eprintln!("usage:");
    eprintln!("  {BIN} status [name|--all]  show project status");
    eprintln!("  {BIN} start [name|--all]   start project(s)");
    eprintln!("  {BIN} stop [name|--all]    stop project(s)");
    eprintln!("  {BIN} reload [name|--all]  restart project(s) (picks up Procfile changes)");
    eprintln!("  {BIN} kill [name|--all]    kill process(es) in project(s)");
    eprintln!("  {BIN} restart [name|--all] restart process(es) in project(s)");
    eprintln!("  {BIN} echo [name|--all]    view logs from project(s)");
    eprintln!("  {BIN} connect [name]       connect to a process in a project");
    eprintln!("  {BIN} <name> <cmd...>      pass command to project's overmind");
    eprintln!("  {BIN} <cmd> <name>         pass command to project's overmind");
    eprintln!(
        "  {BIN} serve [-d] [-p PORT]  start web UI (use -d for daemon mode, default port: 13369)"
    );
    eprintln!("  {BIN} init                 create config file");
    eprintln!("  {BIN} add <name> <dir>     add a project");
    eprintln!();
    eprintln!("when no name is given, commands target the project in the current directory.");
    eprintln!("use --all to target every project.");
    eprintln!();
    eprintln!("examples:");
    eprintln!("  {BIN} status               show status of current project");
    eprintln!("  {BIN} status --all         show status of all projects");
    eprintln!("  {BIN} start                start project in current directory");
    eprintln!("  {BIN} start --all          start all projects");
    eprintln!("  {BIN} start myapp          start just myapp");
    eprintln!("  {BIN} stop --all           stop all projects");
    eprintln!("  {BIN} restart --all        restart processes in all projects");
    eprintln!("  {BIN} myapp connect web    attach to myapp's web process");
    eprintln!("  {BIN} connect web myapp    same thing, project name last");
    eprintln!();
    eprintln!("config:");
    eprintln!("  projects: {}", projects_config_path().display());
    eprintln!("  commands: {}", config_dir().join("commands").display());
    eprintln!();
    eprintln!("projects file defines directory-based projects (name: ~/path/to/project)");
    eprintln!("commands file defines standalone commands in Procfile format (name: command args)");
}

fn check_alias_hint() {
    let arg0 = match env::args().next() {
        Some(a) => PathBuf::from(a),
        None => return,
    };
    if arg0.file_name().map(|n| n == "ub").unwrap_or(false) {
        return;
    }
    let invoked_dir = if arg0.is_absolute() {
        arg0.parent().map(|p| p.to_path_buf())
    } else {
        env::var_os("PATH")
            .and_then(|paths| env::split_paths(&paths).find(|dir| dir.join(&arg0).exists()))
    };
    let dir = match invoked_dir {
        Some(d) => d,
        None => return,
    };
    if dir.join("ub").exists() {
        return;
    }
    let exe = dir.join("ubermind");
    eprintln!("tip: create a short alias with:");
    eprintln!("  ln -s {} {}", exe.display(), dir.join("ub").display());
    eprintln!();
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        print_usage();
        check_alias_hint();
        return ExitCode::SUCCESS;
    }

    match args[0].as_str() {
        "help" | "--help" | "-h" => {
            print_usage();
            ExitCode::SUCCESS
        }
        "version" | "--version" | "-V" => {
            println!("{BIN} {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        "init" => cmd_init(),
        "add" => cmd_add(
            args.get(1).map(|s| s.as_str()),
            args.get(2).map(|s| s.as_str()),
        ),
        "status" | "st" => {
            let services = require_services();
            let name = args.get(1).map(|s| s.as_str());
            if name.is_some_and(|n| !is_all_flag(n)) && args.len() > 2 {
                if let Some(svc) = services.get(name.unwrap()) {
                    let mut passthrough_args = vec!["status".to_string()];
                    passthrough_args.extend_from_slice(&args[2..]);
                    return cmd_passthrough(svc, &passthrough_args);
                }
            }
            cmd_status(&services, name)
        }
        "start" => {
            let s = require_services();
            cmd_start(&s, args.get(1).map(|s| s.as_str()))
        }
        "stop" => {
            let s = require_services();
            cmd_stop(&s, args.get(1).map(|s| s.as_str()))
        }
        "reload" => {
            let s = require_services();
            cmd_reload(&s, args.get(1).map(|s| s.as_str()))
        }
        "echo" => {
            let services = require_services();
            let (name, filters) = if let Some(svc_name) = args.get(1) {
                if services.contains_key(svc_name.as_str()) {
                    (Some(svc_name.as_str()), args[2..].to_vec())
                } else {
                    (None, args[1..].to_vec())
                }
            } else {
                (None, vec![])
            };
            cmd_echo(&services, name, &filters)
        }
        "serve" | "ui" => cmd_serve(&args[1..]),
        cmd if OVERMIND_COMMANDS.contains(&cmd) => {
            let services = require_services();
            let (name, extra) = if let Some(svc_name) = args.get(1) {
                if services.contains_key(svc_name.as_str()) {
                    (Some(svc_name.as_str()), args[2..].to_vec())
                } else {
                    (None, args[1..].to_vec())
                }
            } else {
                (None, vec![])
            };
            cmd_passthrough_all(&services, cmd, name, &extra)
        }
        name => {
            let services = require_services();
            if let Some(svc) = services.get(name) {
                if args.len() < 2 {
                    eprintln!("usage: {BIN} {name} <overmind-command...>");
                    eprintln!("example: {BIN} {name} status");
                    return ExitCode::FAILURE;
                }
                cmd_passthrough(svc, &args[1..])
            } else if let Some(svc) = args.last().and_then(|n| services.get(n.as_str())) {
                let cmd_args: Vec<String> = args[..args.len() - 1].to_vec();
                cmd_passthrough(svc, &cmd_args)
            } else {
                eprintln!("unknown command or project: {name}");
                eprintln!();
                print_usage();
                ExitCode::FAILURE
            }
        }
    }
}
