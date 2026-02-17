# muzan

Daemon lifecycle toolkit for Rust CLIs.

Add a background daemon mode to any CLI with Unix socket IPC, auto-start, PID management, and graceful shutdown. Unix only (macOS + Linux).

## Features

- **Generic IPC** — Newline-delimited JSON over Unix sockets. Bring your own `Serialize`/`Deserialize` request and response types.
- **`ensure_daemon`** — Auto-start the daemon if it isn't running. One call does "connect or spawn + wait + connect."
- **PID management** — Writes/reads PID files, sends SIGTERM to stop.
- **XDG paths** — State and config dirs follow XDG conventions, scoped by app name.
- **Optional clap integration** — Feature flag for ready-made `Run`/`Start`/`Stop`/`Status` subcommands.

## Quick start

```rust
use muzan::{Daemon, DaemonClient, DaemonPaths, ensure_daemon_with_args};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
enum Req { Ping }

#[derive(Serialize, Deserialize, Debug)]
enum Resp { Pong }

// Server side: run the daemon
async fn run_server() {
    let daemon = Daemon::new("myapp");
    daemon.run(|req: Req| async move {
        match req {
            Req::Ping => Resp::Pong,
        }
    }).await;
}

// Client side: connect (auto-starting if needed)
fn send_request() {
    let paths = DaemonPaths::new("myapp");
    let mut client = ensure_daemon_with_args::<Req, Resp>(
        &paths,
        &["daemon", "run"],
    ).unwrap();
    let resp = client.send(&Req::Ping).unwrap();
    println!("{:?}", resp);
}
```

## Clap integration

Enable the `clap` feature for built-in subcommands:

```toml
muzan = { version = "0.1", features = ["clap"] }
```

```rust
use clap::{Parser, Subcommand};
use muzan::{Daemon, DaemonCommand, DaemonPaths};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    #[command(subcommand)]
    Daemon(DaemonCommand),
}
```

The `DaemonCommand` enum provides `run`, `start`, `stop`, and `status` subcommands. Handle `Run` yourself (to wire up your handler); `Start`, `Stop`, and `Status` work out of the box.

## See also

- `examples/echo.rs` — complete working example
