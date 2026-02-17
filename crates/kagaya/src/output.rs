use std::collections::VecDeque;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::Mutex;

use crate::logs;

const RING_BUFFER_SIZE: usize = 64 * 1024;

#[derive(Clone)]
pub struct OutputCapture {
	ring: Arc<Mutex<VecDeque<u8>>>,
	log_writer: Arc<Mutex<LogWriter>>,
	sender: broadcast::Sender<Vec<u8>>,
}

struct LogWriter {
	file: Option<File>,
	path: PathBuf,
	bytes_written: u64,
	max_size: u64,
	log_dir: PathBuf,
	process: String,
}

impl OutputCapture {
	pub fn new(log_dir: &Path, service: &str, process: &str, max_log_size: u64) -> Self {
		let svc_log_dir = logs::service_log_dir(log_dir, service);
		let _ = fs::create_dir_all(&svc_log_dir);

		let log_name = logs::current_log_name(process);
		let log_path = svc_log_dir.join(&log_name);

		let file = OpenOptions::new()
			.create(true)
			.append(true)
			.open(&log_path)
			.ok();

		let bytes_written = file
			.as_ref()
			.and_then(|f| f.metadata().ok())
			.map(|m| m.len())
			.unwrap_or(0);

		let (sender, _) = broadcast::channel(256);

		Self {
			ring: Arc::new(Mutex::new(VecDeque::with_capacity(RING_BUFFER_SIZE))),
			log_writer: Arc::new(Mutex::new(LogWriter {
				file,
				path: log_path,
				bytes_written,
				max_size: max_log_size,
				log_dir: svc_log_dir,
				process: process.to_string(),
			})),
			sender,
		}
	}

	pub async fn write(&self, data: &[u8]) {
		{
			let mut ring = self.ring.lock().await;
			for &byte in data {
				if ring.len() >= RING_BUFFER_SIZE {
					ring.pop_front();
				}
				ring.push_back(byte);
			}
		}

		{
			let mut writer = self.log_writer.lock().await;
			writer.write(data);
		}

		let _ = self.sender.send(data.to_vec());
	}

	pub async fn snapshot(&self) -> Vec<u8> {
		let ring = self.ring.lock().await;
		ring.iter().copied().collect()
	}

	pub fn subscribe(&self) -> broadcast::Receiver<Vec<u8>> {
		self.sender.subscribe()
	}
}

impl LogWriter {
	fn write(&mut self, data: &[u8]) {
		if let Some(ref mut file) = self.file {
			let _ = file.write_all(data);

			self.bytes_written += data.len() as u64;

			if self.bytes_written >= self.max_size {
				self.rotate();
			}
		}
	}

	fn rotate(&mut self) {
		if let Some(file) = self.file.take() {
			drop(file);
		}

		let rotated_name = logs::rotated_log_name(&self.log_dir, &self.process);
		let rotated_path = self.log_dir.join(&rotated_name);
		let _ = fs::rename(&self.path, &rotated_path);

		let new_name = logs::current_log_name(&self.process);
		self.path = self.log_dir.join(&new_name);
		self.file = OpenOptions::new()
			.create(true)
			.append(true)
			.open(&self.path)
			.ok();
		self.bytes_written = 0;
	}
}
