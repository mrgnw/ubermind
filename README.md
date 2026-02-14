<p align="center">
  <img src="logo.svg" width="128" height="128" alt="ubermind logo">
</p>

# ubermind

Manage multiple [overmind](https://github.com/DarthSim/overmind) instances across projects. Each project keeps its own `Procfile` and runs its own overmind daemon. ubermind orchestrates them all from anywhere.

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

overmind (and tmux) will be installed automatically if missing.

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

This is the standard [Procfile](https://devcenter.heroku.com/articles/procfile) format used by overmind, foreman, and others. Each line becomes a named process that ubermind (via overmind) will manage.

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
# Each line is a standalone command that runs as its own overmind instance
tunnel: ssh -N -L 5432:localhost:5432 prod-server
sync: watchman-wait . --max-events 0 -p '*.json' | xargs ./sync.sh
```

Each project directory has its own **Procfile** that defines what processes to run:

```
# ~/dev/myapp/Procfile
web: npm run dev
db: docker compose up postgres

# ~/dev/api-server/Procfile
server: cargo run
worker: cargo run --bin worker

# ~/dev/frontend/Procfile
dev: pnpm dev
```

When you run `ubermind start myapp`, ubermind looks up `myapp` → `~/dev/myapp`, then starts overmind in that directory using its `Procfile`. Each project gets its own isolated overmind instance — one project crashing won't affect the others.

Standalone commands from the `commands` file are auto-expanded into generated Procfiles under `~/.config/ubermind/_commands/` so overmind can manage them the same way.

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
ubermind status myapp        # overmind status within myapp
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
# These run as individual overmind instances without a project directory
tunnel: ssh -N -L 5432:localhost:5432 prod-server
sync: watchman-wait . --max-events 0 -p '*.json' | xargs ./sync.sh
```

See [tmux cheatsheet](tmux.md) for navigating connected sessions (scrolling, copying error text, etc).

## How it works

Each project directory gets its own independent overmind instance with its own `.overmind.sock`. ubermind knows where each project lives and dispatches commands to the right overmind in the right directory.

Standalone commands are auto-expanded into generated Procfiles under `~/.config/ubermind/_commands/` (an internal directory that you shouldn't edit directly).

- `start`/`stop`/`reload` are ubermind-level commands that manage the overmind daemon lifecycle (daemonized start, graceful quit, socket cleanup).
- Everything else is passed through directly to the project's overmind.

## License

MIT

## Extended Features

See [extended_features/README.md](extended_features/README.md) for features added on top of standard overmind functionality.