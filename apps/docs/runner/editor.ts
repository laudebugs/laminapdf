// The page side of "Edit and run" (todo B7e): loaded only when a reader presses the button. It puts
// CodeMirror over the example's code in the Paper look (theme.ts), wires up the bar the page renders, sends the code to the runner (worker.ts, one per page, started
// on the first run), and shows each page of the result, or the error with its code and hint.

import { javascript } from "@codemirror/lang-javascript";
import { Compartment } from "@codemirror/state";
import { basicSetup, EditorView } from "codemirror";
import type { RunReply, RunRequest } from "./protocol.js";
import { paperTheme } from "./theme.js";

/** The runner's address: a fixed path, so its own headers (public/_headers' `/run/*`) apply. */
const WORKER = "/run/worker.js";

/**
 * The width a run's pages are drawn at, in pixels: their column's width in device pixels, so a page
 * widened by the divider (todo Q4q) stays sharp, but at least 832 (about three times the default
 * 15rem, for zoom) and at most 2000 (the worker draws up to 20 pages, each held as a bitmap).
 */
function pagePixels(column: HTMLElement): number {
  const wanted = Math.ceil(column.getBoundingClientRect().width * devicePixelRatio);
  return Math.min(2000, Math.max(832, wanted));
}

/** How long a run may take before the runner is stopped: an edit can loop forever. */
const TIME_LIMIT_MS = 30_000;

/**
 * Typing over a selection that spans lines: CodeMirror replaces it itself, as it would typed text,
 * its input handlers first (closeBrackets wraps the selection in a typed bracket). Left to the
 * browser, Chromium merges the lines' elements and writes inline styles to keep their look, which
 * the docs' CSP blocks and reports (`style-src 'self'`). Composition (an IME, a dead key) can't be
 * cancelled, so it still goes to the browser; its blocked styles change nothing a reader sees, since
 * CodeMirror redraws the lines.
 */
const typeOverLines = EditorView.domEventHandlers({
  beforeinput(event, view) {
    const { state } = view;
    const text = event.data;
    const acrossLines = state.selection.ranges.some(
      (range) => state.doc.lineAt(range.from).number !== state.doc.lineAt(range.to).number,
    );
    if (event.inputType !== "insertText" || text === null || !acrossLines) return false;
    event.preventDefault();
    const { from, to } = state.selection.main;
    const insert = () => state.update(state.replaceSelection(text), { scrollIntoView: true, userEvent: "input.type" });
    if (state.facet(EditorView.inputHandler).some((handler) => handler(view, from, to, text, insert))) return true;
    view.dispatch(insert());
    return true;
  },
});

/**
 * The editor of `source`, in a shadow root on `host`, with `wrap` its Wrap lines toggle. There CodeMirror's styles go into adopted
 * style sheets, which the docs' CSP allows; on the page itself they would be a <style> element,
 * which `style-src 'self'` blocks (public/_headers). The --paper-* tokens are custom properties, so
 * they still reach the theme inside it.
 */
function editor(host: HTMLElement, source: string, wrap: HTMLButtonElement): EditorView {
  const root = host.attachShadow({ mode: "open" });
  const wrapping = new Compartment();
  const view = new EditorView({
    doc: source,
    root,
    parent: root,
    // Lines start unwrapped, as in an editor, so their indentation shows the code's shape; the
    // bar's Wrap lines toggle (wrapOnPress) puts EditorView.lineWrapping in this compartment.
    extensions: [basicSetup, javascript({ typescript: true }), wrapping.of([]), paperTheme, typeOverLines],
  });
  wrapOnPress(view, wrapping, wrap);
  return view;
}

/**
 * Wrap lines: each press wraps the editor's long lines or unwraps them, and the button says which
 * (aria-pressed). Reset leaves it as it is.
 */
