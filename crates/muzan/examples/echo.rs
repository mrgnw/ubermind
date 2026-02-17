use muzan::{Daemon, DaemonClient, DaemonPaths};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
enum Req {
	Ping,
	Echo(String),
	Stop,
}

#[derive(Serialize, Deserialize, Debug)]
enum Resp {
	Pong,
	Echo(String),
	Ok,
}

const APP: &str = "muzan-echo-example";

#[tokio::main]
async fn main() {
	let args: Vec<String> = std::env::args().collect();
	let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("help");

	match cmd {
		"run" => {
			let daemon = Daemon::new(APP);
			daemon
				.run(|req: Req| async move {
					match req {
						Req::Ping => Resp::Pong,
						Req::Echo(s) => Resp::Echo(s),
						Req::Stop => {
							eprintln!("stop requested, exiting...");
							std::process::exit(0);
						}
					}
				})
				.await;
		}
		"start" => {
			let daemon = Daemon::new(APP);
			match daemon.start_background() {
				Ok(()) => eprintln!("daemon started"),
				Err(e) => eprintln!("error: {e}"),
			}
		}
		"stop" => {
			let daemon = Daemon::new(APP);
			match daemon.stop() {
				Ok(()) => eprintln!("daemon stopped"),
				Err(e) => eprintln!("{e}"),
			}
		}
		"ping" => {
			let paths = DaemonPaths::new(APP);
			let mut client = DaemonClient::<Req, Resp>::connect(&paths).expect("daemon not running");
			let resp = client.send(&Req::Ping).unwrap();
			println!("{resp:?}");
		}
		"echo" => {
			let msg = args.get(2).cloned().unwrap_or("hello".into());
			let paths = DaemonPaths::new(APP);
			let mut client = DaemonClient::<Req, Resp>::connect(&paths).expect("daemon not running");
			let resp = client.send(&Req::Echo(msg)).unwrap();
			println!("{resp:?}");
		}
		_ => {
			eprintln!("usage: echo <run|start|stop|ping|echo [msg]>");
		}
	}
}
