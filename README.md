# ubermind

Manage multiple [overmind](https://github.com/DarthSim/overmind) instances across projects. Each project keeps its own `Procfile` and runs its own overmind daemon. ubermind orchestrates them all from anywhere.

## Prerequisites

- [overmind](https://github.com/DarthSim/overmind) (which requires [tmux](https://github.com/tmux/tmux))

## Install

```
cargo install ubermind
```

## Quick start

```
ubermind init
ubermind add myapp ~/dev/myapp
ubermind start myapp
```

The project directory needs a `Procfile` (standard [overmind](https://github.com/DarthSim/overmind)/[foreman](https://github.com/theforeman/foreman) format):

```
web: pnpm dev
api: uv run server.py
```

## Usage

```
project="your_project"

ubermind init                # create standard config file
ubermind add $project <dir>  # add a service in <dir>

ubermind status              # show all services
ubermind start $project      # start service(s)
ubermind stop $project       # stop service(s)
ubermind reload $project     # restart service(s) (picks up Procfile changes)`
```

Pass any overmind command to a specific project:

```
ubermind status myapp        overmind status within myapp
ubermind echo myapp          view myapp's logs
ubermind myapp connect web   attach to myapp's web process
ubermind connect web myapp   same thing, project name last
```

## Config

Services are listed in `~/.config/ubermind/services.tsv` (tab-separated):

```
myapp	~/dev/myapp
api	~/dev/api-server
frontend	~/dev/frontend
```

Respects `$XDG_CONFIG_HOME` if set.

## tmux cheatsheet

When you `connect` to a process, you're inside a tmux session. The prefix key is `Ctrl+b` (press Ctrl+b first, release, then press the next key).

### Basics

| Keys | Action |
|------|--------|
| `Ctrl+b`, `d` | Detach (exit back to your shell) |
| `Ctrl+c` | Send interrupt to the running process |

### Scrolling / copy mode

| Keys | Action |
|------|--------|
| `Ctrl+b`, `[` | Enter scroll mode |
| `q` or `Esc` | Exit scroll mode |
| Up/Down or `k`/`j` | Scroll line by line |
| `Ctrl+u` / `Ctrl+d` | Scroll half page up/down |
| `g` / `G` | Jump to top/bottom of history |

### Selecting and copying text in scroll mode

| Keys | Action |
|------|--------|
| `Space` | Start selection |
| Arrow keys or `h`/`j`/`k`/`l` | Expand selection |
| `Enter` | Copy selection to tmux buffer |
| `Ctrl+b`, `]` | Paste tmux buffer |

### Typical workflow: copy an error

1. `ubermind connect web myapp` — attach to the process
2. `Ctrl+b`, `[` — enter scroll mode
3. Scroll up to find the error
4. `Space` — start selecting
5. Move to end of error text
6. `Enter` — copy
7. `Ctrl+b`, `d` — detach
8. Paste wherever you need it

> On macOS Terminal/iTerm2, you can also hold `Option` and click-drag to select text directly, or use `Cmd+C` to copy the visible terminal content — no scroll mode needed if the error is still on screen.

## How it works

Each project directory gets its own independent overmind instance with its own `.overmind.sock`. ubermind knows where each project lives and dispatches commands to the right overmind in the right directory.

- `start`/`stop`/`reload` are ubermind-level commands that manage the overmind daemon lifecycle (daemonized start, graceful quit, socket cleanup).
- Everything else is passed through directly to the project's overmind.

## License

MIT