function wrapOnPress(view: EditorView, wrapping: Compartment, button: HTMLButtonElement): void {
  button.setAttribute("aria-pressed", "false");
  button.addEventListener("click", () => {
    const on = button.getAttribute("aria-pressed") !== "true";
    view.dispatch({ effects: wrapping.reconfigure(on ? EditorView.lineWrapping : []) });
    button.setAttribute("aria-pressed", String(on));
  });
}

let worker: Worker | undefined;
let nextId = 0;
const waiting = new Map<number, (reply: RunReply) => void>();

/** Stops the runner and answers every waiting run with `message`; the next run starts a new one. */
function stop(code: string, message: string): void {
  worker?.terminate();
  worker = undefined;
  for (const [id, done] of waiting) done({ id, ok: false, code, message });
  waiting.clear();
}

/** Runs `request` in the page's runner, starting it the first time. */
function run(request: Omit<RunRequest, "id">): Promise<RunReply> {
  if (worker === undefined) {
    worker = new Worker(WORKER, { type: "module" });
    worker.addEventListener("message", (event: MessageEvent<RunReply>) => {
      waiting.get(event.data.id)?.(event.data);
      waiting.delete(event.data.id);
    });
    worker.addEventListener("error", (event) => {
      stop("error", event.message || "the runner stopped with an error");
    });
  }
  const id = nextId++;
  const started = worker;
  return new Promise((resolve) => {
    const timer = setTimeout(
      () => stop("timeout", `the code ran for ${TIME_LIMIT_MS / 1000} s without finishing, so it was stopped`),
      TIME_LIMIT_MS,
    );
    waiting.set(id, (reply) => {
      clearTimeout(timer);
      resolve(reply);
    });
    started.postMessage({ id, ...request } satisfies RunRequest);
  });
}

/** `bytes` as a `data:application/pdf` URL. */
function dataUrl(bytes: Uint8Array): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result));
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(new Blob([bytes as Uint8Array<ArrayBuffer>], { type: "application/pdf" }));
  });
}

