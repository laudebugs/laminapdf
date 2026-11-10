// The reader's engine: one document worker and the view over it (a session), the controller that
// runs the reader's lifecycle, and what the chrome shows about the open document. The backend's
// WebAssembly starts loading as soon as the app does, so a file chosen meanwhile waits only for
// what is left. Switching backend (the #debug menu) starts a new session and reopens the file.
// A remote file (`#url=`) is a PlatformFile that names its address: the document worker fetches
// it, so its bytes never reach this page.

import pdfiumWasm from "@embedpdf/pdfium/pdfium.wasm?url";
import type {
  DebugBackend,
  ExternalStore,
  OpenedDocument,
  Platform,
  PlatformFile,
  RemoteFailureReason,
  RepairKind,
} from "@laminapdf/ui";
import { message, ReaderController } from "@laminapdf/ui";
import {
  createViewer,
  type LinkLayerOptions,
  type LinkWords,
  type OpenableLink,
  type PrintedPage,
  type PrintOptions,
  type RenderedBitmap,
  type SourceInit,
  ThumbnailStrip,
  type Viewer,
  ViewerClient,
} from "@laminapdf/viewer";
import { spokenPage } from "./announce.tsx";
import { type DocumentWarning, repairsFrom } from "./document.ts";
import type { NetworkMonitor } from "./network/monitor.ts";
import { pageAddress } from "./open/history.ts";
import type { FragmentPlace } from "./open/navigation.ts";
import { type OutlineModel, outlineModel, type PageInfo } from "./outline.ts";
import { type KeptView, keptPosition, recordPosition } from "./reading/positions.ts";
import { readChoices } from "./storage-choices.ts";

/** A document's properties, as the document worker reads them. */
export type DocumentProperties = Awaited<ReturnType<ViewerClient["properties"]>>;

/** A document for reader mode, as the document worker reads it. */
export type DocumentReflow = Awaited<ReturnType<ViewerClient["reflow"]>>;

/** One backend's document worker and the view over it. */
interface Session {
  readonly backend: DebugBackend;
  readonly client: ViewerClient;
  readonly viewer: Viewer;
  /** The rail's Pages tab. */
  readonly strip: ThumbnailStrip;
}

/** What the chrome shows besides the reader's lifecycle. */
export interface EngineState {
  readonly backend: DebugBackend;
  /** The view, once its backend has loaded. */
  readonly viewer: Viewer | undefined;
  /** The rail's thumbnails of the same pages. */
  readonly strip: ThumbnailStrip | undefined;
  /** The open document's pages (sizes and labels), empty while none is open. */
  readonly pages: readonly PageInfo[];
  /** The open document's outline once the Outline tab asked for it (`loadOutline`). */
  readonly outline: OutlineModel | "loading" | "failed" | undefined;
  /** The open document's metadata title, when it has one. */
  readonly title: string | undefined;
  /** The open document's repairs, one per warning code. */
  readonly repairs: readonly RepairKind[];
  /** For #debug: the backend's name and version, and how long the open took to paint page 1. */
  readonly timing: { readonly backend: string; readonly firstPageMs: number } | undefined;
  /** Why the address's `#url=` link can't be opened, until another document opens or the reader closes. */
  readonly linkProblem: Exclude<RemoteFailureReason, "cors"> | undefined;
  /** A web or email link in the document waiting for the reader to confirm it (R07). */
  readonly externalLink: OpenableLink | undefined;
  /** The open document's fingerprint, the key of its reading position. */
  readonly fingerprint: string | undefined;
  /** The page (0-based) the open document came back at, where its reader stopped last time. */
  readonly resumedAt: number | undefined;
}

/** #debug's log of a link the layer never follows (R07: actions are never run). */
const logInert: NonNullable<LinkLayerOptions["onInert"]> = (link, note) =>
  console.debug("link not followed:", link.kind, note);

/** The link layer's words, from the catalogue. */
const LINK_WORDS: LinkWords = {
  page: (page) => message("link.page", { page: page + 1 }),
  nextPage: message("link.nextPage"),
  previousPage: message("link.previousPage"),
  firstPage: message("link.firstPage"),
  lastPage: message("link.lastPage"),
  otherFile: (file) => (file === undefined ? message("link.otherFile.unnamed") : message("link.otherFile", { file })),
  refused: (uri) => message("link.refused", { uri }),
  unsupported: message("link.unsupported"),
  broken: message("link.broken"),
};

