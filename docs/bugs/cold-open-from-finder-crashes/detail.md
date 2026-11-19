# Opening a PDF from Finder crashed the app when it wasn't running

## What the user saw

On macOS, opening a PDF with Lamina PDF while the app was closed (double-click, "Open with", a drop on the Dock icon, or `open -a "Lamina PDF" file.pdf`) quit the app at once. No window appeared, and macOS showed its "quit unexpectedly" report. With the app already running, the same file opened as it should. Release 0.6.0 does this, and so does a build of `main` on 2026-11-19.

## How to reproduce it

1. Quit Lamina PDF.
2. Run `open --stderr err.txt -a "/Applications/Lamina PDF.app" some.pdf`.
3. `err.txt` holds:

```
thread 'main' panicked at …/tauri-2.12.5/src/lib.rs:761:7:
state() called before manage() for laminapdf_desktop_lib::windows::Windows
libc++abi: terminating due to uncaught foreign exception
```

macOS's crash report (`~/Library/Logs/DiagnosticReports/laminapdf-desktop-*.ips`) shows `SIGABRT` from `objc_exception_rethrow` inside `-[NSAppleEventManager dispatchRawAppleEvent:…]`: the panic unwound into AppKit's handler for the "open documents" Apple event, which caught it as a foreign exception and ended the process.

Starting the binary itself with the path (`…/MacOS/laminapdf-desktop some.pdf`) works, because that path comes in as an argument, not an Apple event. The e2e suite's cold start passes its path that way, so it never met this.

## The cause

macOS delivers a cold start's files as an "open documents" Apple event, which Tauri turns into `RunEvent::Opened`, and that event comes before the app's `setup` hook has run. The handler called `windows::target_window`, which reads the `Windows` state; `setup` manages that state, so it didn't exist yet, and `app.state()` panicked.

The handler had worked when it was written (#476): `target_window` then didn't read any managed state. The multiple-windows work (#529) made it read `Windows`. `route_open` behind it also needs the preferences and the engines, which `setup` manages too, so the files can't be routed anywhere until `setup` has run.
