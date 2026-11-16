# Crash connections dropped after a minute

[Resolution](resolution.md).

## What we saw

While main's Linux desktop e2e was failing (2026-11-16), the app's log showed the crash monitor starting again and again, each start logging the main process's `the crash socket could not be reached`. Most of those restarts came every 30 s from another bug, which #669 fixes: the main process connected before the monitor listened, so the monitor gave up waiting for it. Reading the code for why showed three defects of D7d's own: connections dropped after a minute, which restarts the monitor too; engines started after a restart given the old monitor's socket; and nothing in the log saying whether a window's engine failed to start.

The e2e failure itself, in the edit session's new window, has another cause: the test's click never reaches that window. The test is reworked in its own PR. These three defects showed up while looking into it.

## How to reproduce

Run the desktop app on Linux or macOS for more than a minute with `RUST_LOG=info`. The monitor exits and is started again, and an engine that crashes after that leaves no minidump.

## Cause

Three defects from D7d (#655):

- **A 60-second stale timeout.** The monitor's dump servers ran minidumper's loop with `stale_timeout` set to 60 s. A client sends nothing until it crashes, so every connection counted as stale after a minute and was dropped. The engines' capture ended without a word. The main process's server then had no client left, so the monitor exited and the app started another, on a new socket.
- **Engine arguments read once.** `engines()` in `lib.rs` built the engines' arguments when the app started, and each window's supervisor kept its spawner, arguments and all, for every restart of its engine. An engine started after a monitor restart was given the old socket. The doc comment of `EngineDumps::engine_args` says they are read live.
- **A silent failed start.** When an engine sent no `Ready` within 10 s, the supervisor's error went to the window and nowhere else.
