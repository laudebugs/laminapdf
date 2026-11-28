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
  /** The shell takes files dropped on the window and opens them itself; the UI's drop zone leaves drops alone. */
  readonly shellTakesDrops: boolean;
  /** `chooseFolder` lets the user pick a folder to write several files into. */
  readonly canWriteDirectory: boolean;
  /** `signatures` keeps the user's signature images on this device. */
  readonly signatureLibrary: boolean;
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

/** A folder the user chose to write files into (`chooseFolder`). */
export interface PlatformFolder {
  /** Its name, as the user sees it. */
  readonly name: string;
  /** Which of `names` are files in it already. */
  existing(names: readonly string[]): Promise<string[]>;
  /** Writes `bytes` to the file `name` in it, replacing a file of that name. */
  write(name: string, bytes: Uint8Array): Promise<void>;
}

/** How a save ended. */
export type SaveResult =
  | {
      readonly saved: true;
      readonly name: string;
      /**
       * The file as saved, to read and to save to next: read again after a write in place (the
       * file read before is stale), the new file after a save dialog; none after a download.
       */
      readonly file?: PlatformFile;
    }
  | { readonly saved: false; readonly reason: "cancelled" };

/** One page drawn for printing: an image, and the size it prints at in points. */
export interface PrintPage {
  /** The page as an image (PNG or JPEG). */
  readonly image: Blob;
  /** Its size in points, rotation applied. */
  readonly width: number;
  readonly height: number;
}

/** A print request: the pages, how to draw each for printing, and the original bytes. */
export interface PrintJob {
  /** Page indices, 0-based, in the order they print. */
  readonly pages: readonly number[];
  /** Draws one page for printing; called one page at a time, so one page's pixels are held at once. */
  render(page: number, signal: AbortSignal): Promise<PrintPage>;
  /** `fit` scales each page to the paper; `actual` prints it at its own size. */
  readonly scale: "fit" | "actual";
  /** Told after each page is ready; the last call (`ready === total`) comes as the print window opens. */
  readonly onProgress?: (ready: number, total: number) => void;
  /** Stops preparing: no print window opens, and what was drawn is let go. */
  readonly signal?: AbortSignal;
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

/**
 * A signature image the user keeps on this device: a drawn, typed or uploaded picture, never a
 * certificate or a key (the E10 plan's §6 lists where each lives). Nothing in it leaves the device.
 */
export interface SavedSignature {
  readonly id: string;
  /** What the user calls it: "My signature". */
  readonly name: string;
  /** How it was made. */
  readonly kind: "drawn" | "typed" | "image";
  /** The picture: a PNG or JPEG's bytes. */
  readonly bytes: Uint8Array;
  /** `image/png` or `image/jpeg`. */
  readonly type: "image/png" | "image/jpeg";
  /** The picture's size in pixels, which fixes its proportions. */
  readonly width: number;
  readonly height: number;
  /** The strokes of a drawn signature, `[x, y]` points in pixels, so it can be placed as ink. */
  readonly strokes?: readonly (readonly (readonly [number, number])[])[];
  /** The ink of a drawn or typed signature, RGB 0 to 1: placed as ink, it keeps this colour. */
  readonly ink?: readonly [number, number, number];
  /** When it was added, in milliseconds since the epoch: the list's order. */
  readonly added: number;
}

/** The signature images kept on this device (only when `capabilities.signatureLibrary`). */
export interface SignatureLibrary {
  /** Every signature, newest first. */
  list(): Promise<SavedSignature[]>;
  /** Adds `signature`, or replaces the one with its ID. */
  put(signature: SavedSignature): Promise<void>;
  /** Removes the signature `id`, if any. */
  remove(id: string): Promise<void>;
  /** Removes every signature. */
  clear(): Promise<void>;
}

/** A recently opened document: metadata only, never its bytes or a URL (ADR 0011 §3). */
export interface RecentFile {
  readonly id: string;
  readonly name: string;
  readonly size: number;
  /** When it was last opened, in milliseconds since the epoch. */
  readonly lastOpened: number;
  /** Its page count, when it was known at the last open. */
  readonly pages?: number | undefined;
  /** A handle to the file was kept, so `reopen` may read it again without a picker. */
  readonly hasHandle?: boolean | undefined;
  /** The document's fingerprint, the key its reading position is kept under (`Platform.positions`). */
  readonly fingerprint?: string | undefined;
  /** The file is no longer where it was (the desktop checks): it can't reopen, only be removed. */
  readonly missing?: boolean | undefined;
}

/** What `RecentFiles.add` keeps besides the file's name and size. */
export interface RecentDetails {
  /** The page count. */
  readonly pages?: number | undefined;
  /** The document's fingerprint. */
  readonly fingerprint?: string | undefined;
}

/** The recently opened documents. */
export interface RecentFiles {
  /** Newest first. */
  list(): Promise<readonly RecentFile[]>;
  /** Puts `file` first, replacing an older entry for the same document; gives the entry's id. */
  add(file: PlatformFile, details?: RecentDetails): Promise<string>;
  /** Forgets one document. */
  remove(id: string): Promise<void>;
  /** Forgets every document. */
  clear(): Promise<void>;
  /**
   * Reads a recent document again without a picker. Only when capabilities.canReopenRecent. It asks
   * for permission when the browser no longer holds it, which needs a click; with `ask: false`
   * (at start-up, with no click) it resolves undefined instead of asking.
   */
  reopen?(id: string, options?: { readonly ask?: boolean }): Promise<PlatformFile | undefined>;
}

/**
 * Where a reader stopped in one document: the view (the viewer's position, opaque to this package:
 * the app checks it on read), the page count when it was kept, and when, in milliseconds since the
 * epoch. Never a name, path, address, bytes or text (ADR 0011 §3 guarantee 9).
 */
export interface ReadingPosition {
  readonly view: unknown;
  readonly pageCount: number;
  readonly readAt: number;
}

/** Reading positions by document fingerprint, kept on this device only; the oldest go past a cap. */
export interface ReadingPositions {
  /** The position kept for the document with `fingerprint`, or undefined when there is none. */
  get(fingerprint: string): Promise<ReadingPosition | undefined>;
  /** Keeps `entry` for the document with `fingerprint`, replacing its last; the oldest go past the cap. */
  set(fingerprint: string, entry: ReadingPosition): Promise<void>;
  /** Forgets the position of the document with `fingerprint`. */
  remove(fingerprint: string): Promise<void>;
  /** Forgets every position ("Forget reading positions"). */
  clear(): Promise<void>;
}

/** Everything the reader asks of the shell it runs in (ADR 0010 §1). */
export interface Platform {
  readonly capabilities: Capabilities;
  /**
   * Why a false flag could become true here ("Install Lamina PDF to open files from your
   * computer"), or undefined when this platform can never have it. Rule 2 hides or disables by it.
   */
  howToEnable(flag: Capability): string | undefined;

