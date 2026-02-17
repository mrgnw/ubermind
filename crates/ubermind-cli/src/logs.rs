use crate::protocol::state_dir;
use std::path::PathBuf;

pub fn log_dir() -> PathBuf {
	state_dir().join("logs")
}

pub fn service_log_dir(service: &str) -> PathBuf {
	kagaya::logs::service_log_dir(&log_dir(), service)
}

#[cfg(test)]
mod tests {
	use kagaya::logs;

	#[test]
	fn test_parse_log_date() {
		assert_eq!(logs::parse_log_date("web 26-0214.log"), Some((26, 2, 14)));
		assert_eq!(logs::parse_log_date("web 26-0214 09.log"), Some((26, 2, 14)));
		assert_eq!(logs::parse_log_date("web 26-0214 09.47.log"), Some((26, 2, 14)));
		assert_eq!(logs::parse_log_date("invalid"), None);
	}

	#[test]
	fn test_secs_to_datetime() {
		let (y, m, d, h, min) = logs::secs_to_datetime(1771027200);
		assert_eq!((y, m, d, h, min), (2026, 2, 14, 0, 0));
	}
}
