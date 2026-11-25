# Changelog: the Lamina PDF desktop app

The desktop reader for Windows and Linux. Each release is tagged `desktop-v<version>`, and its notes page at `laminapdf.com/desktop/releases/<version>`, which the app's What's new opens, carries the same account. macOS follows in a later release, and the Mac App Store build is a separate install.

## Unreleased

- **Step back and forward through reading history** with Go › Back and Go › Forward, the toolbar buttons, the keyboard shortcuts (Alt+Left/Right; ⌘[/⌘] on macOS) or mouse Back/Forward buttons. Each tab keeps its own history across tab switches, recording page jumps such as Go to page (links and the outline will add entries once they arrive).

## 0.1.0

The first beta of the desktop app, for Windows and Linux. It opens PDFs on your own computer: nothing you open is uploaded, and the app works offline.

### What it does

- **Read** PDFs in tabs and windows, on your own computer and offline.
- **Open** a PDF the way your system does: double-click it or choose "Open with". The start window lists your recent files; its Clear list and Settings › Files › Clear recent files empty that list and the system's own.
- **Organise pages** in a grid: move, rotate, duplicate and delete them, then save the file in place.
- **Edit the document's** title, author, subject and keywords.
- **Update itself** once a day from the beta channel; Settings turns the check off.
- **Report crashes only if you agree**: crash reports are off until you turn them on in Settings › Privacy.

### Systems

Windows 11 on x64 or Arm, and Ubuntu 22.04 or later or a supported Fedora, on x64 or Arm. Windows 10 22H2 is untested.
