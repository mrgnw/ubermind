# Echo Filtering

Allows filtering `echo` output by specific service names.

## Usage

```bash
ubermind echo <project> [service1] [service2] ...
```

Example:
```bash
ub echo appligator ui api
```

## Implementation Details

-   Spawns `overmind echo` with `CLICOLOR_FORCE=1` to ensure color codes are preserved even though stdout is piped.
-   Reads the output line-by-line.
-   Parses the service name prefix (handling ANSI color codes).
-   Filters lines based on the provided service names.

## Upstream Status

-   [ ] **Todo**: Investigate if we should create a PR for `overmind` to add native support for `overmind echo <service>`.
    -   *Considerations*: Overmind is written in Go. This feature would require modifying the `echo` command handler in Overmind to accept arguments and filter the log stream.
