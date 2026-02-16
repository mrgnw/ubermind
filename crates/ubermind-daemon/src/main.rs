mod api;
mod output;
mod supervisor;

use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use ubermind_core::config;
use ubermind_core::protocol::{self, Request, Response};

#[tokio::main]
async fn main() {
	tracing_subscriber::fmt().init();

	let args: Vec<String> = std::env::args().skip(1).collect();
	let _foreground = args.iter().any(|a| a == "--foreground" || a == "-f");
	let enable_http = args.iter().any(|a| a == "--http");

	let global_config = config::load_global_config();
	let port = global_config.daemon.port;
	let supervisor = supervisor::Supervisor::new(global_config.clone());

	// Ensure state directory exists
	let state_dir = protocol::state_dir();
	let _ = std::fs::create_dir_all(&state_dir);

	// Write PID file
	let pid_path = protocol::pid_path();
	let _ = std::fs::write(&pid_path, std::process::id().to_string());

	// Clean up stale socket
	let socket_path = protocol::socket_path();
	if socket_path.exists() {
		let _ = std::fs::remove_file(&socket_path);
	}

	// Run initial log expiry
	output::expire_logs(global_config.logs.max_age_days, global_config.logs.max_files);

	// Spawn log expiry task (hourly)
	{
		let config = global_config.clone();
		tokio::spawn(async move {
			loop {
				tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
				output::expire_logs(config.logs.max_age_days, config.logs.max_files);
			}
		});
	}

	// Start Unix socket server
	let sup_socket = Arc::clone(&supervisor);
	let socket_handle = tokio::spawn(async move {
		run_socket_server(sup_socket, &socket_path).await;
	});

	// Start HTTP server if requested
	let http_handle = if enable_http {
		let sup_http = Arc::clone(&supervisor);
		Some(tokio::spawn(async move {
			run_http_server(sup_http, port).await;
		}))
	} else {
		None
	};

	tracing::info!("daemon started (pid {})", std::process::id());
	if enable_http {
		tracing::info!("HTTP server on port {}", port);
	}

	// Wait for shutdown
	tokio::select! {
		_ = socket_handle => {},
		_ = async {
			if let Some(h) = http_handle { h.await.ok(); }
			else { std::future::pending::<()>().await; }
		} => {},
		_ = tokio::signal::ctrl_c() => {
			tracing::info!("shutting down");
		}
	}

	// Cleanup
	let _ = std::fs::remove_file(protocol::socket_path());
	let _ = std::fs::remove_file(protocol::pid_path());
}

async fn run_socket_server(supervisor: Arc<supervisor::Supervisor>, socket_path: &std::path::Path) {
	let listener = match UnixListener::bind(socket_path) {
		Ok(l) => l,
		Err(e) => {
			tracing::error!("failed to bind socket: {}", e);
			return;
		}
	};

	tracing::info!("listening on {}", socket_path.display());

	loop {
		let (stream, _) = match listener.accept().await {
			Ok(s) => s,
			Err(e) => {
				tracing::error!("accept error: {}", e);
				continue;
			}
		};

		let sup = Arc::clone(&supervisor);
		tokio::spawn(async move {
			let (reader, mut writer) = stream.into_split();
			let mut lines = BufReader::new(reader).lines();

			while let Ok(Some(line)) = lines.next_line().await {
				let request: Request = match serde_json::from_str(&line) {
					Ok(r) => r,
					Err(e) => {
						let resp = Response::Error {
							message: format!("invalid request: {}", e),
						};
						let _ = write_response(&mut writer, &resp).await;
						continue;
					}
				};

				let response = handle_request(&sup, request).await;
				if write_response(&mut writer, &response).await.is_err() {
					break;
				}
			}
		});
	}
}

async fn handle_request(supervisor: &Arc<supervisor::Supervisor>, request: Request) -> Response {
	match request {
		Request::Ping => Response::Pong,
		Request::Status => {
			let services = supervisor.status().await;
			Response::Status { services }
		}
		Request::Start { names, all, processes } => {
			let mut messages = Vec::new();
			for name in &names {
				match supervisor.start_service_filtered(name, all, &processes).await {
					Ok(msg) => messages.push(msg),
					Err(e) => return Response::Error { message: e },
				}
			}
			Response::Ok {
				message: Some(messages.join("\n")),
			}
		}
		Request::Stop { names } => {
			let mut messages = Vec::new();
			for name in &names {
				match supervisor.stop_service(name).await {
					Ok(msg) => messages.push(msg),
					Err(e) => return Response::Error { message: e },
				}
			}
			Response::Ok {
				message: Some(messages.join("\n")),
			}
		}
		Request::Reload { names, all, processes } => {
			let mut messages = Vec::new();
			for name in &names {
				match supervisor.reload_service_filtered(name, all, &processes).await {
					Ok(msg) => messages.push(msg),
					Err(e) => return Response::Error { message: e },
				}
			}
			Response::Ok {
				message: Some(messages.join("\n")),
			}
		}
		Request::Restart { service, process } => {
			match supervisor.restart_process(&service, &process).await {
				Ok(msg) => Response::Ok { message: Some(msg) },
				Err(e) => Response::Error { message: e },
			}
		}
		Request::Kill { service, process } => {
			match supervisor.kill_process(&service, &process).await {
				Ok(msg) => Response::Ok { message: Some(msg) },
				Err(e) => Response::Error { message: e },
			}
		}
		Request::Logs { service, process, follow: _ } => {
			match supervisor.get_output(&service, process.as_deref()).await {
				Ok(capture) => {
					let snapshot = capture.snapshot().await;
					Response::Log {
						line: String::from_utf8_lossy(&snapshot).to_string(),
					}
				}
				Err(e) => Response::Error { message: e },
			}
		}
		Request::Shutdown => {
			tokio::spawn(async {
				tokio::time::sleep(std::time::Duration::from_millis(100)).await;
				std::process::exit(0);
			});
			Response::Ok {
				message: Some("shutting down".to_string()),
			}
		}
	}
}

async fn write_response(
	writer: &mut tokio::net::unix::OwnedWriteHalf,
	response: &Response,
) -> Result<(), std::io::Error> {
	let mut data = serde_json::to_vec(response).unwrap();
	data.push(b'\n');
	writer.write_all(&data).await
}

async fn run_http_server(supervisor: Arc<supervisor::Supervisor>, port: u16) {
	let app = api::router(supervisor);
	let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
	let listener = match tokio::net::TcpListener::bind(addr).await {
		Ok(l) => l,
		Err(e) => {
			tracing::error!("failed to bind HTTP on {}: {}", addr, e);
			return;
		}
	};
	tracing::info!("HTTP listening on {}", addr);
	if let Err(e) = axum::serve(listener, app).await {
		tracing::error!("HTTP server error: {}", e);
	}
}
