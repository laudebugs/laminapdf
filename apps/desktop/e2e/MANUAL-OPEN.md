# Opening from the OS: manual checks

The e2e runs cover a second launch with a path and a cold start with a path, on all three OSes. Check by hand, once per OS with a dev bundle, what WebDriver can't drive: Finder, Explorer, a file manager, the Dock and a real drag. Record each run in the PR that changes how files open: the OS and version, the bundle, and each row's result.

## Build a dev bundle

```sh
cargo xtask desktop-frontend
pnpm --filter @laminapdf/desktop exec tauri build --debug --config src-tauri/tauri.bundle.conf.json --bundles app   # macOS: target/debug/bundle/macos/Lamina PDF.app
pnpm --filter @laminapdf/desktop exec tauri build --debug --config src-tauri/tauri.bundle.conf.json --bundles nsis  # Windows
pnpm --filter @laminapdf/desktop exec tauri build --debug --config src-tauri/tauri.bundle.conf.json --bundles deb   # Linux
```

Install it (drag the app to Applications, run the installer, or `sudo apt install ./….deb`). On macOS, `/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "<app>"` registers a bundle that isn't in Applications.

## Checks

Use three files: one plain (`fixtures/golden/examples/invoice.pdf`), one whose name has a space and non-ASCII letters (`Relevé de compte 2026.pdf`), and one that isn't a PDF (`notes.txt`). On Windows, also put a copy under a folder path longer than 260 characters, and one on a network share.

| # | Do | Expect |
| --- | --- | --- |
| 1 | With the app closed, choose Lamina PDF in "Open with" for the plain PDF. | The app starts and shows the file. |
| 2 | With the app running, open `Relevé de compte 2026.pdf` with "Open with" → Lamina PDF. | The running window comes to the front and shows it. The process list holds one Lamina PDF (plus its engine). |
| 3 | Drop the plain PDF onto the window, on the home screen and over an open document. | "Drop to open the PDF" shows while dragging; the drop opens the file, and no failure page or second message follows (the webview's own drop handler must not fire too). |
| 4 | Drop `notes.txt` onto the window. | "Only PDF files open in Lamina PDF" shows while dragging; a toast names `notes.txt`; nothing opens. |
| 5 | Drop two PDFs at once. | The overlay says it opens the last of them, and it does. |
| 6 | macOS: drop the PDF onto the Dock icon, with the app running and with it closed. | The file opens. |
| 7 | macOS: `open -a "Lamina PDF" "Relevé de compte 2026.pdf"` with the app running. | The running window shows it. |
| 8 | Windows: pin the app to the taskbar, then "Open with" from Explorer. | The running window shows it. |
| 9 | Windows: open the long-path copy and the network-share copy. | Both open. |
| 10 | With the app running, start its binary again with the path of a PDF that doesn't exist. | A toast names the file; the window keeps what it shows. |
| 11 | Check the default PDF app in the OS's settings after installing. | It is unchanged: Lamina PDF is listed as able to open PDFs but isn't the default. (Windows: on a machine where no default was ever chosen, Tauri's installers still make it the default until D6d changes them; record what you see.) |
