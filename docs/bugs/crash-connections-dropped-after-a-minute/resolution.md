# Resolution: crash connections dropped after a minute

[Diagnosis](detail.md).

## The fix

- **No stale timeout.** `serve_loop` in `laminapdf-desktop-crash` runs minidumper's loop with none, so a quiet connection stays open until its process exits.
- **Arguments read at each start.** #669 fixed this on main first: its `FreshArgs` spawner (`lib.rs`) reads the crash socket through `spawn_args` at each spawn, a supervisor's restart included. This PR took that fix and dropped its own, which did the same.
- **Failed starts logged.** `Engines::engine` logs the supervisor's error, `the document process could not start: …`, as a warning before returning it to the window.

## How it was checked

- The crash crate's tests, among them `a_one_client_server_should_stop_when_its_client_disconnects`, still pass: with no stale timeout, a client that dies still closes its socket, and the monitor still ends with the main process.

## Left alone

- **A test that a quiet connection outlives a minute.** It would wait more than 60 s; the change is one argument, and the comment beside it says why it must stay `None`.
- **The monitor's 30 s restarts at start-up.** That is #669's bug, the main process connecting before the monitor listens; #669 fixed it.
- **A test that every start reads the arguments.** `FreshArgs` needs a running app to read the monitor's socket; this PR's own version had such a test, and went with the version it dropped.
