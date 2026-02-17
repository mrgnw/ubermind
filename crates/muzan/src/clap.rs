use clap::Subcommand;

use crate::paths::DaemonPaths;

#[derive(Debug, Subcommand)]
pub enum DaemonCommand {
	/// Run the daemon in the foreground (used internally)
	Run,
	/// Start the daemon in the background
	Start,
	/// Stop the running daemon
	Stop,
	/// Show daemon status
	Status,
}

impl DaemonCommand {
	pub fn execute(&self, paths: &DaemonPaths) {
		match self {
			DaemonCommand::Run => {
				eprintln!("muzan: use Daemon::run() to start the daemon server");
				eprintln!("the 'run' subcommand must be handled by your application");
			}
			DaemonCommand::Start => {
				if crate::client::is_running(paths) {
					eprintln!("daemon already running");
					return;
				}
				let daemon = crate::Daemon::new(paths.app_name.clone());
				match daemon.start_background() {
					Ok(_) => eprintln!("daemon started"),
					Err(e) => {
						eprintln!("error: {}", e);
						std::process::exit(1);
					}
				}
			}
			DaemonCommand::Stop => {
				let daemon = crate::Daemon::new(paths.app_name.clone());
				match daemon.stop() {
					Ok(_) => eprintln!("daemon stopped"),
					Err(e) => eprintln!("{}", e),
				}
			}
			DaemonCommand::Status => {
				if crate::client::is_running(paths) {
					if let Some(pid) = crate::client::read_pid(paths) {
						eprintln!("daemon running (pid {})", pid);
					} else {
						eprintln!("daemon running");
					}
				} else {
					eprintln!("daemon not running");
				}
			}
		}
	}
}
