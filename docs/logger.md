# Logger Module

The `logger` module initialises the `env_logger` logging backend at startup. It exposes a single `init` function called once from `main`, before any other subsystem is started.

## Key Characteristics

| Property | Value |
|---|---|
| Backend | `env_logger` (wraps the `log` facade) |
| Level source | `--log-level` CLI argument |
| Default level | `off` |
| Output | Console (stdout) by default; file if `--log-file` is provided |
| File mode | Append — existing log files are never truncated |

## Log Format

Each line follows this pattern:

```
<RFC3339 timestamp> [<thread id>] <LEVEL> [<target>] - <message>
```

Example:

```
2026-05-30T08:00:01.234+03:00 [ThreadId(2)] DEBUG [jarvis_core::api::chat] - prompt: what is the weather?
```

## Behaviour on File Error

If the specified log file cannot be opened, the error is printed to `stderr` and logging silently falls back to the console. The application continues to run normally.
