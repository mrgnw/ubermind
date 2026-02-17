# kagaya — process supervisor toolkit for Rust CLIs

branch: muzan

## Status: implemented

Core supervisor extracted from ubermind and integrated back.

## What

Extract ubermind's process supervision into a reusable crate called `kagaya`.
Named after Ubuyashiki Kagaya (Demon Slayer) — the leader who commands and oversees the Demon Slayer Corps.

## What's done

### Core crate (`crates/kagaya/`)
- [x] `types.rs` — ProcessDef, ProcessState, ServiceType, ServiceStatus, ProcessStatus, Service
- [x] `logs.rs` — Date-based log naming, rotation, expiry, secs_to_datetime
- [x] `output.rs` — OutputCapture (ring buffer + broadcast + log file with rotation)
- [x] `supervisor.rs` — Supervisor, ManagedService, ManagedProcess, start/stop/reload/restart/kill
  - Process lifecycle: spawn via `sh -c`, process groups, SIGTERM/SIGKILL
  - Restart policy: configurable retries and delay, tasks never restart
  - Output piping: stdout/stderr -> OutputCapture

### Integration into ubermind
- [x] `types.rs` — re-exports from kagaya
- [x] `logs.rs` — thin wrapper delegating to kagaya::logs
- [x] `daemon/output.rs` — deleted, uses kagaya::OutputCapture directly
- [x] `daemon/supervisor.rs` — wraps kagaya::Supervisor, adds config loading + port detection

### Verified
- [x] `cargo check --workspace` — zero warnings
- [x] `cargo test -p kagaya` — 18 tests pass (2 unit + 16 integration)
- [x] `cargo test -p ubermind` — 2/2 tests pass

## Future work
- Publish to crates.io (alongside muzan)
- Doc comments on all public API
- Example binary
