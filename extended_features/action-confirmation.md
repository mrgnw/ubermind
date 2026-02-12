# Action Confirmation with Visual Feedback

Ubermind provides real-time feedback when managing services, showing command progress and verifying that actions actually succeeded.

## The Problem

Overmind's commands (`restart`, `stop`, `kill`, `quit`) exit immediately with status code 0 when they *accept* a command, but this doesn't guarantee the requested action actually happened. A process might fail to start, crash on boot, or take several seconds to fully stop.

## The Solution

Ubermind polls overmind status after sending commands and prints dots (`.`) each second while waiting (up to 5 seconds), then confirms the actual result.

## Output Format

```bash
# Success after 1-2 seconds
api: restart. running

# Success after 3 seconds
api: stopping... stopped

# Timeout after 5 seconds
api: restart..... failed

# Command accepted but can't verify
api: kill. killed
```

## Commands with Verification

| Command | Verifies | Success Message |
|---------|----------|-----------------|
| `start` | Process shows as `running` in overmind status | `running` |
| `stop` | Process shows as `exited`/`stopped` or socket removed | `stopped` |
| `restart` | Process shows as `running` | `running` |
| `reload` | Process shows as `running` | `running` |
| `kill` | Brief wait (processes killed) | `killed` |
| `quit` | Socket file removed | `stopped` |

## Interactive Commands

Commands that produce their own output (`connect`, `echo`, `run`) don't print dots or confirmation — they go straight to their interactive/streaming output.

## Timeout Behavior

If a process doesn't reach the expected state within 5 seconds, ubermind prints `failed` and exits with a non-zero status. This catches:

- Processes that crash immediately on startup
- Services that hang during shutdown
- Commands that overmind accepts but can't execute

## Implementation

The verification uses two strategies:

1. **Status polling** (`await_process_status`): Queries `overmind status` every second to check if a process reached the target state (`running` or `exited`)
2. **Socket monitoring** (`await_socket_gone`): Checks if the `.overmind.sock` file was removed (used for `quit` and directory-based `stop`)

Each dot (`.`) represents one second of polling. The process name and final state are only printed once verification succeeds or times out.
