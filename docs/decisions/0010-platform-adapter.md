# 0010 — Platform adapter: one `Platform` interface, capability flags, no platform checks in UI code

- Status: Accepted (2026-10-31, when C2a's PR merges)
- Date: 2026-10-31

## Context

The same UI packages (`@lamina/ui`, `@lamina/viewer`, [ADR 0009](0009-web-client-architecture.md)) run in browsers and in the Tauri desktop app (ADR 0008, Round D), and later possibly on mobile (ADR 0025). The platforms differ in what they can do with files, windows, printing and storage ([web-frontend notes §6](../research/architecture/notes/web-frontend.md#6-offlinepwa-file-system-access-api-opfs-large-local-files-drag-and-drop); [research: the web client](../research/architecture/app-and-package-architecture.md#the-web-client-is-react-chrome-around-an-imperative-viewer-core)):

- **File pickers:** `showOpenFilePicker` / `showSaveFilePicker` are still not Baseline (Chromium only). Firefox and Safari have `<input type=file>`, drag and drop, and download-based saving. Safari 26 can write through a handle (`createWritable`) but has no pickers.
- **OPFS:** available in all three engines, including the worker-only `createSyncAccessHandle` (Firefox 111, Safari 15.2).
- **Opening from the OS:** installed PWAs get `launchQueue` file handling in Chromium only.
- **Desktop:** native dialogs, in-place saving, file watching, native printing, several windows and native menus.

If UI code asked "is this Tauri?" or sniffed the user agent, every feature would fork per platform and the web build would carry desktop paths. The research recommends a runtime adapter with capability flags (`canWriteInPlace`, `canWatchFiles`, `hasNativePrint`, `multiWindow`). The desktop todos already reference those names (D1b, D3a, D3b, D4a, D4c).

## Decision

### 1. The interface

`@lamina/ui` defines `Platform` and `Capabilities` in `src/platform/types.ts` (C4a). Each shell creates exactly one implementation at startup and passes it down through a `PlatformProvider`. The web shell uses `createWebPlatform()`; the desktop shell uses `createDesktopPlatform()` in `apps/desktop` (D2a).

```ts
export interface Platform {
  readonly capabilities: Capabilities;

  /** Ask the user for documents. Resolves to [] when the dialog is dismissed. */
  openFiles(options: { multiple: boolean }): Promise<PlatformFile[]>;
  /** Files dropped on the window or pasted (DataTransfer), normalised the same way. */
  filesFromDataTransfer(data: DataTransfer): Promise<PlatformFile[]>;
  /** Files the OS asked us to open (file handlers, "Open with", argv). Subscribe once at startup. */
  onOpenRequest(listener: (files: PlatformFile[]) => void): () => void;

  /** Save as a new file the user chooses (or a download). */
  saveAs(bytes: Uint8Array, suggestedName: string): Promise<SaveResult>;
  /** Overwrite the file `file` came from. Only when capabilities.canWriteInPlace and file.writable. */
  saveInPlace(file: PlatformFile, bytes: Uint8Array): Promise<SaveResult>;

  /** Print pages the viewer rendered, or the PDF itself where hasNativePrint. */
  print(job: PrintJob): Promise<void>;
  /** Plain text to the system clipboard (write only; lamina never reads the clipboard unasked). */
  copyText(text: string): Promise<void>;
  /** Open a URL outside the app, after the UI's own confirmation (C6b). http(s) and mailto only. */
  openExternal(url: string): Promise<void>;

  /** Durable local storage: OPFS directory, small key-value metadata, persistence request. */
  readonly storage: PlatformStorage;
  /** Recently opened documents (metadata only; reopening needs capabilities.canReopenRecent). */
  readonly recent: RecentFiles;

  /** Watch a file for changes on disk. Only when capabilities.canWatchFiles. */
  watch?(file: PlatformFile, onChange: () => void): () => void;
  /** Open a document in a new window. Only when capabilities.multiWindow. */
  openWindow?(file: PlatformFile): Promise<void>;
}

export interface PlatformFile {
  readonly name: string;
  readonly size: number;
  /** Random access for the document worker (ADR 0009 §2): no whole copy. */
  read(offset: number, length: number): Promise<Uint8Array>;
  /** What the document worker opens, sent in the message body (structured clone, never the
   *  transfer list): a File or an OPFS/picker FileSystemFileHandle; undefined for URL sources. */
  readonly handle?: File | FileSystemFileHandle;
  /** True when saveInPlace can write back to this file. */
  readonly writable: boolean;
}
```

- **What `PlatformFile` carries:** a file's name and size are shown in the UI, but its contents only ever go to the document worker. `PlatformFile` carries no path on the web; on desktop, paths stay in the Rust process (ADR 0008).
- **`SaveResult`** is `{ saved: true, name } | { saved: false, reason: "cancelled" }`.
- **`PrintJob`:** the page range, a callback that renders pages at the print DPI, and the original bytes for native PDF printing.
- **`PlatformStorage`:**
  - `opfs()`, returning a `FileSystemDirectoryHandle` or `undefined`;
  - `get(key)`, `set(key, value)` and `delete(key)` for small metadata (IndexedDB on the web);
  - `persist()`, which requests persistent storage;
  - `clear()`, used by the settings page.
- **`RecentFiles`:** `list()`, `add(file)`, `remove(id)` and `clear()`, plus `reopen(id)` when `canReopenRecent`. The entries hold the name, size, last-opened time and an opaque reopen token. Never the bytes or a URL (ADR 0011 §3).

### 2. Capability flags and their web values

UI code **branches on these flags, never on the platform's identity**. Web values are **feature-detected once at startup** (no user-agent sniffing) and don't change during a session.

| Flag | Meaning | Web value | Desktop (D-rounds) |
| --- | --- | --- | --- |
| `canWriteInPlace` | `saveInPlace` can overwrite an opened file | `true` only when `showOpenFilePicker` and `FileSystemFileHandle.prototype.createWritable` both exist (Chromium); else `false` | `true` |
| `canSaveAsDialog` | `saveAs` shows a save dialog (else it downloads) | `true` when `showSaveFilePicker` exists; else `false` (download) | `true` |
| `canWatchFiles` | `watch` reports external changes | `false` | `true` |
| `hasNativePrint` | `print` can hand the PDF to the OS print system | `false`: the web prints rendered pages from a hidden iframe (C6c) | `true` where D1b's spike proves it per OS |
| `multiWindow` | `openWindow` exists | `false` (one document view per tab; a new tab is the browser's job) | `true` |
| `hasFileHandlers` | the OS can open files in the app (`onOpenRequest` fires) | `true` when `"launchQueue" in window` **and** the app runs installed (`matchMedia("(display-mode: standalone)")` or `window-controls-overlay` matches); else `false`. `"launchQueue" in window` alone only means the browser could, so the UI uses it for the "install to open files from your computer" hint (rule 2) | `true` |
| `canReopenRecent` | `recent.reopen` can read a file again without a picker | `true` only where file handles persist in IndexedDB (Chromium, with permission); else `false` | `true` |
| `hasOpfs` | `storage.opfs()` returns a directory, and workers can use `createSyncAccessHandle` | `true` when `navigator.storage.getDirectory` exists (all current engines); else `false` | `true` (the webview's) |
| `canPersistStorage` | `storage.persist()` can make storage durable | `true` when `navigator.storage.persist` exists | `true` |
| `hasNativeMenus` | the shell draws the menu bar; the UI hides its own overflow equivalents | `false` | `true` (D4b) |
| `canOpenUrl` | documents can be opened from an `http(s)` URL (range requests, ADR 0011 §2) | `true` | `true` |
| `crossOriginIsolated` | the threaded variant may run (ADR 0009 §5) | `self.crossOriginIsolated` (`false` in v0.1, which ships no COOP/COEP) | measured in D1a |

### 3. Rules

1. **Capabilities, not platforms.** No package except the shell entry may import `@tauri-apps/*`, call `isTauri()` or read `navigator.userAgent` for behaviour. `apps/desktop` calls `isTauri()` once, in its entry (D2a). A lint check in `cargo xtask check ts` enforces this for `packages/*` (C4a).
2. **Disabled, not missing.** A control whose capability is `false` is hidden when the platform can never have it (no "watch" toggle on the web). It is disabled with an explanation when the user could change that, for example by installing the PWA or using a browser with pickers.
3. **One file contract.** Every way in (picker, drop, paste, OS open, recent, URL) produces a `PlatformFile`, and the viewer opens all of them the same way. The bytes go to the worker by range reads or a cloned `File`/handle, never through React state.
4. **Writing is gated twice.** `saveInPlace` requires both the capability and `file.writable`, and the save planner (ADR 0007) still decides the mode.
5. **Changing the interface.** A new method or flag is added to this ADR's tables in the same PR, with its web value. Desktop-only methods are optional members (`watch?`, `openWindow?`) with matching flags, so the web implementation never stubs behaviour it can't have. D2a may extend the interface this way.

## Consequences

- **The web build carries no desktop code paths**, and desktop features land by implementing the interface, not by editing components.
- **Tests:** UI tests can run against a fake `Platform` with any combination of flags, so every combination is testable in Vitest without a browser matrix (C4a).
- **Fewer features on some browsers:** Firefox and Safari users get downloads instead of in-place saves and no reopen-from-recent. The flags make that visible and explainable, not silently broken.
- **Some extra API surface:** each new platform ability needs a flag and an ADR table row, which is the price of keeping the UI single.

## Alternatives considered

- **`isTauri()` / user-agent checks in components.** The simplest at first. Rejected: platform forks spread through the UI, and desktop code ends up in the web bundle.
- **Separate UI packages per platform.** Rejected: the research's point of a shared UI (about 90 % of it) would be lost.
- **Feature detection inside each component.** Rejected: the logic gets duplicated and the answers can be inconsistent. Detection happens once, in the platform implementation.
- **Capabilities that change during a session** (re-detected live). Rejected for v0.1: installing the PWA reloads the app anyway, and fixed flags keep rendering predictable.
- **Passing paths or URLs around the UI.** Rejected: paths are a desktop security boundary (ADR 0008) and document URLs are private (ADR 0011). `PlatformFile` hides both.
