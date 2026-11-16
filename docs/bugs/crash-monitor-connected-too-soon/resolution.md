# Resolution: the main process connected to its crash monitor before it listened

[Diagnosis](detail.md).

## The fix

- `laminapdf-desktop-crash` has `capture_crashes_within(socket, server, wait)`: it tries again every 20 ms while the socket can't be reached, until `wait` has passed. Any other failure returns at once.
- The app connects its crash handler with it, waiting up to 5 s for a monitor it has just spawned. The first connection now runs on the monitor's supervisor thread rather than during start-up, so the app's start never waits for the monitor.

With the main process connected, the first monitor stays up for the whole run, and it restarts only if it crashes.

## How it was checked

- `a_capture_should_wait_for_a_server_that_starts_late`: the server starts 200 ms after the client begins, and the capture succeeds. `a_capture_should_give_up_once_its_wait_has_passed`: with no server, it fails with `Connect` after its wait.
- The app's crash tests (`cargo test -p laminapdf-desktop --lib crash`) pass.
- The full lane's Linux desktop e2e on this PR runs the suite that failed.

## Left for later

Why a window's engine stops starting while the monitor is being restarted is not known. Once the restarts began, new engines logged that the crash socket couldn't be reached and still started, and then, about a minute later, one didn't start at all. This fix keeps the restarts from happening at start-up, but a monitor that does crash and restart could still hit the stall. That needs a Linux run that crashes the monitor on purpose and records what the stalled engine is doing.
