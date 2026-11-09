// The Platform interface of ADR 0010: everything the reader asks of the shell it runs in (files,
// saving, printing, the clipboard, storage), behind capability flags. UI code branches on the
// flags, never on which platform it is; each shell creates one implementation at startup.

/** What this platform can do, detected once at startup (ADR 0010 §2). */
export interface Capabilities {
  /** `saveInPlace` can overwrite an opened file. */
  readonly canWriteInPlace: boolean;
  /** `saveAs` shows a save dialog; otherwise it downloads. */
  readonly canSaveAsDialog: boolean;
  /** `watch` reports external changes. */
  readonly canWatchFiles: boolean;
  /** `print` can hand the PDF to the OS print system. */
  readonly hasNativePrint: boolean;
  /** `openWindow` exists. */
  readonly multiWindow: boolean;
  /** The OS can open files in the app (`onOpenRequest` fires). */
  readonly hasFileHandlers: boolean;
  /** `recent.reopen` can read a file again without a picker. */
  readonly canReopenRecent: boolean;
  /** `storage.opfs()` returns a directory, and workers can use sync access handles. */
  readonly hasOpfs: boolean;
  /** `storage.persist()` can make storage durable. */
  readonly canPersistStorage: boolean;
  /** The shell draws the menu bar; the UI hides its own overflow equivalents. */
  readonly hasNativeMenus: boolean;
  /** Documents can be opened from an http(s) URL. */
  readonly canOpenUrl: boolean;
  /** The threaded variant may run. */
  readonly crossOriginIsolated: boolean;
  /** No browser sits around the app, so it may bind keys a browser keeps for itself. */
  readonly ownsKeyboard: boolean;
  /** `shareText` hands text to the system's share sheet. */
  readonly canShare: boolean;
}

/** One of the capability flags. */
export type Capability = keyof Capabilities;

/** A document the user chose, dropped or opened from the OS: a name, a size and range reads. */
export interface PlatformFile {
  readonly name: string;
  readonly size: number;
  /** Random access for the document worker: no whole copy. */
  read(offset: number, length: number): Promise<Uint8Array>;
  /**
   * What the document worker opens, sent in the message body (structured clone, never the
   * transfer list): a File or a picker's FileSystemFileHandle; undefined for URL sources.
   */
  readonly handle?: File | FileSystemFileHandle;
  /** True when saveInPlace can write back to this file. */
  readonly writable: boolean;
}

/** How a save ended. */
export type SaveResult =
  | { readonly saved: true; readonly name: string }
  | { readonly saved: false; readonly reason: "cancelled" };

/** A print request: the pages, how to render them at the print resolution, and the original bytes. */
export interface PrintJob {
  /** Page indices, 0-based. */
  readonly pages: readonly number[];
  /** Renders one page at `dpi` for printing. */
  render(page: number, dpi: number): Promise<ImageBitmap>;
  /** The PDF itself, for platforms that print it natively. */
  readonly bytes?: Uint8Array;
}

/** Durable local storage: an OPFS directory, small key-value metadata, a persistence request. */
export interface PlatformStorage {
  /** The origin-private file system's root, or undefined where there is none. */
  opfs(): Promise<FileSystemDirectoryHandle | undefined>;
  /** The value stored under `key`, or undefined when there is none. */
  get(key: string): Promise<unknown>;
  /** Stores `value` under `key`; resolves once it is saved. */
  set(key: string, value: unknown): Promise<void>;
  /** Removes the value under `key`, if any. */
  delete(key: string): Promise<void>;
  /** Asks for persistent storage; resolves to whether it was granted. */
  persist(): Promise<boolean>;
  /** Removes everything this platform stored (the settings page's "clear"). */
  clear(): Promise<void>;
}

/** A recently opened document: metadata only, never its bytes or a URL (ADR 0011 §3). */
export interface RecentFile {
  readonly id: string;
  readonly name: string;
  readonly size: number;
  /** When it was last opened, in milliseconds since the epoch. */
  readonly lastOpened: number;
}

/** The recently opened documents. */
export interface RecentFiles {
  /** Newest first. */
  list(): Promise<readonly RecentFile[]>;
  /** Puts `file` first, replacing an older entry for the same document. */
  add(file: PlatformFile): Promise<void>;
  /** Forgets one document. */
  remove(id: string): Promise<void>;
  /** Forgets every document. */
  clear(): Promise<void>;
  /** Reads a recent document again without a picker. Only when capabilities.canReopenRecent. */
  reopen?(id: string): Promise<PlatformFile | undefined>;
}

/** Everything the reader asks of the shell it runs in (ADR 0010 §1). */
export interface Platform {
  readonly capabilities: Capabilities;
  /**
   * Why a false flag could become true here ("Install Lamina PDF to open files from your
   * computer"), or undefined when this platform can never have it. Rule 2 hides or disables by it.
   */
  howToEnable(flag: Capability): string | undefined;

  /** Asks the user for documents. Resolves to [] when the dialog is dismissed. */
  openFiles(options: { readonly multiple: boolean }): Promise<PlatformFile[]>;
  /** Files dropped on the window or pasted, normalised the same way. */
  filesFromDataTransfer(data: DataTransfer): Promise<PlatformFile[]>;
  /** Files the OS asked us to open. Subscribe once at startup. */
  onOpenRequest(listener: (files: PlatformFile[]) => void): () => void;

  /** Saves as a new file the user chooses, or as a download. */
  saveAs(bytes: Uint8Array, suggestedName: string): Promise<SaveResult>;
  /** Overwrites the file `file` came from. Only when capabilities.canWriteInPlace and file.writable. */
  saveInPlace(file: PlatformFile, bytes: Uint8Array): Promise<SaveResult>;

  /** Prints pages the viewer rendered, or the PDF itself where hasNativePrint. */
  print(job: PrintJob): Promise<void>;
  /** Plain text to the system clipboard (write only). */
  copyText(text: string): Promise<void>;
  /** Hands `text` to the system's share sheet (where `canShare`); rejects when the user cancels. */
  shareText(text: string): Promise<void>;
  /** Opens a URL outside the app, after the UI's own confirmation. http(s) and mailto only. */
  openExternal(url: string): Promise<void>;

  readonly storage: PlatformStorage;
  readonly recent: RecentFiles;

  /** Watches a file for changes on disk. Only when capabilities.canWatchFiles. */
  watch?(file: PlatformFile, onChange: () => void): () => void;
  /** Opens a document in a new window. Only when capabilities.multiWindow. */
  openWindow?(file: PlatformFile): Promise<void>;
}