  /**
   * Asks the user for documents, with `accept: "images"` for pictures (PNG and JPEG), or with
   * `accept: "any"` for files of any type (a file to attach). Resolves to [] when the dialog is
   * dismissed.
   */
  openFiles(options: {
    readonly multiple: boolean;
    readonly accept?: "pdf" | "images" | "any";
  }): Promise<PlatformFile[]>;
  /** Files dropped on the window or pasted, normalised the same way. */
  filesFromDataTransfer(data: DataTransfer): Promise<PlatformFile[]>;
  /** Files the OS asked us to open. Subscribe once at startup. */
  onOpenRequest(listener: (files: PlatformFile[]) => void): () => void;

  /**
   * Saves as a new file the user chooses, or as a download: a PDF, or a zip of several when the
   * name ends in `.zip` (the parts of a split where no folder can be chosen). `bytes` may still be
   * coming: a save dialog opens first, while the click that asked for it lets the browser show it.
   */
  saveAs(bytes: Uint8Array | Promise<Uint8Array>, suggestedName: string): Promise<SaveResult>;
  /**
   * Saves an export (pictures of pages, text, JSON, CSV, or a zip of them) as a new file: a save
   * dialog typed by the name's extension where the platform has one, else a download. Nothing
   * reopens it. Takes a `Blob`, so a large zip need not sit in one buffer. Optional: a platform
   * without it doesn't offer exports.
   */
  saveExport?(file: Blob | Promise<Blob>, suggestedName: string): Promise<SaveResult>;
  /** Asks the user for a folder to write files into; undefined when they cancel. Only when capabilities.canWriteDirectory. */
  chooseFolder?(): Promise<PlatformFolder | undefined>;
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
  /** Where the reader stopped in each document, by its fingerprint. */
  readonly positions: ReadingPositions;
  /** The user's signature images, kept on this device. Only when `capabilities.signatureLibrary`. */
  readonly signatures: SignatureLibrary;

  /** Watches a file for changes on disk. Only when capabilities.canWatchFiles. */
  watch?(file: PlatformFile, onChange: () => void): () => void;
  /** Opens a document in a new window. Only when capabilities.multiWindow. */
  openWindow?(file: PlatformFile): Promise<void>;
}
