// Import, export, flatten and the comment summary in the Comments panel (Paper C06, C07): the
// state of the dialogs and the work each does over the document worker. Export and the import
// preview read the document as its edits leave it; import and flatten are ordinary edit steps
// (one undo each); the summary is a new document the worker makes and hands back as saved bytes,
// which opens in a new tab (or window, where the platform has them) or prints.

import type { Op } from "@laminapdf/sdk";
import type { Platform, PlatformFile, ToastQueue } from "@laminapdf/ui";
import {
  type CommentExportChoice,
  type CommentExportFormat,
  type CommentExportPreview,
  exportName,
  exportType,
  type FlattenAnswer,
  type FlattenSignatures,
  type ImportFile,
  type ImportMode,
  laterCountMessage,
  laterMessage,
  type NotDrawn,
  notDrawn,
  type SummaryChoice,
  type SummaryRun,
  startChoice,
} from "@laminapdf/ui/editing";
import type { Viewer, ViewerClient } from "@laminapdf/viewer";
import { downloadName } from "../download-name.ts";
import { HANDOFF_KEY, type Offer, offer } from "../open/handoff.ts";
import { memoryFile } from "../open/memory-file.ts";
import { readSummaryFonts } from "../pdf-font.ts";
import type { SaveFlow } from "../save-flow.ts";
import type { Engine } from "../session.ts";
import {
  type CommentScopes,
  exportOptions,
  flattenCounts,
  flattenOp,
  flattenThreads,
  previewOf,
  scopeCounts,
  staysAmong,
  summaryCounts,
  summaryRequest,
} from "./interchange-data.ts";
import { printSummary } from "./summary-print.ts";
import type { ThreadView } from "./threads.ts";

/** Which dialog is open. */
type InterchangeDialog = "export" | "import" | "flatten" | "summary";

/** What the dialogs show: which is open, with the comments it was opened over. */
export interface InterchangeState {
  readonly dialog: InterchangeDialog | undefined;
  /** The format Export opened on. */
  readonly start: CommentExportFormat;
  /** What the open dialog counts. */
  readonly scopes: CommentScopes;
  /** How the file's signatures meet flattening, read when Flatten opens. */
  readonly signatures: FlattenSignatures;
  /** The file Import was given, once chosen. */
  readonly file: ImportFile | undefined;
  /** Whether the open dialog's work is running. */
  readonly busy: boolean;
  /** Where making the summary is. */
  readonly run: SummaryRun;
}

const NO_SCOPES: CommentScopes = { all: [], shown: [], page: [], selected: [] };
const NO_SIGNATURES: FlattenSignatures = { approval: 0, certified: false };

/** What the controller needs of the Comment mode and the reader. */
export interface InterchangeHost {
  readonly engine: Engine;
  readonly viewer: Viewer;
  readonly platform: Platform;
  readonly toasts: ToastQueue;
  /** Saving as it is now: a copy of a certified file, and a rewrite at the next save. */
  readonly flow: () => SaveFlow;
  /** The open file's name, with `.pdf`. */
  readonly fileName: () => string;
  /** Every thread, as the Comments panel reads them. */
  readonly threads: () => Promise<readonly ThreadView[]>;
  /** The threads the panel shows now; undefined while the panel isn't showing a list. */
  readonly shown: () => readonly string[] | undefined;
  /** The threads ticked in the panel, else the one comment selected on the page. */
  readonly selected: () => readonly string[];
}

/** An error's code, when it is a Lamina PDF error. */
function codeOf(error: unknown): string | undefined {
  return typeof error === "object" && error !== null && "code" in error
    ? String((error as { code: unknown }).code)
    : undefined;
}

/** A file's extension in capitals, without the dot: `XFDF`. */
function formatOf(name: string): string {
  const dot = name.lastIndexOf(".");
  return dot < 0 ? "XFDF" : name.slice(dot + 1).toLocaleUpperCase();
}

