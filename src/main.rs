use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

const BIN: &str = "ubermind";

// --- Service ---

struct Service {
    name: String,
    dir: PathBuf,
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

fn config_path() -> PathBuf {
    let primary = config_dir().join("services.tsv");
    if primary.exists() {
        return primary;
    }
    let legacy_dm = home_dir().join(".config/dm/services.tsv");
    if legacy_dm.exists() {
        return legacy_dm;
    }
    let legacy_daemons = home_dir().join("dev/_daemons/services.tsv");
    if legacy_daemons.exists() {
        return legacy_daemons;
    }
    primary
}

fn check_overmind() {
    if Command::new("overmind")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_err()
    {
        eprintln!("overmind not found in PATH");
        eprintln!("install: https://github.com/DarthSim/overmind");
        std::process::exit(1);
    }
}

fn load_services() -> BTreeMap<String, Service> {
    let path = config_path();
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("no services configured");
            eprintln!(
                "run '{BIN} init' to create {}",
                config_dir().join("services.tsv").display()
            );
            std::process::exit(1);
        }
    };

    let mut services = BTreeMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.splitn(2, '\t').collect();
        if parts.len() != 2 {
            eprintln!("bad config line (expected name\\tdir): {line}");
            continue;
        }

        let name = parts[0].trim().to_string();
        let dir_str = expand_tilde(parts[1].trim());
        let dir = PathBuf::from(&dir_str);

        if !dir.exists() {
            eprintln!("warning: dir does not exist for {name}: {dir_str}");
        }

        services.insert(name.clone(), Service { name, dir });
    }

    services
}

