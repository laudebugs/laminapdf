# Changelog: the Lamina PDF desktop app

The desktop reader for Windows and Linux. Each release is tagged `desktop-v<version>`, and its notes page at `laminapdf.com/desktop/releases/<version>`, which the app's What's new opens, carries the same account. macOS follows in a later release, and the Mac App Store build is a separate install.

## Unreleased

- **Insert pages from another PDF, or replace pages with its pages**, in Organise from the Insert menu. Pick the file in your system's dialog, see its pages, choose which to bring in and where they go, and the change is one step you can undo. Password-protected files ask for their password first.
- **Read in focus mode** from View › Focus mode or with Ctrl+\ (⌘\ on macOS). Focus mode hides the secondary toolbar and open panels, shows a Focus mode button in the status bar to leave it, and turns off when you press Escape or close the last tab.
- **Step back and forward through reading history** with Go › Back and Go › Forward, the toolbar buttons, mouse Back/Forward buttons, or the keyboard shortcuts (Alt+Left/Right; ⌘[/⌘] on macOS). Each tab keeps its own history across tab switches, recording page jumps such as Go to page; links and the outline will add entries once they arrive.
- **Choose how the reader looks** from the View settings popover, Settings › Appearance and accessibility or the View menu. Pick a theme (Light, Dark, High contrast or Match the system), an accent colour (Teal, Blue, Plum or Green) and a page appearance (Original, Sepia, Dark or High contrast). Every open window and tab updates at once, and the app remembers your choices across launches.

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