/**
 * Starts `backend`'s document worker and makes a view over it, whose web links `onExternal`
 * receives for confirming; `debug` (#debug) outlines text spans and logs links never followed.
 */
async function startSession(
  backend: DebugBackend,
  debug: boolean,
  onExternal: (link: OpenableLink) => void,
  pageName: (index: number, count: number) => string,
): Promise<Session> {
  const client = await ViewerClient.create({
    backend: backend === "pdfium" ? { backend: "pdfium", wasm: pdfiumWasm } : { backend: "hayro" },
  });
  const viewer = createViewer({
    client,
    text: { outline: debug },
    // An internal link's address keeps the fragment's `url`, so a new tab opens the same document there.
    links: {
      href: (page) => pageAddress(location, page),
      onExternal,
      words: LINK_WORDS,
      ...(debug ? { onInert: logInert } : {}),
    },
    // Each page is a region named by its label and position; a jump moves focus to its page.
    pages: { name: pageName, roleDescription: message("viewer.page.role"), focusOnJump: true },
  });
  return { backend, client, viewer, strip: new ThumbnailStrip(client, viewer, { name: message("shell.rail.pages") }) };
}

/** Remote files by the PlatformFile that stands for each in the reader. */
const remotes = new WeakMap<PlatformFile, string>();

/** The file name a remote address shows: its last path segment, else its host. */
function remoteName(url: string): string {
  const { pathname, hostname } = new URL(url);
  const last = pathname
    .split("/")
    .filter((part) => part !== "")
    .at(-1);
  if (last === undefined) return hostname;
  try {
    return decodeURIComponent(last);
  } catch {
    return last;
  }
}

/** The PlatformFile that stands for the remote file at `url`. Only the document worker reads it. */
function remoteFile(url: string): PlatformFile {
  const file: PlatformFile = {
    name: remoteName(url),
    size: 0,
    writable: false,
    read: () => Promise.reject(new Error("a remote file is read by the document worker")),
  };
  remotes.set(file, url);
  return file;
}

/** The address `file` stands for, when it is a remote file. */
export function remoteUrlOf(file: PlatformFile | undefined): string | undefined {
  return file === undefined ? undefined : remotes.get(file);
}

/** What the document worker opens for `file`: the File itself, never a copy of its bytes; a remote file's address. */
async function sourceOf(file: PlatformFile): Promise<SourceInit> {
  const url = remotes.get(file);
  if (url !== undefined) return { url };
  const { handle } = file;
  if (handle instanceof Blob) return handle;
  if (handle !== undefined) return handle.getFile();
  return file.read(0, file.size);
}

/** Resolves true on the first animation frame where `done()` holds, or false once `stale()` does. */
function whenPainted(done: () => boolean, stale: () => boolean): Promise<boolean> {
  return new Promise((resolve) => {
    const check = () => {
      if (stale()) resolve(false);
      else if (done()) resolve(true);
      else requestAnimationFrame(check);
    };
    requestAnimationFrame(check);
  });
}

/** What the document worker answers an open with: pages, title, fingerprint. */
type OpenedFile = Awaited<ReturnType<ViewerClient["open"]>>;

/** The reader's engine, as an external store for React (`useStoreState`). */
export class Engine implements ExternalStore<EngineState> {
  /** The reader's lifecycle: the chrome's controls call it, and it calls `open` below. */
  readonly reader: ReaderController;
  readonly #listeners = new Set<() => void>();
  #session: Promise<Session>;
  /** The file open now, for reopening it in another backend. */
  #file: PlatformFile | undefined;
  /** Counts opens and backend switches: an open that isn't the latest shows and closes nothing. */
  #turn = 0;
  /**
   * Warnings received since the last open answered. The worker opens one file at a time and sends
   * a file's warnings before its answer, so each answer takes exactly its own.
   */
  #incoming: DocumentWarning[] = [];
  #state: EngineState;
  /** What is read once per open document (`#turn`): its properties, its reflow. */
  readonly #once = new Map<string, { readonly turn: number; readonly read: Promise<unknown> }>();
  /** Where to go once `file` has opened (from `#page=` or `#nameddest=`): only that open uses it. */
  #place: { readonly file: PlatformFile; readonly place: FragmentPlace } | undefined;

