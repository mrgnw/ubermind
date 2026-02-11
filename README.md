# ubermind

Manage multiple [overmind](https://github.com/DarthSim/overmind) instances across projects. Each project keeps its own `Procfile` and runs its own overmind daemon. ubermind orchestrates them all from anywhere.

## Prerequisites

- [overmind](https://github.com/DarthSim/overmind) (which requires [tmux](https://github.com/tmux/tmux))

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

Services are listed in `~/.config/ubermind/services` (Procfile-style):

```
myapp: ~/dev/myapp
api: ~/dev/api-server
frontend: ~/dev/frontend
```

Respects `$XDG_CONFIG_HOME` if set.

See [tmux cheatsheet](tmux.md) for navigating connected sessions (scrolling, copying error text, etc).

## How it works

Each project directory gets its own independent overmind instance with its own `.overmind.sock`. ubermind knows where each project lives and dispatches commands to the right overmind in the right directory.

- `start`/`stop`/`reload` are ubermind-level commands that manage the overmind daemon lifecycle (daemonized start, graceful quit, socket cleanup).
- Everything else is passed through directly to the project's overmind.

## License

MIT

## Extended Features

See [extended_features/README.md](extended_features/README.md) for features added on top of standard overmind functionality.
