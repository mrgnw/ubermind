//! # muzan
//!
//! Daemon lifecycle toolkit for Rust CLIs.
//!
//! Add a background daemon mode to any CLI with Unix socket IPC,
//! auto-start, PID management, and graceful shutdown.
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use muzan::{Daemon, DaemonClient, ensure_daemon_with_args};
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Serialize, Deserialize)]
//! enum Req { Ping }
//!
//! #[derive(Serialize, Deserialize)]
//! enum Resp { Pong }
//!
//! // Server: run the daemon
//! let daemon = Daemon::new("myapp");
//! // daemon.run(|req: Req| async { Resp::Pong }).await;
//!
//! // Client: connect (auto-starting if needed)
//! let mut client = ensure_daemon_with_args::<Req, Resp>(
//!     &daemon.paths,
//!     &["daemon", "run"],
//! ).unwrap();
//! let resp = client.send(&Req::Ping).unwrap();
//! ```

pub mod paths;
pub mod server;
pub mod client;
pub mod daemon;

#[cfg(feature = "clap")]
pub mod clap;

pub use paths::DaemonPaths;
pub use client::{DaemonClient, ClientError};
pub use daemon::Daemon;
pub use daemon::ensure_daemon;
pub use daemon::ensure_daemon_with_args;

#[cfg(feature = "clap")]
pub use crate::clap::DaemonCommand;
