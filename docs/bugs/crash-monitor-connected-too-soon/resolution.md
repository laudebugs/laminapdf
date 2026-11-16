# Resolution: the main process connected to its crash monitor before it listened

[Diagnosis](detail.md).

## The fix

- `laminapdf-desktop-crash` has `capture_crashes_within(socket, server, wait)`: it tries again every 20 ms while the socket can't be reached, until `wait` has passed. Any other failure returns at once.
- The app connects its crash handler with it, waiting up to 25 s, a little under the monitor's own 30 s wait, for a monitor it has just spawned. The connection runs on the monitor's supervisor thread rather than during start-up, so the app's start never waits for the monitor. On Linux the handler's alternate signal stack is that thread's; the main thread keeps the standard library's.
- Each engine's crash arguments are read as it spawns, so an engine gets the monitor that is running then, and none once the monitor is down for good. The engine waits up to 2 s for a monitor that is still starting.

With the main process connected, the first monitor stays up for the whole run, and it restarts only if it crashes.

## How it was checked

- `a_capture_should_wait_for_a_server_that_starts_late`: the server starts 200 ms after the client begins, and the capture succeeds. `a_capture_should_give_up_once_its_wait_has_passed`: with no server, it fails with `Connect` after its wait.
- The app's, the crash crate's and the engine's tests pass.
- The full lane's Linux desktop e2e on this PR runs the suite that failed; its log should no longer show the main process's warning.

## Left for later

- The monitor is never dropped while the app runs (its supervisor thread holds it, and Tauri doesn't drop managed state at exit), so `Monitor`'s `Drop` doesn't run; the monitor ends when the app's process does. This fix doesn't change that.
- Why a window's engine stopped starting while the monitor was being restarted is not known. Once the restarts began, new engines logged that the crash socket couldn't be reached and still started, and then, about a minute later, one didn't start at all. This fix keeps the restarts from happening at start-up, but a monitor that does crash and restart could still hit the stall. That needs a Linux run that crashes the monitor on purpose and records what the stalled engine is doing.
- If the main process can't connect within 25 s, it carries on without capture, and the monitor exits and is restarted as before; engines still get its socket meanwhile.