  /** #debug outlines each text span, to check it lies over its glyphs. */
  readonly #outline: boolean;

  /** Where the document worker's requests are counted (the page's are counted by page-feed.ts). */
  readonly #network: NetworkMonitor | undefined;

  /** Where reading positions are kept. */
  readonly #platform: Platform;

  /** Stops recording the open document's position, saving it one last time. */
  #recording: (() => void) | undefined;

  constructor(
    platform: Platform,
    backend: DebugBackend,
    options: { readonly outlineText?: boolean; readonly network?: NetworkMonitor } = {},
  ) {
    this.#outline = options.outlineText === true;
    this.#network = options.network;
    this.reader = new ReaderController(platform, (file, password) => this.#open(file, password));
    this.#state = {
      backend,
      viewer: undefined,
      strip: undefined,
      pages: [],
      outline: undefined,
      title: undefined,
      repairs: [],
      timing: undefined,
      linkProblem: undefined,
      externalLink: undefined,
      fingerprint: undefined,
      resumedAt: undefined,
    };
    this.#platform = platform;
    this.#session = this.#start(backend);
  }

  getState = (): EngineState => this.#state;

  subscribe = (listener: () => void): (() => void) => {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  };

  /** Opens the remote file at `url`, then goes to `place` when given. */
  openRemote(url: string, place?: FragmentPlace): Promise<void> {
    const file = remoteFile(url);
    this.#place = place === undefined ? undefined : { file, place };
    return this.reader.open(file);
  }

  /** Shows why the address's link can't be opened, in place of the home screen. */
  showLinkProblem(problem: Exclude<RemoteFailureReason, "cors">): void {
    this.reader.close();
    this.#set({ linkProblem: problem });
  }

  /** Asks the reader to confirm the web link `link`, as the link layer read it. */
  confirmExternalLink(link: OpenableLink): void {
    this.#set({ externalLink: link });
  }

  /** The external link was opened, copied and closed, or cancelled: nothing waits any more. */
  closeExternalLink(): void {
    this.#set({ externalLink: undefined });
  }

  /** Clears a link problem: the home screen shows again. */
  dismissLinkProblem(): void {
    this.#set({ linkProblem: undefined });
  }

