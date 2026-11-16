# Resolution: the main process connected to its crash monitor before it listened

[Diagnosis](detail.md).

## The fix

- `laminapdf-desktop-crash` has `capture_crashes_within(socket, server, wait, cancel)`: it tries again every 20 ms while the socket can't be reached, until `wait` has passed or `cancel` is set. Any other failure returns at once.
- The app connects its crash handler with it, waiting up to 25 s, a little under the monitor's own 30 s wait, for a monitor it has just spawned. The connection runs on the monitor's supervisor thread rather than during start-up, so the app's start never waits for the monitor. On Linux the handler's alternate signal stack is that thread's; the main thread keeps the standard library's.
- A shutdown cancels the wait and drops a handler that connects anyway, and ends a monitor nothing connected to rather than waiting out its 30 s. Otherwise a handler installed after shutdown cleared it would keep the monitor waiting for the main process while shutdown waited for the monitor.
- Engines are given the monitor's socket only once the main process has connected to it, which means its engines' socket, bound first, is listening; until then they start without capture.

With the main process connected, the first monitor stays up for the whole run, and it restarts only if it crashes.

## How it was checked

- `a_capture_should_wait_for_a_server_that_starts_late`: the server starts 200 ms after the client begins, and the capture succeeds. `a_capture_should_give_up_once_its_wait_has_passed` and `a_cancelled_capture_should_stop_waiting`: with no server, it fails with `Connect` after its wait, or at once when cancelled.
- The app's crash tests (`cargo test -p laminapdf-desktop --lib crash`) pass.
- The full lane's Linux desktop e2e on this PR runs the suite that failed; its log should no longer show the main process's warning.

## Left for later

Why a window's engine stops starting while the monitor is being restarted is not known. Once the restarts began, new engines logged that the crash socket couldn't be reached and still started, and then, about a minute later, one didn't start at all. This fix keeps the restarts from happening at start-up, but a monitor that does crash and restart could still hit the stall. That needs a Linux run that crashes the monitor on purpose and records what the stalled engine is doing.
