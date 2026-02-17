use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::marker::PhantomData;

use serde::{Serialize, de::DeserializeOwned};

use crate::paths::DaemonPaths;

#[derive(Debug)]
pub enum ClientError {
	NotRunning,
	Io(io::Error),
	Serialize(String),
	Deserialize(String),
}

impl std::fmt::Display for ClientError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			ClientError::NotRunning => write!(f, "daemon not running"),
			ClientError::Io(e) => write!(f, "io error: {}", e),
			ClientError::Serialize(e) => write!(f, "serialize error: {}", e),
			ClientError::Deserialize(e) => write!(f, "deserialize error: {}", e),
		}
	}
}

impl std::error::Error for ClientError {}

impl From<io::Error> for ClientError {
	fn from(e: io::Error) -> Self {
		ClientError::Io(e)
	}
}

pub struct DaemonClient<Req, Resp> {
	stream: UnixStream,
	_phantom: PhantomData<(Req, Resp)>,
}

impl<Req, Resp> DaemonClient<Req, Resp>
where
	Req: Serialize,
	Resp: DeserializeOwned,
{
	pub fn connect(paths: &DaemonPaths) -> Result<Self, ClientError> {
		let socket_path = paths.socket_path();
		let stream = UnixStream::connect(&socket_path).map_err(|_| ClientError::NotRunning)?;
		Ok(Self {
			stream,
			_phantom: PhantomData,
		})
	}

	pub fn send(&mut self, request: &Req) -> Result<Resp, ClientError> {
		let mut data =
			serde_json::to_vec(request).map_err(|e| ClientError::Serialize(e.to_string()))?;
		data.push(b'\n');
		self.stream.write_all(&data)?;

		let mut reader = BufReader::new(&self.stream);
		let mut line = String::new();
		reader.read_line(&mut line)?;

		serde_json::from_str(&line).map_err(|e| ClientError::Deserialize(e.to_string()))
	}
}

pub fn is_running(paths: &DaemonPaths) -> bool {
	let socket_path = paths.socket_path();
	UnixStream::connect(&socket_path).is_ok()
}

pub fn read_pid(paths: &DaemonPaths) -> Option<u32> {
	let pid_path = paths.pid_path();
	std::fs::read_to_string(pid_path)
		.ok()
		.and_then(|s| s.trim().parse().ok())
}
