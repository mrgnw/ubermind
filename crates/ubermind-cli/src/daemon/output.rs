use std::collections::VecDeque;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
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
	service: String,
	process: String,
}

impl OutputCapture {
	pub fn new(service: &str, process: &str, max_log_size: u64) -> Self {
		let log_dir = logs::service_log_dir(service);
		let _ = fs::create_dir_all(&log_dir);

		let log_name = logs::current_log_name(process);
		let log_path = log_dir.join(&log_name);

		let file = OpenOptions::new()
			.create(true)
			.append(true)
			.open(&log_path)
			.ok();

		let bytes_written = file.as_ref().and_then(|f| f.metadata().ok()).map(|m| m.len()).unwrap_or(0);

		let (sender, _) = broadcast::channel(256);

		Self {
			ring: Arc::new(Mutex::new(VecDeque::with_capacity(RING_BUFFER_SIZE))),
			log_writer: Arc::new(Mutex::new(LogWriter {
				file,
				path: log_path,
				bytes_written,
				max_size: max_log_size,
				service: service.to_string(),
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

		let log_dir = logs::service_log_dir(&self.service);
		let rotated_name = logs::rotated_log_name(&self.process);
		let rotated_path = log_dir.join(&rotated_name);
		let _ = fs::rename(&self.path, &rotated_path);

		let new_name = logs::current_log_name(&self.process);
		self.path = log_dir.join(&new_name);
		self.file = OpenOptions::new()
			.create(true)
			.append(true)
			.open(&self.path)
			.ok();
		self.bytes_written = 0;
	}
}

pub fn expire_logs(max_age_days: u32, max_files: u32) {
	let log_dir = logs::log_dir();
	if !log_dir.exists() {
		return;
	}

	let entries = match fs::read_dir(&log_dir) {
		Ok(e) => e,
		Err(_) => return,
	};

	for entry in entries.flatten() {
		if !entry.path().is_dir() {
			continue;
		}
		expire_service_logs(&entry.path(), max_age_days, max_files);
	}
}

fn expire_service_logs(dir: &std::path::Path, max_age_days: u32, max_files: u32) {
	let mut log_files: Vec<(PathBuf, Option<(u32, u32, u32)>)> = Vec::new();

	let entries = match fs::read_dir(dir) {
		Ok(e) => e,
		Err(_) => return,
	};

	for entry in entries.flatten() {
		let path = entry.path();
		if path.extension().and_then(|e| e.to_str()) != Some("log") {
			continue;
		}
		let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
		let date = logs::parse_log_date(&name);
		log_files.push((path, date));
	}

	if max_age_days > 0 {
		let now_secs = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_secs();
		let cutoff_secs = now_secs.saturating_sub(max_age_days as u64 * 86400);

		for (path, date) in &log_files {
			if let Some((y, m, d)) = date {
				let file_epoch = date_to_epoch(*y, *m, *d);
				if file_epoch < cutoff_secs {
					let _ = fs::remove_file(path);
				}
			}
		}
	}

	if max_files > 0 && log_files.len() > max_files as usize {
		log_files.sort_by(|a, b| {
			let a_time = a.0.metadata().and_then(|m| m.modified()).ok();
			let b_time = b.0.metadata().and_then(|m| m.modified()).ok();
			a_time.cmp(&b_time)
		});
		let to_remove = log_files.len() - max_files as usize;
		for (path, _) in log_files.iter().take(to_remove) {
			let _ = fs::remove_file(path);
		}
	}
}

fn date_to_epoch(year: u32, month: u32, day: u32) -> u64 {
	let full_year = if year < 100 { 2000 + year } else { year };
	let y = full_year as i64;
	let m = month as i64;
	let d = day as i64;

	let y_adj = if m <= 2 { y - 1 } else { y };
	let m_adj = if m <= 2 { m + 9 } else { m - 3 };

	let era = if y_adj >= 0 { y_adj } else { y_adj - 399 } / 400;
	let yoe = y_adj - era * 400;
	let doy = (153 * m_adj + 2) / 5 + d - 1;
	let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
	let days = era * 146097 + doe - 719468;
	(days * 86400) as u64
}