fn require_services() -> BTreeMap<String, Service> {
    check_overmind();
    load_services()
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
                eprintln!("unknown service: {n}");
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

// --- Commands ---

fn cmd_init() -> ExitCode {
    let dir = config_dir();
    let path = dir.join("services.tsv");
    if path.exists() {
        eprintln!("config already exists: {}", path.display());
        return ExitCode::SUCCESS;
    }
    if let Err(e) = fs::create_dir_all(&dir) {
        eprintln!("failed to create {}: {e}", dir.display());
        return ExitCode::FAILURE;
    }
    let content = "# name\tdir\n# myapp\t~/dev/myapp\n";
    match fs::write(&path, content) {
        Ok(_) => {
            eprintln!("created {}", path.display());
            eprintln!("add services with '{BIN} add <name> <dir>' or edit the file directly");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("failed to write {}: {e}", path.display());
            ExitCode::FAILURE
        }
    }
}

fn cmd_add(name: &str, dir: &str) -> ExitCode {
    let path = config_path();
    if !path.exists() {
        eprintln!("no config file found. run '{BIN} init' first");
        return ExitCode::FAILURE;
    }
    let expanded = expand_tilde(dir);
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
        if let Some(existing_name) = line.split('\t').next() {
            if existing_name.trim() == name {
                eprintln!("service '{name}' already exists in {}", path.display());
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

    if let Err(e) = writeln!(file, "{name}\t{dir}") {
        eprintln!("failed to write: {e}");
        return ExitCode::FAILURE;
    }

    eprintln!("added {name} -> {dir}");
    ExitCode::SUCCESS
}

fn cmd_start(services: &BTreeMap<String, Service>, name: Option<&str>) -> ExitCode {
    let targets = match resolve_targets(services, name) {
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
        eprint!("{}: starting... ", svc.name);
        if svc.run(&["start", "-D"]) {
            eprintln!("ok");
        } else {
            failed = true;
        }
    }

    exit_code(failed)
}

fn cmd_stop(services: &BTreeMap<String, Service>, name: Option<&str>) -> ExitCode {
    let targets = match resolve_targets(services, name) {
        Some(t) => t,
        None => return ExitCode::FAILURE,
    };

    let mut failed = false;
    for svc in targets {
        if !svc.is_running() {
            eprintln!("{}: not running", svc.name);
            continue;
        }
        eprint!("{}: stopping... ", svc.name);
        if svc.run(&["quit"]) {
            let _ = fs::remove_file(svc.socket_path());
            eprintln!("ok");
        } else {
            failed = true;
        }
    }

    exit_code(failed)
}

fn cmd_reload(services: &BTreeMap<String, Service>, name: Option<&str>) -> ExitCode {
    let targets = match resolve_targets(services, name) {
        Some(t) => t,
        None => return ExitCode::FAILURE,
    };

    let mut failed = false;
    for svc in targets {
        eprint!("{}: reloading... ", svc.name);
        if svc.is_running() {
            let _ = svc.overmind(&["quit"]);
            std::thread::sleep(std::time::Duration::from_secs(1));
            let _ = fs::remove_file(svc.socket_path());
        }
        if !svc.has_procfile() {
            eprintln!("no Procfile in {}", svc.dir.display());
            failed = true;
            continue;
        }
        if svc.run(&["start", "-D"]) {
            eprintln!("ok");
        } else {
            failed = true;
        }
    }

    exit_code(failed)
}

fn cmd_status(services: &BTreeMap<String, Service>) -> ExitCode {
    for svc in services.values() {
        let state = if svc.is_running() {
            "running"
        } else {
            "stopped"
        };
        println!("{}\t{}\t{}", svc.name, state, svc.dir.display());
    }
    ExitCode::SUCCESS
}

fn cmd_passthrough(svc: &Service, args: &[String]) -> ExitCode {
    if !svc.is_running() && !args.first().is_some_and(|a| a == "start") {
        eprintln!("{}: not running", svc.name);
        return ExitCode::FAILURE;
    }

    let str_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    match svc.overmind(&str_args) {
        Ok(s) => exit_code(!s.success()),
        Err(e) => {
            eprintln!("overmind error: {e}");
            ExitCode::FAILURE
        }
    }
}

// --- CLI ---

fn print_usage() {
    let v = env!("CARGO_PKG_VERSION");
    eprintln!("{BIN} {v} - manage multiple overmind instances");
    eprintln!();
    eprintln!("usage:");
    eprintln!("  {BIN} status              show all services");
    eprintln!("  {BIN} start [name]        start service(s)");
    eprintln!("  {BIN} stop [name]         stop service(s)");
    eprintln!("  {BIN} reload [name]       restart service(s) (picks up Procfile changes)");
    eprintln!("  {BIN} <name> <cmd...>     pass command to project's overmind");
    eprintln!("  {BIN} <cmd> <name>        pass command to project's overmind");
    eprintln!("  {BIN} init                create config file");
    eprintln!("  {BIN} add <name> <dir>    add a service");
    eprintln!();
    eprintln!("examples:");
    eprintln!("  {BIN} start               start all services");
    eprintln!("  {BIN} start myapp         start just myapp");
    eprintln!("  {BIN} status myapp        show myapp's overmind process status");
    eprintln!("  {BIN} echo myapp          view myapp's logs");
    eprintln!("  {BIN} myapp connect web   attach to myapp's web process");
    eprintln!("  {BIN} connect web myapp   same thing, project name last");
    eprintln!();
    eprintln!("config: {}", config_path().display());
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        print_usage();
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
        "add" => match (args.get(1), args.get(2)) {
            (Some(name), Some(dir)) => cmd_add(name, dir),
            _ => {
                eprintln!("usage: {BIN} add <name> <dir>");
                ExitCode::FAILURE
            }
        },
        "status" | "st" => {
            let services = require_services();
            if let Some(svc) = args.get(1).and_then(|n| services.get(n.as_str())) {
                let mut passthrough_args = vec!["status".to_string()];
                passthrough_args.extend_from_slice(&args[2..]);
                cmd_passthrough(svc, &passthrough_args)
            } else {
                cmd_status(&services)
            }
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
                eprintln!("unknown command or service: {name}");
                eprintln!();
                print_usage();
                ExitCode::FAILURE
            }
        }
    }
}
