# Resolution: crash connections dropped after a minute

[Diagnosis](detail.md).

## The fix

- **No stale timeout.** `serve_loop` in `laminapdf-desktop-crash` runs minidumper's loop with none, so a quiet connection stays open until its process exits.
- **Arguments read at each launch.** The engines' launcher calls `engine_args` each time it starts an engine, so a new engine gets the current monitor's socket.
- **Failed starts logged.** `Engines::engine` logs a warning, `the document process could not start`, before returning the error to the window. An open that shows the file's tab in another window instead (`AlreadyOpen`) is logged too.

## How it was checked

The crash crate's tests and clippy pass. In CI's Linux desktop e2e on #671, the monitor no longer restarts after a minute. The edit session's new-window test still failed there, for the separate reason the [diagnosis](detail.md) gives.