/** The dialogs of Export, Import, Flatten and the Comment summary, over one open document. */
export class InterchangeController {
  readonly #host: InterchangeHost;
  #state: InterchangeState = {
    dialog: undefined,
    start: "xfdf",
    scopes: NO_SCOPES,
    signatures: NO_SIGNATURES,
    file: undefined,
    busy: false,
    run: { kind: "idle" },
  };
  readonly #listeners = new Set<() => void>();
  /** The bytes of the file Import is reading. */
  #imported: Uint8Array | undefined;
  /** The summary being made: its abort and the tab waiting for it. */
  #making: { readonly abort: AbortController; readonly offer: Offer | undefined } | undefined;

  constructor(host: InterchangeHost) {
    this.#host = host;
  }

  getState(): InterchangeState {
    return this.#state;
  }

  subscribe(listener: () => void): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  #set(change: Partial<InterchangeState>): void {
    this.#state = { ...this.#state, ...change };
    for (const listener of [...this.#listeners]) listener();
  }

  async #client(): Promise<ViewerClient> {
    return (await this.#host.engine.view()).client;
  }

  /** The comments each dialog counts, read as they are now. */
  async #scopes(): Promise<CommentScopes> {
    const all = await this.#host.threads();
    const shownIds = this.#host.shown();
    const shown = shownIds === undefined ? all : all.filter((t) => shownIds.includes(t.id));
    const page = this.#host.viewer.getState().currentPage;
    const picked = this.#host.selected();
    return { all, shown, page: all.filter((t) => t.page === page), selected: all.filter((t) => picked.includes(t.id)) };
  }

  /** Opens the dialog named, over the comments as they are now. */
  async open(dialog: "export" | "flatten" | "summary", start: CommentExportFormat = "xfdf"): Promise<void> {
    const scopes = await this.#scopes();
    const signatures = dialog === "flatten" ? await this.#signatures(scopes) : NO_SIGNATURES;
    this.#set({ dialog, start, scopes, signatures, busy: false, run: { kind: "idle" } });
  }

  /** Closes the open dialog. */
  close(): void {
    this.#imported = undefined;
    this.#set({ dialog: undefined, file: undefined, busy: false, run: { kind: "idle" } });
  }

  // Export (C06).

  /** What each choice of Export's Comments names. */
  get exportCounts() {
    return scopeCounts(this.#state.scopes);
  }

  /** The Markdown `choice` would write, for the preview. */
  async preview(choice: CommentExportChoice): Promise<CommentExportPreview> {
    const { scopes } = this.#state;
    const client = await this.#client();
    const bytes = await client.exportComments(exportOptions(scopes, choice, this.#host.fileName()));
    return previewOf(new TextDecoder().decode(bytes), scopes, choice);
  }

  /** Writes the export and offers it for saving. */
  async save(choice: CommentExportChoice): Promise<void> {
    this.#set({ busy: true });
    try {
      const client = await this.#client();
      const bytes = await client.exportComments(exportOptions(this.#state.scopes, choice, this.#host.fileName()));
      const name = exportName(this.#host.fileName().replace(/\.pdf$/i, ""), choice.format);
      const blob = new Blob([bytes as BlobPart], { type: exportType(choice.format) });
      const { platform } = this.#host;
      const result = await (platform.saveExport?.(blob, name) ?? platform.saveAs(bytes, name));
      if (result.saved) this.#host.toasts.add({ message: laterMessage("commentsExport.done") });
      this.#set({ busy: false, ...(result.saved ? { dialog: undefined } : {}) });
    } catch (error) {
      this.#failed(error, "commentsExport.failed");
    }
  }

  /** Puts the Markdown on the clipboard. */
  async copy(choice: CommentExportChoice): Promise<void> {
    try {
      const { text } = await this.preview(choice);
      await this.#host.platform.copyText(text);
      this.#host.toasts.add({ message: laterMessage("commentsExport.copied") });
      this.close();
    } catch (error) {
      this.#failed(error, "commentsExport.failed");
    }
  }

  /** Copies the highlights and notes as Markdown without a dialog (`comments.copy-markdown`). */
  async copyHighlights(): Promise<void> {
    this.#set({ scopes: await this.#scopes() });
    await this.copy(startChoice("markdown"));
  }

  // Import (C06).

  /** Asks for an XFDF or FDF file and opens Import on it; nothing opens when the picker is dismissed. */
  async chooseImport(): Promise<void> {
    const [file] = await this.#host.platform.openFiles({ multiple: false, accept: "any" });
    if (file === undefined) return;
    await this.#readImport(file);
  }

  async #readImport(file: PlatformFile): Promise<void> {
    this.#imported = await file.read(0, file.size);
    const shown: ImportFile = {
      key: `${file.name}:${file.size}:${Date.now()}`,
      name: file.name,
      format: formatOf(file.name),
    };
    if (this.#state.dialog !== "import") this.#set({ scopes: await this.#scopes() });
    this.#set({ dialog: "import", file: shown, busy: false });
  }

  /** What importing the chosen file as `mode` would do. Rejects for a file that can't be read. */
  async loadImport(mode: ImportMode) {
    const bytes = this.#imported;
    if (bytes === undefined) throw new Error("no file is chosen");
    const client = await this.#client();
    return client.previewImport(bytes, { mode, file: this.#host.fileName() });
  }

  /** Imports the chosen file as one step. */
  async doImport(mode: ImportMode): Promise<void> {
    const bytes = this.#imported;
    if (bytes === undefined) return;
    this.#set({ busy: true });
    try {
      const { summary } = await this.#host.viewer.edits.importComments(
        bytes,
        { mode, file: this.#host.fileName() },
        laterMessage("import.step"),
      );
      const { added } = summary;
      const done = added === 0 ? "import.done.none" : added === 1 ? "import.done.one" : "import.done";
      this.#host.toasts.add({ message: laterMessage(done, { count: added }) });
      this.close();
    } catch (error) {
      this.#failed(error, "import.failed.toast");
    }
  }

  // Flatten (C07).

  /** How many comments each Flatten scope names, and which of them can't be flattened. */
  get flatten() {
    const { scopes } = this.#state;
    return {
      counts: flattenCounts(scopes),
      stays: (scope: Parameters<typeof flattenThreads>[1]) => staysAmong(flattenThreads(scopes, scope)),
    };
  }

  /** Whether the file is signed, and whether flattening breaks a signature (E5 plan §5). */
  async #signatures(scopes: CommentScopes): Promise<FlattenSignatures> {
    const { engine, viewer } = this.#host;
    try {
      const op = flattenOp(scopes.all.slice(0, 1));
      const [plan, violations] = await Promise.all([
        viewer.edits.plan({}),
        scopes.all.length === 0 ? [] : engine.wouldViolate([op]),
      ]);
      const certified = violations.some((v) => v.restriction === "docMdp" || v.restriction === "usageRights");
      const approval = plan.signatures.filter((s) => s.kind === "approval").length;
      return { approval, certified };
    } catch {
      return NO_SIGNATURES;
    }
  }

  /** Flattens as `action` says: here, here with the file rewritten at the next save, or into a copy. */
  async doFlatten(answer: FlattenAnswer, action: "flatten" | "rewrite" | "copy"): Promise<void> {
    const threads = flattenThreads(this.#state.scopes, answer.scope);
    const ops: Op[] = [flattenOp(threads)];
    const label = laterCountMessage(threads.length, { one: "flatten.step.one", many: "flatten.step" });
    this.#set({ busy: true });
    try {
      if (action === "copy") await this.#flattenInCopy(ops);
      else {
        await this.#host.viewer.edits.apply(ops, label);
        if (action === "rewrite") this.#host.flow().rewriteNextSave(true);
        const done = laterCountMessage(threads.length, { one: "flatten.done.one", many: "flatten.done" });
        this.#host.toasts.add({ message: done });
      }
      this.close();
    } catch (error) {
      this.#failed(error, "flatten.failed");
    }
  }

  /** Writes a copy of the file with the comments flattened, which leaves this one signed and unchanged. */
  async #flattenInCopy(ops: readonly Op[]): Promise<void> {
    const violations = await this.#host.engine.wouldViolate(ops);
    const signatures = violations.map(({ signature }) => ({ kind: signature.kind, field: signature.field }));
    const unique = signatures.filter(
      (s, i) => signatures.findIndex((o) => o.kind === s.kind && o.field === s.field) === i,
    );
    const written = await this.#host.flow().copyWith(
      ops,
      unique.map((s) => (s.field === undefined ? { kind: s.kind } : { kind: s.kind, field: s.field })),
    );
    if (written) this.#host.toasts.add({ message: laterMessage("flatten.copy.done") });
  }

  // The summary (C07).

  /** What each choice of the summary's Comments lists. */
  get summaryCounts() {
    return summaryCounts(this.#state.scopes);
  }

  /**
   * Makes the summary and opens it, or prints it. Called from the press that asked, so a new tab
   * can open before the work starts, as a browser allows only then.
   */
  async makeSummary(choice: SummaryChoice, print: boolean): Promise<void> {
    const { platform } = this.#host;
    const native = platform.capabilities.multiWindow && platform.openWindow !== undefined;
    const tab = print || native ? undefined : this.#openTab();
    const abort = new AbortController();
    this.#making = { abort, offer: tab?.offer };
    this.#set({ run: { kind: "making", print } });
    try {
      const made = await this.#made(choice, abort.signal);
      abort.signal.throwIfAborted();
      const run = await this.#finish(made.bytes, print, native, tab);
      this.#set({ run: { ...run, notDrawn: notDrawn(made.warnings) } });
    } catch (error) {
      tab?.offer.cancel();
      if (abort.signal.aborted) this.#set({ run: { kind: "idle" } });
      else this.#set({ run: { kind: "failed", reason: this.#reason(error) } });
    } finally {
      this.#making = undefined;
    }
  }

  /** Asks the worker for the summary in the fonts and words the app has. */
  async #made(choice: SummaryChoice, signal: AbortSignal) {
    const client = await this.#client();
    const fonts = await readSummaryFonts();
    const request = summaryRequest(this.#state.scopes, choice, fonts, undefined, document.documentElement.lang || "en");
    return client.commentSummary(request, { signal });
  }

  /** Opens, prints or saves the made summary: where it went, for the dialog to say. */
  async #finish(
    bytes: Uint8Array,
    print: boolean,
    native: boolean,
    tab: { readonly offer: Offer; readonly opened: boolean } | undefined,
  ): Promise<Extract<SummaryRun, { kind: "done" }>> {
    const { platform } = this.#host;
    const name = downloadName(laterMessage("summary.name", { name: this.#host.fileName().replace(/\.pdf$/i, "") }));
    const none: readonly NotDrawn[] = [];
    if (print) {
      await printSummary(await this.#client(), platform, bytes);
      return { kind: "done", opened: "printed", notDrawn: none };
    }
    if (native) {
      await platform.openWindow?.(memoryFile(name, bytes));
      return { kind: "done", opened: "window", notDrawn: none };
    }
    if (tab?.opened === true) {
      tab.offer.deliver(name, bytes);
      return { kind: "done", opened: "tab", notDrawn: none };
    }
    // The browser didn't open a tab (a pop-up blocker): the summary is saved as a file instead.
    tab?.offer.cancel();
    await platform.saveAs(bytes, name);
    return { kind: "done", opened: "download", notDrawn: none };
  }

  /** Why a summary failed, in the dialog's three words. */
  #reason(error: unknown): "glyph" | "limit" | "other" {
    reportError(error);
    const code = codeOf(error);
    return code === "missing-glyph" ? "glyph" : code === "limit-exceeded" ? "limit" : "other";
  }

  /** Opens the new tab the summary will arrive in, now, while the press that asked is still a gesture. */
  #openTab(): { readonly offer: Offer; readonly opened: boolean } {
    const handoff = offer();
    const address = new URL(location.href);
    address.hash = `${HANDOFF_KEY}=${handoff.token}`;
    return { offer: handoff, opened: window.open(address.href, "_blank") !== null };
  }

  /** Drops the summary being made: the call stops being waited for, and its bytes are let go. */
  cancelSummary(): void {
    this.#making?.abort.abort();
    this.#making?.offer?.cancel();
  }

  /** Shows a failure as a toast and frees the dialog. */
  #failed(error: unknown, words: "commentsExport.failed" | "import.failed.toast" | "flatten.failed"): void {
    reportError(error);
    this.#host.toasts.add({ message: laterMessage(words) });
    this.#set({ busy: false });
  }
}
