// Fill & Sign (Paper P01): the form layer over the open document, the order Tab visits its fields
// in, and what a signature lets change. The controller reads the form through the document's
// worker, gives the layer its fields with what each may do, applies each field visit as one
// `setFieldValue` step, and keeps the state the toolbar, the Fields tab and the status bar show.
//
// What a signature allows comes from one `wouldViolate` probe over every fillable field, made on
// entering the mode (filling a field changes no signature, so later edits read only the values):
// a certification that permits no change (DocMDP P1) or usage rights that don't cover filling
// guard every field, so the first change asks; a FieldMDP lock or a signature field's `/Lock`
// locks the fields it names, which draw read-only and ask before any change.

import laminaSansUrl from "@laminapdf/fonts/pdf/lamina-sans-regular.ttf?url";
import type { FieldValue, FormField, Op } from "@laminapdf/sdk";
import { type FlatFormTool, laterMessage } from "@laminapdf/ui/editing";
import {
  type FieldAccess,
  FormLayer,
  type PageFrame,
  pageToUser,
  type Viewer,
  type ViewerClient,
} from "@laminapdf/viewer";
import { TextEditor } from "../comment/text-editor.ts";
import type { TextEdit, TextNow } from "../comment/text-session.ts";
import { pointOn } from "../page-points.ts";
import type { Engine } from "../session.ts";
import {
  type FlatPlacementContext,
  flatStampOp,
  flatTextEdit,
  flatTextFinished,
  localeDate,
  needsGlyphFallback,
} from "./flat-tools.ts";
import { accessFrom, type FormBanner, fieldLabel, isFilled, probeOps, type TabStop, tabStops } from "./rules.ts";

/** A change waiting for the person's answer: the conflict dialog. */
interface FormQuestion {
  readonly field: FormField;
  readonly kind: "certified" | "locked";
}

/** What Fill & Sign shows. */
export interface FormState {
  /** The fields as last read; undefined until they are. */
  readonly fields: readonly FormField[] | undefined;
  /** The stops Tab visits, across pages. */
  readonly order: readonly TabStop[];
  /** Each locked field's reason, by field ID. */
  readonly locks: ReadonlyMap<string, string>;
  readonly highlight: boolean;
  /** The active flat-form placement tool, if any. */
  readonly tool: FlatFormTool | undefined;
  /** The field focused now. */
  readonly focused: string | undefined;
  readonly banner: FormBanner | undefined;
  /** A signature allows filling: the status bar says how saving keeps it valid. */
  readonly signed: boolean;
  readonly question: FormQuestion | undefined;
  readonly needAppearances: boolean;
}

/** The first element in `container` that takes the focus, if any. */
function firstFocusable(container: Element | null): HTMLElement | undefined {
  return (
    container?.querySelector<HTMLElement>("button:not([disabled]), [href], input, select, textarea, [tabindex]") ??
    undefined
  );
}

/** The bundled Lamina Sans font's bytes, fetched once (a failed fetch is tried again). */
let laminaSans: Promise<Uint8Array> | undefined;
function readLaminaSans(): Promise<Uint8Array> {
  laminaSans ??= fetch(laminaSansUrl)
    .then((answer) => {
      if (!answer.ok) throw new Error(`the fallback font answered ${answer.status}`);
      return answer.arrayBuffer();
    })
    .then((bytes) => new Uint8Array(bytes));
  laminaSans.catch(() => {
    laminaSans = undefined;
  });
  return laminaSans;
}

/** The bundled Lamina Sans blob ID per viewer/document, reused across mode switches. */
const flatFallbacks = new WeakMap<Viewer, { readonly document: number; readonly blob: Promise<string> }>();

