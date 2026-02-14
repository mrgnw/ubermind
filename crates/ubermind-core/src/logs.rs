use crate::protocol::state_dir;
use std::path::PathBuf;

pub fn log_dir() -> PathBuf {
	state_dir().join("logs")
}

pub fn service_log_dir(service: &str) -> PathBuf {
	log_dir().join(service)
}

pub fn current_log_name(process: &str) -> String {
	let now = now_ymd();
	format!("{} {}.log", process, now)
}

pub fn rotated_log_name(process: &str) -> String {
	let now = now_ymdhm();
	let (date, hour, minute) = now;
	let candidate = format!("{} {} {}.log", process, date, hour);
	let candidate_path = log_dir().join(&candidate);
	if candidate_path.exists() {
		format!("{} {} {}.{}.log", process, date, hour, minute)
	} else {
		candidate
	}
}

pub fn parse_log_date(filename: &str) -> Option<(u32, u32, u32)> {
	// Extract "YY-MMDD" from filename like "web 26-0214.log"
	let parts: Vec<&str> = filename.splitn(2, ' ').collect();
	if parts.len() < 2 {
		return None;
	}
	let rest = parts[1];
	// Extract date portion - everything before first space or .log
	let date_str = rest
		.split(' ')
		.next()
		.unwrap_or(rest)
		.trim_end_matches(".log");

	// Parse "YY-MMDD"
	let parts: Vec<&str> = date_str.splitn(2, '-').collect();
	if parts.len() != 2 {
		return None;
	}
	let year: u32 = parts[0].parse().ok()?;
	let mmdd = parts[1];
	if mmdd.len() != 4 {
		return None;
	}
	let month: u32 = mmdd[..2].parse().ok()?;
	let day: u32 = mmdd[2..].parse().ok()?;
	Some((year, month, day))
}

fn now_ymd() -> String {
	// Format: "YY-MMDD"
	use std::time::SystemTime;
	let now = SystemTime::now()
		.duration_since(SystemTime::UNIX_EPOCH)
		.unwrap()
		.as_secs();
	let (year, month, day, _, _) = secs_to_datetime(now);
	format!("{:02}-{:02}{:02}", year % 100, month, day)
}

fn now_ymdhm() -> (String, String, String) {
	use std::time::SystemTime;
	let now = SystemTime::now()
		.duration_since(SystemTime::UNIX_EPOCH)
		.unwrap()
		.as_secs();
	let (year, month, day, hour, minute) = secs_to_datetime(now);
	(
		format!("{:02}-{:02}{:02}", year % 100, month, day),
		format!("{:02}", hour),
		format!("{:02}", minute),
	)
}

fn secs_to_datetime(secs: u64) -> (u32, u32, u32, u32, u32) {
	// Simple UTC datetime from unix timestamp
	let days = (secs / 86400) as i64;
	let time_of_day = secs % 86400;
	let hour = (time_of_day / 3600) as u32;
	let minute = ((time_of_day % 3600) / 60) as u32;

	// Days since epoch to date (civil_from_days algorithm)
	let z = days + 719468;
	let era = if z >= 0 { z } else { z - 146096 } / 146097;
	let doe = (z - era * 146097) as u32;
	let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
	let y = yoe as i64 + era * 400;
	let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
	let mp = (5 * doy + 2) / 153;
	let d = doy - (153 * mp + 2) / 5 + 1;
	let m = if mp < 10 { mp + 3 } else { mp - 9 };
	let y = if m <= 2 { y + 1 } else { y };

	(y as u32, m, d, hour, minute)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse_log_date() {
		assert_eq!(parse_log_date("web 26-0214.log"), Some((26, 2, 14)));
		assert_eq!(parse_log_date("web 26-0214 09.log"), Some((26, 2, 14)));
		assert_eq!(parse_log_date("web 26-0214 09.47.log"), Some((26, 2, 14)));
		assert_eq!(parse_log_date("invalid"), None);
	}

	#[test]
	fn test_secs_to_datetime() {
		// 2026-02-14 00:00:00 UTC = 1771027200
		let (y, m, d, h, min) = secs_to_datetime(1771027200);
		assert_eq!((y, m, d, h, min), (2026, 2, 14, 0, 0));
	}
}
