use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

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

fn config_path() -> PathBuf {
    let home = env::var("HOME").expect("HOME not set");
    let home = Path::new(&home);
    let xdg = home.join(".config/dm/services.tsv");
    if xdg.exists() {
        return xdg;
    }
    let legacy = home.join("dev/_daemons/services.tsv");
    if legacy.exists() {
        return legacy;
    }
    xdg
}

fn load_services() -> BTreeMap<String, Service> {
    let path = config_path();
    let content = fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("failed to read {}: {e}", path.display());
        std::process::exit(1);
    });

    let home = env::var("HOME").expect("HOME not set");
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
        let raw_dir = parts[1].trim();
        let dir_str = if let Some(rest) = raw_dir.strip_prefix("~/") {
            format!("{home}/{rest}")
        } else {
            raw_dir.to_string()
        };
        let dir = PathBuf::from(&dir_str);

        if !dir.exists() {
            eprintln!("warning: dir does not exist for {name}: {dir_str}");
        }

        services.insert(name.clone(), Service { name, dir });
    }

    services
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
        if !svc.dir.join("Procfile").exists() {
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
        Ok(status) => {
            if status.success() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(e) => {
            eprintln!("overmind error: {e}");
            ExitCode::FAILURE
        }
    }
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
        if !svc.dir.join("Procfile").exists() {
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

fn print_usage() {
    eprintln!("dm - daemon manager for overmind projects");
    eprintln!();
    eprintln!("usage:");
    eprintln!("  dm status              show all services");
    eprintln!("  dm start [name]        start service(s)");
    eprintln!("  dm stop [name]         stop service(s)");
    eprintln!("  dm reload [name]       restart service(s) (picks up Procfile changes)");
    eprintln!("  dm <name> <cmd...>     pass command to project's overmind");
    eprintln!("  dm <cmd> <name>        pass command to project's overmind");
    eprintln!();
    eprintln!("examples:");
    eprintln!("  dm start               start all services");
    eprintln!("  dm start anani         start just anani");
    eprintln!("  dm status anani        show anani's overmind process status");
    eprintln!("  dm echo anani          view anani's logs");
    eprintln!("  dm anani connect dev   attach to anani's dev process");
    eprintln!("  dm connect dev anani   same thing, project name last");
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        print_usage();
        return ExitCode::SUCCESS;
    }

    let services = load_services();

    match args[0].as_str() {
        "help" | "--help" | "-h" => {
            print_usage();
            ExitCode::SUCCESS
        }
        "status" | "st" => {
            if let Some(svc) = args.get(1).and_then(|n| services.get(n.as_str())) {
                let mut passthrough_args = vec!["status".to_string()];
                passthrough_args.extend_from_slice(&args[2..]);
                cmd_passthrough(svc, &passthrough_args)
            } else {
                cmd_status(&services)
            }
        }
        "start" => cmd_start(&services, args.get(1).map(|s| s.as_str())),
        "stop" => cmd_stop(&services, args.get(1).map(|s| s.as_str())),
        "reload" => cmd_reload(&services, args.get(1).map(|s| s.as_str())),
        name => {
            if let Some(svc) = services.get(name) {
                if args.len() < 2 {
                    eprintln!("usage: dm {name} <overmind-command...>");
                    eprintln!("example: dm {name} status");
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