/** Fill & Sign's controller for one viewer. */
export class FormController {
  readonly #engine: Engine;
  readonly #viewer: Viewer;
  #client: ViewerClient | undefined;
  #layer: FormLayer | undefined;
  #editor: TextEditor | undefined;
  #state: FormState = {
    fields: undefined,
    order: [],
    locks: new Map(),
    highlight: true,
    tool: undefined,
    focused: undefined,
    banner: undefined,
    signed: false,
    question: undefined,
    needAppearances: false,
  };
  readonly #listeners = new Set<() => void>();
  #stops: (() => void)[] = [];
  #current = false;
  /** The answer the open question waits for. */
  #answer: ((yes: boolean) => void) | undefined;
  /** The person agreed to change the certified file: no field asks again. */
  #agreedAll = false;
  /** What each field may do, from the probe made on entering the mode. */
  #access = new Map<string, FieldAccess>();
  /** Each page's tab order, read on entering the mode: filling a field doesn't change it. */
  #orders = new Map<number, readonly { readonly widget: string; readonly field: string }[]>();
  /** The newest read, so an older one finishing late is dropped. */
  #reading = 0;
  /** The widget focused now: a field may have several, each its own stop. */
  #focusedWidget: string | undefined;
  /**
   * The document moved into a copy that lays out text: a field's new appearance draws its value
   * in the field's font, which the base module leaves out. Started on entering the mode.
   */
  #textCopy: Promise<void> | undefined;

  constructor(engine: Engine, viewer: Viewer) {
    this.#engine = engine;
    this.#viewer = viewer;
  }

  getState(): FormState {
    return this.#state;
  }

  subscribe(listener: () => void): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  /** Reads the fields without showing them: the status bar's note outside the mode. */
  async peek(): Promise<void> {
    const { client } = await this.#engine.view();
    this.#client = client;
    await this.#read();
  }

  /** Fill & Sign became the mode: the layer, the keys, the form and what the signatures allow. */
  enter(): void {
    if (this.#current) return;
    this.#current = true;
    void this.#engine.view().then(({ client }) => {
      if (!this.#current || this.#layer !== undefined) return;
      this.#client = client;
      this.#attach();
    }, reportError);
  }

  /** Another mode took over: the layer goes, the state stays for the status bar. */
  leave(): void {
    this.#current = false;
    this.#editor?.end("away");
    for (const stop of this.#stops.splice(0)) stop();
    this.#layer = undefined;
    this.#editor = undefined;
    this.#viewer.element.classList.remove("lamina-form-placing");
    this.#answer?.(false);
    this.#set({ tool: undefined, focused: undefined, question: undefined });
  }

  /** The text-laying copy every value is drawn in, started once; a copy that fails is tried again. */
  #prepare(): Promise<void> {
    const client = this.#client;
    if (client === undefined) return Promise.reject(new Error("Fill & Sign has no document"));
    this.#textCopy ??= client.editInTextCopy({ text: true }).catch((e: unknown) => {
      this.#textCopy = undefined;
      throw e;
    });
    return this.#textCopy;
  }

  #attach(): void {
    const layer = new FormLayer(this.#viewer, {
      words: { noChoice: laterMessage("form.noChoice") },
      onCommit: (field, value) => void this.#commit(field, value).catch(reportError),
      guard: (field) => this.#guard(field),
      onFocus: (field, widget) => {
        this.#focusedWidget = widget.id;
        this.#set({ focused: field.id });
      },
      onEnter: (field, widget) => this.#stepFrom({ widget: widget.id, field: field.id }, 1),
      onEscape: () => this.#viewer.element.focus({ preventScroll: true }),
    });
    layer.setHighlight(this.#state.highlight);
    this.#layer = layer;
    const editor = new TextEditor(
      this.#viewer,
      {
        ended: (edit, now) => this.#finishFlatText(edit, now),
        changed: () => undefined,
        inInspector: () => false,
      },
      {
        name: () => laterMessage("form.text.label"),
        placeholder: laterMessage("textbox.placeholder"),
        start: laterMessage("textbox.handle.start"),
        end: laterMessage("textbox.handle.end"),
        knee: laterMessage("textbox.handle.knee"),
        point: laterMessage("textbox.handle.point"),
      },
    );
    this.#editor = editor;
    const element = this.#viewer.element;
    const key = (event: KeyboardEvent) => this.#key(event);
    const press = (event: PointerEvent) => this.#press(event);
    element.addEventListener("keydown", key, { capture: true });
    element.addEventListener("pointerdown", press);
    this.#stops = [
      this.#viewer.addLayer(layer),
      this.#viewer.addLayer(editor),
      () => layer.destroy(),
      () => editor.destroy(),
      () => element.removeEventListener("keydown", key, { capture: true }),
      () => element.removeEventListener("pointerdown", press),
      // An edit redraws the pages it touched with new appearances: read the values again.
      this.#viewer.on("invalidate", () => void this.#read()),
    ];
    void this.#probe();
    // The person reads the form while the copy opens; a copy that fails is tried at the first
    // commit again, whose failure the edit session reports.
    void this.#prepare().catch(reportError);
  }

  /** Reads the fields and each page's tab order, then what the signatures allow, and shows them. */
  async #probe(): Promise<void> {
    const client = this.#client;
    if (client === undefined) return;
    try {
      const { fields } = await client.formFields();
      const pages = [...new Set(fields.flatMap((f) => f.widgets.map((w) => w.page)).filter((p) => p !== undefined))];
      const orders = await Promise.all(
        pages.map(async (page) => [page, (await client.tabOrder(page)).widgets] as const),
      );
      this.#orders = new Map(orders);
      const { access, locks, banner } = accessFrom(fields, await client.wouldViolate(probeOps(fields)));
      this.#access = access;
      this.#set({
        locks,
        banner,
        signed: fields.some((f) => f.kind === "signature" && f.signed) && banner === undefined,
      });
      await this.#read();
    } catch (error) {
      reportError(error);
    }
  }

  /** Reads the fields as they are now and shows them: after an edit, only the values change. */
  async #read(): Promise<void> {
    const client = this.#client;
    if (client === undefined) return;
    const reading = ++this.#reading;
    try {
      const form = await client.formFields();
      if (reading !== this.#reading) return;
      const access = new Map(this.#access);
      if (this.#agreedAll) for (const [id, a] of access) if (a.kind === "guarded") access.set(id, { kind: "open" });
      this.#layer?.setForm(form.fields, access);
      this.#set({
        fields: form.fields,
        order: tabStops(form.fields, this.#orders.size > 0 ? this.#orders : pagesOf(form.fields)),
        needAppearances: form.needAppearances,
      });
    } catch (error) {
      reportError(error);
    }
  }

  /** One field visit's value, as one undo step. */
  async #commit(field: FormField, value: FieldValue): Promise<void> {
    const op = { op: "setFieldValue", args: { field: field.id, value } } as Op;
    // A read already on its way has the value from before this change: it is dropped, and the
    // edit's own redraw reads again.
    this.#reading += 1;
    await this.#prepare();
    await this.#viewer.edits.apply([op], laterMessage("form.step", { field: fieldLabel(field) }));
  }

  /**
   * Asks the person before a guarded or locked field's first change (P01's conflict dialog). Once
   * they agree to change a certified file, no field asks again; the banner stays, naming the
   * certification the changes break.
   */
  #guard(field: FormField): Promise<boolean> {
    if (this.#agreedAll && !this.#state.locks.has(field.id)) return Promise.resolve(true);
    this.#answer?.(false);
    const kind = this.#state.locks.has(field.id) ? "locked" : "certified";
    this.#set({ question: { field, kind } });
    return new Promise((resolve) => {
      this.#answer = (yes) => {
        this.#answer = undefined;
        this.#set({ question: undefined });
        if (yes && kind === "certified") this.#agreedAll = true;
        resolve(yes);
      };
    });
  }

  /** The conflict dialog's answer: Apply anyway (`yes`) or Cancel. */
  answer(yes: boolean): void {
    this.#answer?.(yes);
  }

  /**
   * Tab and Shift Tab inside a field move through the form's order, across pages. Past the last
   * field the focus goes to the status bar, before the first to the Fill & Sign toolbar: never
   * round to the start, so the form is no keyboard trap (WCAG 2.1.2).
   */
  #key(event: KeyboardEvent): void {
    if (event.key === "Escape" && this.#state.tool !== undefined && !this.#editor?.busy) {
      this.#setTool(undefined);
      event.preventDefault();
      return;
    }
    if (event.key !== "Tab" || event.altKey || event.ctrlKey || event.metaKey) return;
    const target = event.target;
    if (!(target instanceof HTMLElement)) return;
    const control = target.closest<HTMLElement>("[data-widget-id]");
    const widget = control?.dataset.widgetId;
    const field = control?.dataset.fieldId;
    if (widget === undefined || field === undefined) return;
    const step = event.shiftKey ? -1 : 1;
    if (this.#stepFrom({ widget, field }, step)) {
      event.preventDefault();
      return;
    }
    const out =
      step === 1
        ? firstFocusable(document.querySelector(".paper-reader-status"))
        : firstFocusable(document.querySelector(".paper-form-toolbar"));
    // Nothing to move to there (a phone hides the status bar): the browser moves the focus on.
    if (out === undefined || !out.checkVisibility()) return;
    event.preventDefault();
    out.focus();
  }

  /**
   * The stop of `at` in the tab order: its widget's, else its field's (a radio group is one
   * stop, whichever of its buttons has the focus).
   */
  #indexOf(at: { readonly widget: string | undefined; readonly field: string }): number {
    const order = this.#state.order;
    const own = order.findIndex((s) => s.widget === at.widget);
    return own === -1 ? order.findIndex((s) => s.field === at.field) : own;
  }

  /** Moves from `at`'s stop by `step` stops; false when there is none that way. */
  #stepFrom(at: { readonly widget: string | undefined; readonly field: string }, step: 1 | -1): boolean {
    const next = this.#state.order[this.#indexOf(at) + step];
    if (next === undefined) return false;
    this.#focus(next);
    return true;
  }

  /** Focuses `stop`, turning to its page first when it isn't shown. */
  #focus(stop: TabStop): void {
    if (this.#layer?.focusWidget(stop.widget) !== true) {
      this.#viewer.goToPage(stop.page, { anchor: "center" });
      this.#layer?.focusWidget(stop.widget);
    }
    this.#focusedWidget = stop.widget;
    this.#set({ focused: stop.field });
  }

  /** Focuses the field of `stop` (a Fields-tab row). */
  focusStop(stop: TabStop): void {
    this.#focus(stop);
  }

  /** Next field or previous field, from the focused one (the first or last when none is). */
  step(step: 1 | -1): void {
    const order = this.#state.order;
    const { focused } = this.#state;
    if (focused !== undefined && this.#stepFrom({ widget: this.#focusedWidget, field: focused }, step)) return;
    const end = order[step === 1 ? 0 : order.length - 1];
    if (focused === undefined && end !== undefined) this.#focus(end);
  }

  /** The next empty required field after the focused one, round to the start. */
  nextRequired(): void {
    const { order, fields, focused } = this.#state;
    if (fields === undefined) return;
    const byId = new Map(fields.map((f) => [f.id, f]));
    const start = focused === undefined ? -1 : this.#indexOf({ widget: this.#focusedWidget, field: focused });
    for (let i = 1; i <= order.length; i++) {
      const stop = order[(start + i + order.length) % order.length];
      const field = stop === undefined ? undefined : byId.get(stop.field);
      if (stop !== undefined && field?.required === true && !isFilled(field)) {
        this.#focus(stop);
        return;
      }
    }
  }

  /** Highlight fields: the tint on or off. */
  toggleHighlight(): void {
    const highlight = !this.#state.highlight;
    this.#layer?.setHighlight(highlight);
    this.#set({ highlight });
  }

  /** Toggles one of the flat-form placement tools (Text, Check, Date). */
  toggleTool(tool: FlatFormTool): void {
    this.#editor?.end("away");
    this.#setTool(this.#state.tool === tool ? undefined : tool);
  }

  #setTool(tool: FlatFormTool | undefined): void {
    this.#viewer.element.classList.toggle("lamina-form-placing", tool !== undefined);
    this.#set({ tool });
  }

  /** Places a flat-form text box, tick or today's date where the page was clicked. */
  #press(event: PointerEvent): void {
    const tool = this.#state.tool;
    if (tool === undefined || event.button !== 0) return;
    if (this.#editor?.busy === true) return;
    const target = event.target instanceof Element ? event.target : null;
    if (target?.closest(".lamina-form-field") !== null) return;
    const at = pointOn(this.#viewer, this.#engine, event);
    if (at === undefined) return;
    event.preventDefault();
    if (tool === "text") {
      this.#editor?.open(flatTextEdit(at.page, [at.x, at.y]));
      return;
    }
    void this.#placeStamp(tool, at.page, [at.x, at.y]).catch(reportError);
  }

  /** Commits a finished flat-form Text box as one undo step. */
  async #finishFlatText(edit: TextEdit, now: TextNow): Promise<void> {
    const context = await this.#placementContext(edit.page, needsGlyphFallback(now.text));
    const made = context === undefined ? undefined : flatTextFinished(now, context);
    if (made === undefined) return;
    await this.#prepare();
    await this.#viewer.edits.apply(made.ops, made.label);
  }

  /** Commits a Check or Date placement immediately as one undo step. */
  async #placeStamp(
    tool: Exclude<FlatFormTool, "text">,
    page: number,
    point: readonly [number, number],
  ): Promise<void> {
    const created = new Date();
    const needFallback = tool === "check" || needsGlyphFallback([[{ text: localeDate(created) }]]);
    const context = await this.#placementContext(page, needFallback, created);
    if (context === undefined) return;
    const made = flatStampOp(tool, point, context);
    await this.#prepare();
    await this.#viewer.edits.apply(made.ops, made.label);
  }

  /** Page frame, page ID and (when needed) bundled fallback font for a flat-form placement. */
  async #placementContext(
    page: number,
    needFallback: boolean,
    created: Date = new Date(),
  ): Promise<FlatPlacementContext | undefined> {
    const info = this.#engine.getState().pages[page];
    if (info === undefined) return undefined;
    const boxes = await this.#engine.pageBoxes(page);
    const frame: PageFrame = { crop: boxes.crop, rotate: info.rotation, unit: boxes.userUnit };
    const fallback = needFallback ? await this.#fallbackFont() : undefined;
    return {
      pageId: info.id,
      toUser: (pt) => pageToUser(frame, pt),
      rotation: frame.rotate,
      fallback,
      created,
    };
  }

  /** Adds Lamina Sans to the open document once for non-WinAnsi flat-form glyphs such as ✓. */
  async #fallbackFont(): Promise<string> {
    const { client } = await this.#engine.view();
    const document = client.document?.id ?? 0;
    const known = flatFallbacks.get(this.#viewer);
    if (known?.document === document) return known.blob;
    const blob = readLaminaSans().then((bytes) => this.#engine.addBlob(bytes));
    flatFallbacks.set(this.#viewer, { document, blob });
    blob.catch(() => {
      if (flatFallbacks.get(this.#viewer)?.blob === blob) flatFallbacks.delete(this.#viewer);
    });
    return blob;
  }

  #set(change: Partial<FormState>): void {
    this.#state = { ...this.#state, ...change };
    for (const listener of [...this.#listeners]) listener();
  }
}

/** Each page's widgets in the order the page lists them: the tab order before it is read. */
function pagesOf(fields: readonly FormField[]): Map<number, { widget: string; field: string }[]> {
  const pages = new Map<number, { widget: string; field: string }[]>();
  for (const field of fields) {
    for (const widget of field.widgets) {
      if (widget.page === undefined) continue;
      const list = pages.get(widget.page) ?? [];
      list.push({ widget: widget.id, field: field.id });
      pages.set(widget.page, list);
    }
  }
  return pages;
}

/** Says a Fill & Sign call failed, in the console: the edit session's own toasts say why an edit did. */
export function reportError(error: unknown): void {
  console.error("Fill & Sign failed", error);
}
