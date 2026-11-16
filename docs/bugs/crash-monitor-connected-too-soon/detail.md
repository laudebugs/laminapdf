# The main process connected to its crash monitor before it listened

[Resolution](resolution.md).

## What we saw

From 2026-11-16 (UTC), the full lane's Linux desktop e2e failed on `main` and on every branch that had merged it (runs 43793878336, 43794610069, 43795318563, 43797007256, 43798278690). Each time the edit-session test "closes a window with no unsaved edits without asking, and one with them once told to" opened a window with `invoice.pdf` and waited 15 s for page 1 to draw; it never did. Its retry failed with the same message, `page 1 wasn't drawn again`. Runs on macOS and Windows passed.

The app's log in the run's `desktop-e2e-linux` artefact shows two things:

- At start-up, and then every 30 s: `the main process's native crashes won't leave a dump: the crash socket could not be reached`.
- Engines started for new windows until the first crash monitor exited, 30 s after start-up. After that, each new engine logged that the crash socket couldn't be reached. Within about a minute, a new window's engine stopped reporting ready at all, and the window drew nothing. The test that failed was the first one after that point.

## How to reproduce

Start the app on Linux or macOS and read its log: the warning about the main process's crash socket appears at once, on every start. The e2e failure needs the suite's timing: a window opened after the monitor has been restarted a few times.

## Cause

[#655](https://github.com/laudebugs/laminapdf/pull/655) (D7d) added the crash monitor: the app spawns its own binary with `--crash-monitor`, then connects its crash handler to the monitor's main socket. It connected in the same call that spawned the monitor, before the new process had created its socket, so the connection failed with "could not be reached" on every start. Nothing else fits the log: the socket paths reach the monitor as arguments, so both processes use the same ones; and a monitor that failed to bind would have exited at once and used up its 5 restarts in milliseconds, not one every 30 s. D7d's own monitor test waits for the socket before connecting.

The monitor waits 30 s for the main process. With nothing connected it exits, and the supervisor restarts it, connecting too soon again, up to 5 times. So the main process's own crashes were never captured, and the engines got a monitor that died every 30 s. An engine started just after a monitor did could lose the same race, since it too connected once. The e2e failed once those restarts were under way: a window's engine stopped starting. Why a restart stops a new engine from starting wasn't found (see the resolution's last section).
