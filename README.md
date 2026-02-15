<p align="center">
  <img src="logo.svg" width="128" height="128" alt="ubermind logo">
</p>

# ubermind

A native Rust process supervisor for managing multiple projects. Each project keeps its own `Procfile`, and ubermind orchestrates them all from anywhere with auto-restart, log management, and live monitoring.

Inspired by [overmind](https://github.com/DarthSim/overmind) and [foreman](https://github.com/ddollar/foreman).

## Install

```sh
# shell script (prebuilt binary)
curl -fsSL https://raw.githubusercontent.com/mrgnw/ubermind/main/install.sh | sh

# gah (github asset helper)
gah install mrgnw/ubermind

# cargo
cargo install ubermind       # from source
cargo binstall ubermind      # prebuilt binary
```

### Shell completion

Tab completion for commands, project names, and flags:

```sh
ub start appli<tab>    # completes to: ub start appligator
ub st<tab>             # completes to: ub status / ub start / ub stop
ub status <tab>        # shows all project names
```

**Setup:**

If installed via the install script, completions are in `~/.local/share/ubermind/completions/`.

**Bash:**
```sh
echo 'source ~/.local/share/ubermind/completions/ub.bash' >> ~/.bashrc
```

**Zsh:**
```sh
# Add to ~/.zshrc
fpath=(~/.local/share/ubermind/completions $fpath)
autoload -Uz compinit && compinit
```

**Fish:**
```sh
ln -s ~/.local/share/ubermind/completions/ub.fish ~/.config/fish/completions/
```

## Quick start

### 1. Initialize ubermind

```sh
ubermind init
```

This creates a projects config at `~/.config/ubermind/projects`.

### 2. Create a Procfile in your project

Each project you want to manage needs a `Procfile` in its root directory. A Procfile lists the processes to run — one per line, in `name: command` format:

```sh
# ~/dev/myapp/Procfile
web: npm run dev
api: python server.py
worker: ruby worker.rb
```

This is the standard [Procfile](https://devcenter.heroku.com/articles/procfile) format. Each line becomes a named process that ubermind will manage.

### 3. Register your project with ubermind

```sh
ubermind add myapp ~/dev/myapp
```

This tells ubermind "there's a project called `myapp` at `~/dev/myapp` that has a Procfile."

### 4. Start it

```sh
ubermind start myapp    # start one project
ubermind start          # or start everything
```

## How it fits together

ubermind has two layers of config:

**Projects file** (`~/.config/ubermind/projects`) — maps project names to directories:

```
myapp: ~/dev/myapp
api: ~/dev/api-server
frontend: ~/dev/frontend
```

**Commands file** (`~/.config/ubermind/commands`) — optional, defines standalone commands in Procfile format:

```
tunnel: ssh -N -L 5432:localhost:5432 prod-server
sync: watchman-wait . --max-events 0 -p '*.json' | xargs ./sync.sh
```

Each project directory has its own **Procfile** that defines what processes to run:

```
# ~/dev/myapp/Procfile
web: npm run dev
api: python server.py

# ~/dev/api-server/Procfile
server: cargo run
worker: cargo run --bin worker

# ~/dev/frontend/Procfile
dev: pnpm dev
```

When you run `ubermind start myapp`, ubermind looks up `myapp` → `~/dev/myapp`, then starts its daemon in that directory using the `Procfile`. Each project gets its own isolated supervisor instance — one project crashing won't affect the others.

Standalone commands from the `commands` file are auto-expanded into generated Procfiles under `~/.config/ubermind/_commands/`.

## Usage

```
ubermind init                # create projects config file
ubermind add <name> <dir>    # register a project directory

ubermind status              # show all projects
ubermind start [name]        # start project(s)
ubermind stop [name]         # stop project(s)
ubermind reload [name]       # restart project(s) (picks up Procfile changes)
ubermind kill [name]         # kill process(es) in project(s)
ubermind restart [name]      # restart process(es) in project(s)
ubermind echo [name]         # view logs from project(s)
ubermind connect [name]      # connect to a process in a project
ubermind serve [-p PORT]     # start web UI server (default port: 13369)
```

Pass any overmind command to a specific project:

```
ubermind status myapp        # show status of myapp
ubermind echo myapp          # view myapp's logs
ubermind myapp connect web   # attach to myapp's web process
ubermind connect web myapp   # same thing, project name last
```

Omit the name to target all projects:

```
ubermind start               # start all projects
ubermind stop                # stop all projects
ubermind echo                # view logs from all projects
```

## Config

The projects file lives at `~/.config/ubermind/projects` (respects `$XDG_CONFIG_HOME`).

You can edit it directly or use `ubermind add`:

```
# name: directory
myapp: ~/dev/myapp
api: ~/dev/api-server
frontend: ~/dev/frontend
```

Optionally, define standalone commands in `~/.config/ubermind/commands`:

```
tunnel: ssh -N -L 5432:localhost:5432 prod-server
sync: watchman-wait . --max-events 0 -p '*.json' | xargs ./sync.sh
```

See [tmux cheatsheet](tmux.md) for navigating connected sessions (scrolling, copying error text, etc).

## How it works

ubermind uses native Rust process supervision with:
- Direct PID-based process management
- Auto-restart on crash with configurable retry limits
- Log files with rotation (stored in `~/.local/share/ubermind/log/`)
- Live log streaming via ring buffers
- Unix socket communication for CLI commands
- HTTP/WebSocket API for the web UI

Each project directory gets its own independent supervisor instance. ubermind knows where each project lives and dispatches commands to the right supervisor.

Standalone commands are auto-expanded into generated Procfiles under `~/.config/ubermind/_commands/` (an internal directory that you shouldn't edit directly).

## License

MIT

## History

ubermind v0.6+ uses native Rust process management. Earlier versions (v0.1-v0.5) were thin wrappers around [overmind](https://github.com/DarthSim/overmind) and tmux.