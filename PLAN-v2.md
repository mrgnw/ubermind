# Implementation Plan: `ubermind` v2 — Native Daemon Manager

## Overview

Replace overmind/tmux dependency with native Rust process management for better reliability:
- Direct PID-based process supervision
- Auto-restart on crash with configurable retry limits
- Log files with rotation and expiry (timestamped: `YY-MMDD`, `YY-MMDD HH`, `YY-MMDD HH.MM`)
- Ring buffer for live log streaming
- Hybrid daemon model: spawn on demand, optional launchd/systemd installation
- Keep simple Procfile support, add optional TOML for advanced config
- Preserve existing CLI UX and UI

## Phase 0: Project Restructure

Convert to Cargo workspace:

```
ubermind/
  Cargo.toml                    # Workspace root
  crates/
    ubermind-core/              # Shared types, config, protocol
    ubermind-cli/               # CLI binary (thin client)
    ubermind-daemon/            # Supervisor daemon
  ui/                           # Existing (adapted later)
  completions/
```

## Phase 1: `ubermind-core` — Shared Types & Config

Config formats:
1. `~/.config/ubermind/projects` — existing `name: path`
2. `~/.config/ubermind/commands` — existing `name: command`
3. `~/.config/ubermind/config.toml` — new global config
4. `Procfile` — per-project processes
5. `.ubermind.toml` — per-project overrides

Log file naming: `{process} {YY-MMDD}.log`, rotated to `{process} {YY-MMDD} {HH}.log` or `{process} {YY-MMDD} {HH.MM}.log`

## Phase 2: `ubermind-daemon` — The Supervisor

- Spawns child processes with piped stdout/stderr
- Sets FORCE_COLOR=1, CLICOLOR_FORCE=1 by default
- Monitors via waitpid, auto-restarts on crash (configurable retries)
- Ring buffer (64KB) per process for live streaming
- Log files with rotation and expiry
- Unix socket server for CLI communication
- HTTP/WS server on port 13369 for UI

## Phase 3: `ubermind-cli` — Thin Client

- Talks to daemon via Unix socket
- Auto-starts daemon if not running
- Preserves existing UX (dots, colors, flexible args)
- `ub logs` tails files directly (no daemon needed)

## Phase 4: UI Adaptation

- Drop overmind/tmux shelling, talk to daemon HTTP API
- Terminal switches to incremental append (not clear+rewrite)

## Phase 5: Consolidation

- Daemon IS the HTTP server (no separate ubermind-serve)
- Optional `ub daemon install` for launchd/systemd

## Implementation Order

| Step | What | Effort | Depends on |
|------|------|--------|------------|
| 1 | Workspace restructure + `ubermind-core` | Small | — |
| 2 | `ubermind-daemon` supervisor | Medium | 1 |
| 3 | `ubermind-daemon` output capture | Medium | 2 |
| 4 | `ubermind-daemon` socket server | Small | 2, 3 |
| 5 | `ubermind-cli` rewrite | Medium | 1, 4 |
| 6 | `ubermind-daemon` HTTP/WS API | Small | 2, 3 |
| 7 | UI backend adaptation | Small | 6 |
| 8 | UI frontend tweaks | Small | 7 |
| 9 | Log expiry, daemon install | Small | 3 |
| 10 | Testing, polish | Medium | All |
