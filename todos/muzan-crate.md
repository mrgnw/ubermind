# muzan — daemon lifecycle toolkit for Rust CLIs

branch: muzan

## What

Extract ubermind's daemon plumbing into a reusable crate called `muzan`.
Named after Kibutsuji Muzan (Demon Slayer) — the progenitor demon who creates and commands all other demons.

Any Rust CLI can use `muzan` to add a background daemon mode with IPC, auto-start, and lifecycle management.

## Why

No Rust crate exists for this. The ecosystem has:
- `daemonize` / `fork` — just the fork/detach syscalls, no IPC
- `interprocess` / `tipsy` — just transport, no protocol or daemon pattern
- `service-manager` — delegates to OS (systemd/launchd), not self-managed
- `psup` — dead (2021), attempted this exact thing
- `pmdaemon` — monolithic PM2 clone, not composable

`muzan` fills the gap: composable daemon lifecycle for any CLI.

## Scope

### In scope (what muzan provides)
- **Paths** — XDG-compliant state/config dirs, socket_path, pid_path, configurable app name
- **Server** — tokio Unix socket server, newline-delimited JSON, generic over user's `Request`/`Response` types
- **Client** — sync connect + send/recv over Unix socket
- **Daemon lifecycle** — daemonize (fork current binary as background), PID file write/cleanup, socket cleanup
- **ensure_daemon** — check PID, auto-start daemon if not running, wait for socket, return connected client
- **Shutdown** — graceful shutdown on SIGTERM/SIGINT, cleanup

### Out of scope (stays in ubermind)
- Process supervision (Supervisor, ManagedProcess, run_process_loop)
- Output capture, ring buffer, log rotation
- HTTP/WebSocket API
- Config file format (projects.toml, services.toml)
- launchd integration
- Web UI

### Future (not now)
- `feature = "clap"` for ready-made subcommands
- Process supervisor as a second crate layered on top

## Design

### Generic protocol
Users define their own request/response enums. Muzan just needs `Serialize + DeserializeOwned`:

```rust
use muzan::{Daemon, DaemonClient};

#[derive(Serialize, Deserialize)]
enum Req { Ping, DoWork { id: u32 } }

#[derive(Serialize, Deserialize)]
enum Resp { Pong, Done, Error(String) }

// Server side
let daemon = Daemon::new("myapp");
daemon.run(|req: Req| async {
	match req {
		Req::Ping => Resp::Pong,
		Req::DoWork { id } => { /* ... */ Resp::Done }
	}
}).await;

// Client side
let client = DaemonClient::<Req, Resp>::ensure("myapp")?;
let resp = client.send(&Req::Ping)?;
```

### Module layout
```
crates/muzan/src/
  lib.rs       — re-exports
  paths.rs     — XDG dirs, socket/pid paths
  server.rs    — Unix socket server, JSON framing
  client.rs    — sync client, connect, send/recv
  daemon.rs    — daemonize, PID file, run loop, shutdown
```

### Dependencies
- `tokio` (net, signal, io, macros, rt-multi-thread)
- `serde` + `serde_json`
- `nix` (signal, process) — for kill/cleanup
- `tracing` — optional logging

### Platform
Unix only (macOS + Linux). No Windows support.

## Implementation steps

1. Create `crates/muzan/` with Cargo.toml
2. Write `paths.rs` — extracted from protocol.rs, parameterized by app name
3. Write `server.rs` — extracted from daemon/mod.rs socket server, generic over handler
4. Write `client.rs` — extracted from main.rs connect_daemon/send_request
5. Write `daemon.rs` — daemonize + PID + run loop + shutdown
6. Write `lib.rs` — public API surface
7. Add muzan to workspace
8. Refactor ubermind-cli to depend on muzan
9. Verify everything compiles
