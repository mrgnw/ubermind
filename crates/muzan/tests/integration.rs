use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use serde::{Deserialize, Serialize};

use muzan::paths::DaemonPaths;
use muzan::client::{self, DaemonClient, ClientError};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum Req {
	Ping,
	Echo(String),
	Add(i32, i32),
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum Resp {
	Pong,
	Echo(String),
	Sum(i32),
	Error(String),
}

use std::sync::atomic::{AtomicU32, Ordering};
static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

fn temp_paths(name: &str) -> DaemonPaths {
	let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
	let app_name = format!("mzt{}{}", n, name);
	// Use /tmp directly so socket paths stay short (SUN_LEN limit ~104)
	unsafe { std::env::set_var("XDG_STATE_HOME", "/tmp") };
	DaemonPaths::new(app_name)
}

fn cleanup_paths(paths: &DaemonPaths) {
	let _ = std::fs::remove_file(paths.socket_path());
	let _ = std::fs::remove_file(paths.pid_path());
	let _ = std::fs::remove_dir(paths.state_dir());
}

// --- Paths tests ---

#[test]
fn paths_xdg_state_override() {
	let paths = DaemonPaths::new("testapp");
	let tmp = std::env::temp_dir();
	unsafe { std::env::set_var("XDG_STATE_HOME", &tmp) };
	assert_eq!(paths.state_dir(), tmp.join("testapp"));
	assert_eq!(paths.socket_path(), tmp.join("testapp").join("daemon.sock"));
	assert_eq!(paths.pid_path(), tmp.join("testapp").join("daemon.pid"));
}

#[test]
fn paths_xdg_config_override() {
	let paths = DaemonPaths::new("testapp");
	let tmp = std::env::temp_dir();
	unsafe { std::env::set_var("XDG_CONFIG_HOME", &tmp) };
	assert_eq!(paths.config_dir(), tmp.join("testapp"));
}

#[test]
fn paths_socket_and_pid_under_state() {
	let paths = DaemonPaths::new("myapp");
	let state = paths.state_dir();
	assert!(paths.socket_path().starts_with(&state));
	assert!(paths.pid_path().starts_with(&state));
	assert!(paths.socket_path().to_str().unwrap().ends_with("daemon.sock"));
	assert!(paths.pid_path().to_str().unwrap().ends_with("daemon.pid"));
}

// --- Client helpers ---

#[test]
fn is_running_false_when_no_socket() {
	let paths = temp_paths("no-socket");
	assert!(!client::is_running(&paths));
	cleanup_paths(&paths);
}

#[test]
fn read_pid_none_when_no_file() {
	let paths = temp_paths("no-pid");
	assert_eq!(client::read_pid(&paths), None);
	cleanup_paths(&paths);
}

#[test]
fn read_pid_parses_file() {
	let paths = temp_paths("pid-file");
	let _ = std::fs::create_dir_all(paths.state_dir());
	std::fs::write(paths.pid_path(), "12345\n").unwrap();
	assert_eq!(client::read_pid(&paths), Some(12345));
	cleanup_paths(&paths);
}

#[test]
fn read_pid_none_for_garbage() {
	let paths = temp_paths("pid-garbage");
	let _ = std::fs::create_dir_all(paths.state_dir());
	std::fs::write(paths.pid_path(), "not-a-number").unwrap();
	assert_eq!(client::read_pid(&paths), None);
	cleanup_paths(&paths);
}

#[test]
fn client_connect_returns_not_running() {
	let paths = temp_paths("no-server");
	let result = DaemonClient::<Req, Resp>::connect(&paths);
	match result {
		Err(ClientError::NotRunning) => {}
		Err(other) => panic!("expected NotRunning, got {:?}", other),
		Ok(_) => panic!("expected error, got Ok"),
	}
	cleanup_paths(&paths);
}

// --- Client + Server roundtrip ---

#[tokio::test]
async fn server_client_roundtrip() {
	let paths = temp_paths("roundtrip");
	let _ = std::fs::create_dir_all(paths.state_dir());

	let server_paths = paths.clone();
	let server_handle = tokio::spawn(async move {
		muzan::server::run_socket_server(&server_paths, |req: Req| async move {
			match req {
				Req::Ping => Resp::Pong,
				Req::Echo(s) => Resp::Echo(s),
				Req::Add(a, b) => Resp::Sum(a + b),
			}
		})
		.await;
	});

	// Wait for server to bind
	tokio::time::sleep(std::time::Duration::from_millis(100)).await;

	// Use sync client from a blocking task
	let client_paths = paths.clone();
	let result = tokio::task::spawn_blocking(move || {
		let mut client = DaemonClient::<Req, Resp>::connect(&client_paths).unwrap();

		let r1 = client.send(&Req::Ping).unwrap();
		assert_eq!(r1, Resp::Pong);

		let r2 = client.send(&Req::Echo("hello".into())).unwrap();
		assert_eq!(r2, Resp::Echo("hello".into()));

		let r3 = client.send(&Req::Add(3, 7)).unwrap();
		assert_eq!(r3, Resp::Sum(10));
	})
	.await;
	result.unwrap();

	server_handle.abort();
	cleanup_paths(&paths);
}

#[tokio::test]
async fn server_handles_multiple_clients() {
	let paths = temp_paths("multi-client");
	let _ = std::fs::create_dir_all(paths.state_dir());

	let server_paths = paths.clone();
	let server_handle = tokio::spawn(async move {
		muzan::server::run_socket_server(&server_paths, |req: Req| async move {
			match req {
				Req::Ping => Resp::Pong,
				Req::Echo(s) => Resp::Echo(s),
				Req::Add(a, b) => Resp::Sum(a + b),
			}
		})
		.await;
	});

	tokio::time::sleep(std::time::Duration::from_millis(100)).await;

	let mut handles = vec![];
	for i in 0..5 {
		let cp = paths.clone();
		handles.push(tokio::task::spawn_blocking(move || {
			let mut client = DaemonClient::<Req, Resp>::connect(&cp).unwrap();
			let resp = client.send(&Req::Add(i, 100)).unwrap();
			assert_eq!(resp, Resp::Sum(i + 100));
		}));
	}

	for h in handles {
		h.await.unwrap();
	}

	server_handle.abort();
	cleanup_paths(&paths);
}

#[tokio::test]
async fn server_parse_error_callback() {
	let paths = temp_paths("parse-error");
	let _ = std::fs::create_dir_all(paths.state_dir());

	let server_paths = paths.clone();
	let server_handle = tokio::spawn(async move {
		muzan::server::run_socket_server_with_error(
			&server_paths,
			|req: Req| async move {
				match req {
					Req::Ping => Resp::Pong,
					_ => Resp::Error("unexpected".into()),
				}
			},
			Some(|err: String| Resp::Error(err)),
		)
		.await;
	});

	tokio::time::sleep(std::time::Duration::from_millis(100)).await;

	// Send raw malformed JSON, expect error response
	let client_paths = paths.clone();
	let result = tokio::task::spawn_blocking(move || {
		let mut stream = UnixStream::connect(client_paths.socket_path()).unwrap();
		stream.write_all(b"this is not json\n").unwrap();

		let mut reader = BufReader::new(&stream);
		let mut line = String::new();
		reader.read_line(&mut line).unwrap();

		let resp: Resp = serde_json::from_str(&line).unwrap();
		match resp {
			Resp::Error(msg) => assert!(msg.contains("invalid request"), "got: {}", msg),
			other => panic!("expected Error, got {:?}", other),
		}
	})
	.await;
	result.unwrap();

	server_handle.abort();
	cleanup_paths(&paths);
}

#[tokio::test]
async fn server_without_error_callback_drops_bad_requests() {
	let paths = temp_paths("no-error-cb");
	let _ = std::fs::create_dir_all(paths.state_dir());

	let server_paths = paths.clone();
	let server_handle = tokio::spawn(async move {
		muzan::server::run_socket_server(&server_paths, |req: Req| async move {
			match req {
				Req::Ping => Resp::Pong,
				_ => Resp::Error("unexpected".into()),
			}
		})
		.await;
	});

	tokio::time::sleep(std::time::Duration::from_millis(100)).await;

	// Send bad JSON then a valid request — should get response to valid one
	let client_paths = paths.clone();
	let result = tokio::task::spawn_blocking(move || {
		let mut stream = UnixStream::connect(client_paths.socket_path()).unwrap();
		stream
			.set_read_timeout(Some(std::time::Duration::from_secs(2)))
			.unwrap();

		// Bad request (silently dropped)
		stream.write_all(b"garbage\n").unwrap();
		// Valid request
		let req = serde_json::to_string(&Req::Ping).unwrap();
		stream.write_all(format!("{}\n", req).as_bytes()).unwrap();

		let mut reader = BufReader::new(&stream);
		let mut line = String::new();
		reader.read_line(&mut line).unwrap();

		let resp: Resp = serde_json::from_str(&line).unwrap();
		assert_eq!(resp, Resp::Pong);
	})
	.await;
	result.unwrap();

	server_handle.abort();
	cleanup_paths(&paths);
}

// --- Daemon cleanup ---

#[test]
fn daemon_cleanup_removes_files() {
	let paths = temp_paths("cleanup");
	let _ = std::fs::create_dir_all(paths.state_dir());
	std::fs::write(paths.socket_path(), "fake").unwrap();
	std::fs::write(paths.pid_path(), "99999").unwrap();

	assert!(paths.socket_path().exists());
	assert!(paths.pid_path().exists());

	let daemon = muzan::Daemon::new(paths.app_name.clone());
	daemon.cleanup();

	assert!(!paths.socket_path().exists());
	assert!(!paths.pid_path().exists());

	cleanup_paths(&paths);
}

// --- ClientError Display ---

#[test]
fn client_error_display() {
	assert_eq!(format!("{}", ClientError::NotRunning), "daemon not running");
	assert_eq!(
		format!("{}", ClientError::Serialize("bad".into())),
		"serialize error: bad"
	);
	assert_eq!(
		format!("{}", ClientError::Deserialize("bad".into())),
		"deserialize error: bad"
	);
}