  /** Reads the open document's outline, once: the Outline tab shows it. */
  async loadOutline(): Promise<void> {
    if (this.#state.outline !== undefined || this.reader.getState().phase !== "open") return;
    const turn = this.#turn;
    this.#set({ outline: "loading" });
    const session = await this.#session;
    try {
      const nodes = await session.client.outline();
      // Another document opened meanwhile: its own tab asks again.
      if (turn === this.#turn) this.#set({ outline: outlineModel(nodes, this.#state.pages) });
    } catch (error) {
      // The tab says the outline couldn't be read, rather than that there is none.
      if (turn === this.#turn) this.#set({ outline: "failed" });
      reportError(error);
    }
  }

  /** `read` once per open document under `key`; a failed read isn't kept, so the next caller asks again. */
  #readOnce<T>(key: string, read: (client: ViewerClient) => Promise<T>): Promise<T> {
    const turn = this.#turn;
    const kept = this.#once.get(key);
    if (kept?.turn === turn) return kept.read as Promise<T>;
    const reading = this.#session.then((session) => read(session.client));
    reading.catch(() => {
      if (this.#once.get(key)?.read === reading) this.#once.delete(key);
    });
    this.#once.set(key, { turn, read: reading });
    return reading;
  }

  /**
   * The open document's properties, read once per document: the properties dialog and the print
   * dialog (for the author's permissions) share them.
   */
  properties(): Promise<DocumentProperties> {
    return this.#readOnce("properties", (client) => client.properties());
  }

  /** The open document for reader mode, read once per document. */
  reflow(): Promise<DocumentReflow> {
    return this.#readOnce("reflow", (client) => client.reflow());
  }

  /** Part of a page drawn by the document worker: a reader-mode crop. */
  async renderPart(request: Parameters<ViewerClient["render"]>[0], signal: AbortSignal): Promise<RenderedBitmap> {
    return (await this.#session).client.render(request, { signal, priority: "visible" });
  }

  /**
   * The open file's bytes exactly as opened, for Download a copy: a local file's own bytes, read
   * here; a remote file's from the document worker, which has them cached or fetches them again
   * from its host. An encrypted file stays encrypted.
   */
  async originalBytes(): Promise<{ readonly name: string; readonly bytes: Uint8Array }> {
    const file = this.#file;
    if (file === undefined) throw new Error("no document is open");
    const bytes = remotes.has(file) ? await (await this.#session).client.fileBytes() : await file.read(0, file.size);
    return { name: file.name, bytes };
  }

  /** Page `index` of the open document drawn for printing, by the document worker. */
  async printPage(index: number, options: PrintOptions, signal: AbortSignal): Promise<PrintedPage> {
    return (await this.#session).client.printPage(index, options, { signal });
  }

  /** Goes to outline item `id` of the open document. */
  async goToOutlineItem(id: string): Promise<void> {
    const outline = this.#state.outline;
    const target = typeof outline === "object" ? outline.targets.get(id) : undefined;
    if (target !== undefined) (await this.#session).viewer.jumpToDestination(target);
  }

  /** Goes to `place` in the open document, or in the one opening once it opens. */
  goTo(place: FragmentPlace): void {
    const state = this.reader.getState();
    if (state.phase === "open") void this.#goTo(place);
    else if (state.phase !== "empty" && state.phase !== "error") this.#place = { file: state.file, place };
  }

  /**
   * Goes to `place` in the open document: a page, or a named destination it has. A name the
   * document lacks moves nothing, as in pdf.js; a destination that can't be read is reported.
   */
  async #goTo(place: FragmentPlace): Promise<void> {
    const session = await this.#session;
    if ("page" in place) {
      session.viewer.goToPage(place.page - 1);
      return;
    }
    try {
      const target = await session.client.namedDestination(place.dest);
      if (target !== undefined) session.viewer.goToDestination(target);
    } catch (error) {
      reportError(error);
    }
  }

  /** Moves to `backend`: a new document worker, and the open file reopened in it. */
  async switchBackend(backend: DebugBackend): Promise<void> {
    if (backend === this.#state.backend) return;
    const old = this.#session;
    const file = this.#file;
    this.reader.close();
    this.#turn += 1;
    this.#set({ backend, viewer: undefined, strip: undefined });
    this.#session = this.#start(backend);
    void old.then(({ client, viewer, strip }) => {
      strip.destroy();
      viewer.destroy();
      client.terminate();
    });
    if (file !== undefined) await this.reader.open(file);
  }

  #start(backend: DebugBackend): Promise<Session> {
    const session = startSession(
      backend,
      this.#outline,
      (link) => this.confirmExternalLink(link),
      (index) => spokenPage(this.#state.pages, index),
    );
    session.then(
      (started) => {
        if (this.#session !== session) return;
        started.client.on("warning", (warning) => {
          this.#incoming.push(warning as DocumentWarning);
        });
        // A restarted worker can't reopen an encrypted file: the password lived only in the old
        // one, and this page never kept it. Opening the file again asks for it.
        started.client.on("password-required", () => {
          const file = this.#file;
          if (this.#session === session && file !== undefined) void this.reader.open(file);
        });
        this.#watchWorkerNetwork(started.client);
        this.#set({ viewer: started.viewer, strip: started.strip });
      },
      // A backend that can't load fails the next open, which the reader reports.
      () => undefined,
    );
    return session;
  }

  /**
   * Counts the document worker's requests: its events from now on, and its whole log, which
   * holds what it requested before anyone listened (loading the backend). A restarted worker is a
   * new worker, with a log of its own.
   */
  #watchWorkerNetwork(client: ViewerClient): void {
    const network = this.#network;
    if (network === undefined) return;
    client.on("network", ({ worker, index, record }) => network.workerRequest(worker, index, record));
    client.on("csp-violation", ({ worker, index, violation }) => network.workerViolation(worker, index, violation));
    const readLog = () =>
      void client.networkLog().then(
        (log) => network.workerLog(log),
        () => undefined,
      );
    client.on("restarted", readLog);
    readLog();
  }

  async #open(file: PlatformFile, password: string | undefined): Promise<OpenedDocument> {
    // Opening anything leaves an unusable link's page behind.
    if (this.#state.linkProblem !== undefined) this.#set({ linkProblem: undefined });
    // The network panel counts document traffic from this moment.
    this.#network?.documentOpening(Date.now(), remoteUrlOf(file));
    this.#turn += 1;
    const turn = this.#turn;
    const latest = () => turn === this.#turn;
    const session = await this.#session;
    const started = performance.now();
    let opened: OpenedFile;
    let warnings: DocumentWarning[];
    try {
      // A large file's disk copy outlives the document only while the user lets recent files be kept.
      const keepCopy = readChoices().rememberLocal;
      opened = await session.client.open(
        await sourceOf(file),
        password === undefined ? { keepCopy } : { password, keepCopy },
      );
    } finally {
      warnings = this.#incoming.splice(0);
    }
    // A link that names a page or destination wins over where the reader stopped.
    const place = this.#place?.file === file ? this.#place.place : undefined;
    const resume = place === undefined && latest() ? await this.#kept(opened) : undefined;
    // A later open or a backend switch came first: its own open replaces this one in the worker,
    // so nothing here is shown, and closing it must not close the document the worker holds now.
    if (!latest()) return { pageCount: opened.pageCount };
    const shownFirst = this.#show(session, opened, resume);
    this.#file = file;
    this.#place = undefined;
    this.#set({
      title: opened.title,
      pages: opened.pages,
      outline: undefined,
      repairs: repairsFrom(warnings),
      timing: undefined,
      linkProblem: undefined,
      externalLink: undefined,
      fingerprint: opened.fingerprint,
      resumedAt: resume === undefined ? undefined : shownFirst,
    });
    const firstPage = () => {
      const state = session.viewer.stats().pages[shownFirst];
      return state === "low" || state === "sharp" || state === "error";
    };
    // The view shows once the reader is open, so the page asked for is scrolled to after the first paint.
    void whenPainted(firstPage, () => !latest()).then((painted) => {
      if (!painted) return;
      if (place !== undefined) void this.#goTo(place);
      const backend = `${opened.backend.name} ${opened.backend.version}`;
      this.#set({ timing: { backend, firstPageMs: Math.round(performance.now() - started) } });
    });
    return { pageCount: opened.pageCount, close: () => this.#close(session, opened, latest) };
  }

  /**
   * Shows `opened` in the view and the strip, at `resume` when its reader stopped somewhere, and
   * starts recording its position; returns the page shown at the top (0-based).
   */
  #show(session: Session, opened: OpenedFile, resume: KeptView | undefined): number {
    this.#stopRecording();
    session.viewer.open(opened);
    // In the task that opened it: the first frame paints the kept page, never page 1 first.
    if (resume !== undefined) session.viewer.restore(resume);
    session.strip.open(opened);
    this.#recording = recordPosition(session.viewer, (view) => this.#keep(opened, view));
    // Read from the layout: the view's state only follows at the next frame.
    return resume === undefined ? 0 : (session.viewer.viewPosition()?.point.page ?? 0);
  }

  /** Closes `opened`, when it is still the latest open: its position is saved a last time. */
  #close(session: Session, opened: OpenedFile, latest: () => boolean): void {
    if (!latest()) return;
    this.#stopRecording();
    this.#file = undefined;
    this.#network?.documentClosed();
    this.#set({
      title: undefined,
      pages: [],
      outline: undefined,
      repairs: [],
      timing: undefined,
      fingerprint: undefined,
      resumedAt: undefined,
    });
    if (session.client.document?.id === opened.id) void session.client.close().catch(() => undefined);
  }

  /** The position kept for `opened`, while the reader keeps positions. */
  async #kept(opened: OpenedFile): Promise<KeptView | undefined> {
    if (!readChoices().rememberPositions) return undefined;
    return keptPosition(this.#platform, opened.fingerprint);
  }

  /** Keeps `view` as where the reader stopped in `opened`, while the reader keeps positions. */
  #keep(opened: OpenedFile, view: KeptView): void {
    if (!readChoices().rememberPositions) return;
    const entry = { view, pageCount: opened.pageCount, readAt: Date.now() };
    void this.#platform.positions.set(opened.fingerprint, entry).catch(reportError);
  }

  #stopRecording(): void {
    this.#recording?.();
    this.#recording = undefined;
  }

  #set(next: Partial<EngineState>): void {
    this.#state = { ...this.#state, ...next };
    for (const listener of this.#listeners) listener();
  }
}