/** An element with a class and, optionally, text. */
function element<K extends keyof HTMLElementTagNameMap>(tag: K, className: string, text?: string) {
  const node = document.createElement(tag);
  node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

/** Page `index` of a run's PDF, drawn from its bitmap. */
function pageCanvas(bitmap: ImageBitmap, index: number): HTMLCanvasElement {
  const canvas = element("canvas", "run-page");
  canvas.width = bitmap.width;
  canvas.height = bitmap.height;
  canvas.setAttribute("role", "img");
  canvas.setAttribute("aria-label", `Page ${index + 1} of the PDF your code made`);
  canvas.getContext("bitmaprenderer")?.transferFromImageBitmap(bitmap);
  return canvas;
}

/** The bar's buttons, which the page renders with their icons (Example.astro), hidden until now. */
export interface RunButtons {
  readonly bar: HTMLElement;
  readonly run: HTMLButtonElement;
  readonly reset: HTMLButtonElement;
  readonly download: HTMLButtonElement;
  /** Wraps the editor's long lines, or unwraps them (wrapOnPress). */
  readonly wrap: HTMLButtonElement;
}

/** The example a runner edits. */
export interface RunExample {
  /** The example's TypeScript file, from the repository root (a key of `RUNNABLE`). */
  readonly file: string;
  /** The example's code as the page shows it. */
  readonly source: string;
  /** The name Download gives the PDF: the example's own (`hello.pdf`). */
  readonly name: string;
}

/** Where the runner goes on the page (Example.astro). */
export interface RunPlaces {
  /** The TypeScript code's place: the editor, the bar and the run's status and errors go at its end. */
  readonly code: HTMLElement;
  /** The first page's picture, hidden once a run has drawn its own pages. */
  readonly picture: HTMLElement;
  /** The picture's column, where a run's pages go. */
  readonly pages: HTMLElement;
}

/**
 * Opens the example's editor in place of its TypeScript code, with the bar under it; a run's pages
 * go where the first page's picture was.
 *
 * @param buttons the bar with Run, Reset and Download
 * @param example the example's file, code and PDF name
 * @param places the code's place, the picture and its column
 */
export function openRunner(buttons: RunButtons, example: RunExample, places: RunPlaces): void {
  const { file, source, name } = example;
  const status = element("p", "run-status");
  status.setAttribute("role", "status");
  const problem = element("div", "run-error");
  problem.setAttribute("role", "alert");
  problem.hidden = true;
  const pages = element("div", "run-pages");
  // Outside Starlight's content spacing, which would push CodeMirror's lines apart.
  const editorHost = element("div", "run-editor not-content");
  places.code.append(editorHost, buttons.bar, status, problem);
  places.pages.append(pages);
  buttons.bar.hidden = false;
  buttons.download.hidden = true;

  const view = editor(editorHost, source, buttons.wrap);
  // The pressed "Edit and run" is gone: focus goes to the editor, where the reader's work starts.
  view.focus();
  /** The last PDF as a data: URL: not a blob: one, since Chromium checks a blob: download against
   * the page's CSP, which lists no blob: (ADR 0011), and reports a violation though the file arrives. */
  let pdf = "";
  const state: Shown = { drawn: undefined, running: false };
  keepSharp(state, file, places.pages, pages);

  // Reset puts back the code and the picture of what it makes.
  buttons.reset.addEventListener("click", () => {
    view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: source } });
    pages.replaceChildren();
    places.picture.hidden = false;
    problem.hidden = true;
    status.textContent = "";
    buttons.download.hidden = true;
    pdf = "";
    state.drawn = undefined;
  });
  buttons.download.addEventListener("click", () => {
    const link = element("a", "");
    link.href = pdf;
    link.download = name;
    link.click();
  });
  buttons.run.addEventListener("click", async () => {
    buttons.run.disabled = true;
    state.running = true;
    status.textContent = worker === undefined ? "Loading Lamina PDF (about 3 MB, once)…" : "Running…";
    const width = pagePixels(places.pages);
    const code = view.state.doc.toString();
    const reply = await run({ file, source: code, width });
    state.running = false;
    buttons.run.disabled = false;
    status.textContent = "";
    if (!reply.ok) {
      showProblem(problem, reply);
      return;
    }
    problem.hidden = true;
    places.picture.hidden = true;
    pages.replaceChildren(...reply.pages.map(pageCanvas));
    state.drawn = { source: code, width };
    pdf = await dataUrl(reply.bytes);
    buttons.download.hidden = false;
    const count = reply.pageCount === 1 ? "1 page" : `${reply.pageCount} pages`;
    const shown = reply.pages.length < reply.pageCount ? `; the first ${reply.pages.length} are shown` : "";
    status.textContent = `Done: ${count}${shown}.`;
  });
}

/** A failed run's code, message and hint, in the bar's alert. */
function showProblem(problem: HTMLElement, reply: Extract<RunReply, { ok: false }>): void {
  problem.replaceChildren(element("strong", "", reply.code), document.createTextNode(`: ${reply.message}`));
  if (reply.hint !== undefined) problem.append(element("p", "run-hint", reply.hint));
  problem.hidden = false;
}

/** A runner's pages: the code that made them and the width they were drawn at, and whether a run is going. */
interface Shown {
  drawn: { readonly source: string; readonly width: number } | undefined;
  running: boolean;
}

/**
 * Draws a run's pages again when the divider (Example.astro, todo Q4q) has widened their column
 * past the width they were drawn at, from the same code, so they stay sharp; the status and the
 * download don't change.
 */
function keepSharp(shown: Shown, file: string, column: HTMLElement, pages: HTMLElement): void {
  addEventListener("lamina-example-page-resized", async () => {
    const width = pagePixels(column);
    const before = shown.drawn;
    if (before === undefined || shown.running || width <= before.width) return;
    shown.running = true;
    const reply = await run({ file, source: before.source, width });
    shown.running = false;
    // A Reset or another run since then has its own pages.
    if (!reply.ok || shown.drawn !== before) return;
    pages.replaceChildren(...reply.pages.map(pageCanvas));
    shown.drawn = { source: before.source, width };
  });
}
