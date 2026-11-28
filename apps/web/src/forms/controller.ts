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

import type { FieldValue, FormField, Op } from "@laminapdf/sdk";
import type { Platform, ToastQueue } from "@laminapdf/ui";
import {
  type ExportChoice,
  type FieldsNote,
  type FlatFormTool,
  laterCountMessage,
  laterMessage,
  type XfaNoticeKind,
} from "@laminapdf/ui/editing";
import {
  type FieldAccess,
  type FormImportPlan,
  FormLayer,
  type PageFrame,
  pageToUser,
  type Viewer,
  type ViewerClient,
} from "@laminapdf/viewer";
import { type EndReason, TextEditor } from "../comment/text-editor.ts";
import type { TextContext, TextEdit, TextNow } from "../comment/text-session.ts";
import { pointOn } from "../page-points.ts";
import { laminaSansBlob } from "../pdf-font.ts";
import type { Engine } from "../session.ts";
import {
  exportCounts,
  type FlattenPlan,
  importMessage,
  importNote,
  importNothing,
  keepFilledJson,
  keepFilledXfdf,
  planFlatten,
  planReset,
  type ResetPlan,
} from "./data-rules.ts";
import { flatStampOp, flatTextEdit, flatTextFinished, localeDate, needsGlyphFallback } from "./flat-tools.ts";
import {
  accessFrom,
  type FormBanner,
  fieldLabel,
  hasSignedSignature,
  isFilled,
  probeOps,
  type TabStop,
  tabStops,
  xfaNotice,
} from "./rules.ts";

/** A change waiting for the person's answer: the conflict dialog. */
interface FormQuestion {
  readonly label: string;
  readonly kind: "certified" | "locked";
}

/** The Form data dialog open now, if any. */
type FormDialog =
  | { readonly kind: "export"; readonly counts: { readonly filled: number; readonly total: number } }
  | { readonly kind: "reset"; readonly plan: ResetPlan }
  | { readonly kind: "flatten"; readonly plan: FlattenPlan; readonly certified: boolean }
  | { readonly kind: "import-error"; readonly file: string; readonly reason: string };

/** What the controller needs of the app around it. */
export interface FormHost {
  readonly platform: Platform;
  readonly toasts: ToastQueue;
  /** The open file's name. */
  readonly fileName: () => string;
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
  /** The Form data dialog open now. */
  readonly dialog: FormDialog | undefined;
  /** What the last import left to say in the Fields tab. */
  readonly note: FieldsNote | undefined;
  /** The notice for a form with an XFA part (E8 plan §2.4); undefined when it has none. */
  readonly xfa: XfaNoticeKind | undefined;
  /** Whether the notice's Undo still takes the removal back: the removal is the newest step. */
  readonly xfaUndoable: boolean;
}

/** The op that takes a form's XFA part out; a first change to a static XFA form carries it. */
const REMOVE_XFA = { op: "removeXfa", args: {} } as Op;

/** The history of the open document's edit session. */
type EditHistory = ReturnType<Viewer["edits"]["getState"]>["history"];

/** The first element in `container` that takes the focus, if any. */
function firstFocusable(container: Element | null): HTMLElement | undefined {
  return (
    container?.querySelector<HTMLElement>("button:not([disabled]), [href], input, select, textarea, [tabindex]") ??
    undefined
  );
}

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
    dialog: undefined,
    note: undefined,
    xfa: undefined,
    xfaUndoable: false,
  };
  /** The history the step that removed the XFA part made, while it is the newest. */
  #xfaMade: EditHistory | undefined;
  /** The reader took the XFA part out once: the notice says so while it stays out. */
  #xfaRemoved = false;
  /** A fill that takes the XFA part out is applying. */
  #xfaApplying = false;
  /** The reader asked to remove the XFA part and was refused: later fills leave it in. */
  #xfaRefused = false;
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

  readonly #host: FormHost;
  /** What the flatten the dialog asks about would break, shown in the banner once it is agreed. */
  #flattenViolations: Awaited<ReturnType<ViewerClient["wouldViolate"]>> = [];

  constructor(engine: Engine, viewer: Viewer, host: FormHost) {
    this.#engine = engine;
    this.#viewer = viewer;
    this.#host = host;
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
    this.#set({ tool: undefined, focused: undefined, question: undefined, dialog: undefined });
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
        ended: (edit, now, reason) => this.#finishFlatText(edit, now, reason),
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
    const escapeKey = (event: KeyboardEvent) => this.#escapeTool(event);
    const press = (event: PointerEvent) => this.#press(event);
    element.addEventListener("keydown", key, { capture: true });
    document.addEventListener("keydown", escapeKey);
    element.addEventListener("pointerdown", press);
    this.#stops = [
      this.#viewer.addLayer(layer),
      this.#viewer.addLayer(editor),
      () => layer.destroy(),
      () => editor.destroy(),
      () => element.removeEventListener("keydown", key, { capture: true }),
      () => document.removeEventListener("keydown", escapeKey),
      () => element.removeEventListener("pointerdown", press),
      // An edit redraws the pages it touched with new appearances: read the values again.
      this.#viewer.on("invalidate", () => void this.#read()),
      // The notice's Undo takes the removal back only while it is the newest step.
      this.#viewer.edits.subscribe(() => this.#followHistory()),
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
      const part = form.hasXfa ? await client.xfa() : undefined;
      // A signature holds the XFA part: a signed field, or the save planner's word that removing
      // it (a document change) would break one, as a certification or usage rights do.
      const held =
        part?.kind === "static" &&
        (hasSignedSignature(form.fields) || (await client.wouldViolate([REMOVE_XFA])).length > 0);
      if (reading !== this.#reading) return;
      this.#layer?.setForm(form.fields, access);
      this.#set({
        fields: form.fields,
        order: tabStops(form.fields, this.#orders.size > 0 ? this.#orders : pagesOf(form.fields)),
        needAppearances: form.needAppearances,
        xfa: xfaNotice(part?.kind, held, this.#xfaRemoved),
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
    await this.#applyFilling([op], laterMessage("form.step", { field: fieldLabel(field) }));
  }

  /**
   * Applies the ops that change answers as one undo step. The first change to a static XFA form
   * carries `removeXfa` in the same step, so Acrobat shows the answers too (E8 plan §2.4); a
   * signed file keeps its XFA part, and the notice says so.
   */
  async #applyFilling(ops: Op[], label: string): Promise<void> {
    const { edits } = this.#viewer;
    // Decided before the first await: a second fill made while this one applies must not carry
    // `removeXfa` too, which would be refused once the part is gone.
    const removing = this.#state.xfa === "static" && !this.#xfaApplying && !this.#xfaRefused;
    if (!removing) {
      await edits.apply(ops, label);
      return;
    }
    this.#xfaApplying = true;
    try {
      try {
        await edits.apply([REMOVE_XFA, ...ops], label);
      } catch {
        // The XFA part couldn't go: treat the form as held from now on, and keep the answer with
        // the part left in. Only this retry's failure, if any, reaches the caller.
        this.#xfaRefused = true;
        await edits.apply(ops, label);
        return;
      }
      this.#xfaRemoved = true;
      this.#xfaMade = edits.getState().history;
      // Read before the flag drops: until the read shows the part gone, a fill made now would
      // carry `removeXfa` against a part that isn't there.
      await this.#read();
      this.#followHistory();
    } finally {
      this.#xfaApplying = false;
    }
  }

  /** Whether the notice's Undo still acts on the step that removed the XFA part. */
  #followHistory(): void {
    const undoable = this.#xfaMade !== undefined && this.#viewer.edits.getState().history === this.#xfaMade;
    if (undoable !== this.#state.xfaUndoable) this.#set({ xfaUndoable: undoable });
  }

  /** The notice's Undo: takes the change that removed the XFA part back, while it is the newest. */
  undoXfaRemoval(): void {
    const { edits } = this.#viewer;
    if (this.#xfaMade === undefined || edits.getState().history !== this.#xfaMade) return;
    // The notice's button goes with the removal: the focus moves to the toolbar, not to the page.
    void edits.undo().then(() => firstFocusable(document.querySelector(".paper-form-toolbar"))?.focus());
  }

  /**
   * Asks the person before a guarded or locked field's first change (P01's conflict dialog). Once
   * they agree to change a certified file, no field asks again; the banner stays, naming the
   * certification the changes break.
   */
  #guard(field: FormField): Promise<boolean> {
    if (this.#agreedAll && !this.#state.locks.has(field.id)) return Promise.resolve(true);
    const kind = this.#state.locks.has(field.id) ? "locked" : "certified";
    return this.#askConflict(fieldLabel(field), kind);
  }

  /** Opens the conflict dialog for `label` and resolves to the person's answer. */
  #askConflict(label: string, kind: "certified" | "locked"): Promise<boolean> {
    this.#answer?.(false);
    this.#set({ question: { label, kind } });
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

  /** Puts down the active flat-form tool on Escape, even when focus is on the toolbar or body. */
  #escapeTool(event: KeyboardEvent): void {
    if (event.key !== "Escape" || this.#state.tool === undefined) return;
    if (this.#editor?.busy === true || this.#state.question !== undefined) return;
    this.#setTool(undefined);
    event.preventDefault();
  }

  /** Commits a finished flat-form Text box as one undo step. */
  async #finishFlatText(edit: TextEdit, now: TextNow, reason: EndReason): Promise<void> {
    if (reason === "escape") this.#viewer.element.focus({ preventScroll: true });
    const context = await this.#placementContext(edit.page, needsGlyphFallback(now.text));
    const made = context === undefined ? undefined : flatTextFinished(edit, now, context);
    if (made === undefined) return;
    await this.#applyFlatOp(made.ops, made.label);
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
    const made = flatStampOp(tool, page, point, context);
    await this.#applyFlatOp(made.ops, made.label);
  }

  /** What a signature image placed on `page` needs: its frame and ID, as every placement does. */
  placementFor(page: number): Promise<TextContext | undefined> {
    return this.#placementContext(page, false);
  }

  /** Whether saving `ops` would break a signature: the probe a placement reads, never a block. */
  async wouldViolate(ops: Op[]): Promise<Awaited<ReturnType<ViewerClient["wouldViolate"]>>> {
    const client = this.#client ?? (await this.#engine.view()).client;
    return client.wouldViolate(ops);
  }

  /** Applies `ops` as one undo step, after asking when saving them would break a signature. */
  placeOps(ops: Op[], label: string): Promise<boolean> {
    return this.#applyFlatOp(ops, label);
  }

  /** Checks signature constraints on a flat-form `addAnnot` op, asks on conflict, then applies it. */
  async #applyFlatOp(ops: Op[], label: string): Promise<boolean> {
    if (!(await this.#confirmOps(ops, label))) return false;
    await this.#prepare();
    await this.#viewer.edits.apply(ops, label);
    return true;
  }

  /**
   * Whether `ops` may go ahead: when saving would break a signature, the banner says so and the
   * conflict dialog asks (once the person agreed to change a certified file, it doesn't ask again).
   */
  async #confirmOps(ops: Op[], label: string): Promise<boolean> {
    const client = this.#client;
    if (client === undefined || this.#agreedAll) return true;
    const violations = await client.wouldViolate(ops);
    if (violations.length === 0) return true;
    const yes = await this.#askConflict(label, "certified");
    if (yes) this.#showBanner(violations);
    return yes;
  }

  /** The banner for what `violations` break, unless one shows already. */
  #showBanner(violations: Awaited<ReturnType<ViewerClient["wouldViolate"]>>): void {
    if (this.#state.banner !== undefined) return;
    const kind = violations.some((v) => v.restriction === "usageRights") ? "usage-rights" : "certified";
    this.#set({ banner: { kind, violations } });
  }

  /** Page frame, page ID and (when needed) bundled fallback font for a flat-form placement. */
  async #placementContext(
    page: number,
    needFallback: boolean,
    created: Date = new Date(),
  ): Promise<TextContext | undefined> {
    const info = this.#engine.getState().pages[page];
    if (info === undefined) return undefined;
    const boxes = await this.#engine.pageBoxes(page);
    const frame: PageFrame = { crop: boxes.crop, rotate: info.rotation, unit: boxes.userUnit };
    const fallback = needFallback ? await laminaSansBlob(this.#viewer, this.#engine) : undefined;
    return {
      pageId: info.id,
      toUser: (pt) => pageToUser(frame, pt),
      rotation: frame.rotate,
      fallback,
      created,
    };
  }

  // Form data: import, export, reset and flatten.

  /** Form data reads the form as it is now: false, with a toast, when the file has none. */
  #form(): readonly FormField[] | undefined {
    const fields = this.#state.fields;
    if (fields === undefined || fields.length === 0) {
      this.#host.toasts.add({ message: laterMessage("form.data.noForm") });
      return undefined;
    }
    return fields;
  }

  /** Form data ▸ Import: picks a file, reads what it would change and applies that as one step. */
  async importData(): Promise<void> {
    const client = this.#client;
    if (client === undefined || this.#form() === undefined) return;
    const [picked] = await this.#host.platform.openFiles({ multiple: false, accept: "any" });
    if (picked === undefined) return;
    // A value being typed commits first, so the plan compares with the form as it is.
    await this.#prepare();
    const file = picked.name;
    let plan: FormImportPlan;
    try {
      const bytes = new Uint8Array(await picked.read(0, picked.size));
      plan = await client.planFormImport(bytes);
    } catch (error) {
      this.#set({
        dialog: { kind: "import-error", file, reason: error instanceof Error ? error.message : String(error) },
      });
      return;
    }
    const { ops } = plan;
    if (ops.length === 0) {
      // Nothing to change: the notice says why, in the Fields tab, beside the rows it would have changed.
      this.#set({ note: importNothing(file, plan) });
      return;
    }
    if (!(await this.#confirmOps(ops, file))) return;
    await this.#prepare();
    await this.#applyFilling(ops, laterMessage("form.import.step"));
    // Undo takes the import back only while it is the newest step, never a later edit.
    const { edits } = this.#viewer;
    const made = edits.getState().history;
    this.#host.toasts.add({
      message: importMessage(plan),
      action: {
        label: laterMessage("form.import.undo"),
        onPress: () => {
          if (edits.getState().history === made) void edits.undo();
        },
      },
    });
    this.#set({ note: importNote(file, plan) });
  }

  /** Form data ▸ Export: opens the export dialog. */
  exportData(): void {
    const fields = this.#form();
    if (fields !== undefined) this.#set({ dialog: { kind: "export", counts: exportCounts(fields) } });
  }

  /** The export dialog's Export…: reads the answers in the format and saves them as a separate file. */
  async runExport(choice: ExportChoice): Promise<void> {
    const client = this.#client;
    this.#set({ dialog: undefined });
    const save = this.#host.platform.saveExport;
    if (client === undefined || save === undefined) return;
    let bytes = await client.exportFormData(choice.format, this.#host.fileName());
    if (choice.filledOnly) bytes = this.#onlyFilled(bytes, choice.format);
    const saved = await save.call(
      this.#host.platform,
      new Blob([bytes as BlobPart], { type: mimeOf(choice.format) }),
      choice.name,
    );
    if (saved.saved) this.#host.toasts.add({ message: laterMessage("form.export.done", { name: saved.name }) });
  }

  /** `bytes` with the fields that have no answer left out; FDF, which can't be cut here, keeps them all. */
  #onlyFilled(bytes: Uint8Array, format: ExportChoice["format"]): Uint8Array {
    if (format === "json") return keepFilledJson(bytes);
    if (format === "xfdf") {
      return keepFilledXfdf(bytes, (xml) => new DOMParser().parseFromString(xml, "application/xml"));
    }
    return bytes;
  }

  /** Form data ▸ Reset form: asks, then gives every unlocked field its default as one step. */
  async resetData(): Promise<void> {
    const fields = this.#form();
    if (fields === undefined) return;
    const plan = planReset(fields, this.#state.locks);
    if (plan.ops.length === 0) {
      this.#host.toasts.add({ message: laterMessage("form.reset.done") });
      return;
    }
    // A certified file asks first, before the reset dialog, as a first change to a field does.
    if (!(await this.#confirmOps(plan.ops, laterMessage("form.reset.step")))) return;
    this.#set({ dialog: { kind: "reset", plan } });
  }

  /** Form data ▸ Flatten form: the dialog, which on a certified file is also the conflict question. */
  async flattenData(): Promise<void> {
    const fields = this.#form();
    const client = this.#client;
    if (fields === undefined || client === undefined) return;
    const plan = planFlatten(fields, this.#state.locks);
    if (plan.op === undefined) {
      this.#host.toasts.add({ message: laterMessage("form.flatten.allLocked") });
      return;
    }
    const violations = this.#agreedAll ? [] : await client.wouldViolate([plan.op]);
    this.#set({ dialog: { kind: "flatten", plan, certified: violations.length > 0 } });
    this.#flattenViolations = violations;
  }

  /** A Form data dialog was answered: `yes` does what it asked, anything else closes it. */
  async answerDialog(yes: boolean): Promise<void> {
    const dialog = this.#state.dialog;
    this.#set({ dialog: undefined });
    if (!yes || dialog === undefined) return;
    try {
      await this.#prepare();
      if (dialog.kind === "reset") {
        await this.#applyFilling(dialog.plan.ops, laterMessage("form.reset.step"));
        this.#host.toasts.add({ message: laterMessage("form.reset.done") });
      } else if (dialog.kind === "flatten" && dialog.plan.op !== undefined) {
        // The person agreed: the banner names what saving now breaks.
        if (dialog.certified) this.#showBanner(this.#flattenViolations);
        await this.#viewer.edits.apply([dialog.plan.op], laterMessage("form.flatten.step"));
        this.#host.toasts.add({
          message: laterCountMessage(dialog.plan.ids.length, {
            one: "form.flatten.done.one",
            many: "form.flatten.done.many",
          }),
        });
      }
    } catch (error) {
      reportError(error);
    }
  }

  /** Closes the Form data dialog without doing anything. */
  closeDialog(): void {
    this.#set({ dialog: undefined });
  }

  /** The Fields tab's import notice was dismissed. */
  dismissNote(): void {
    this.#set({ note: undefined });
  }

  #set(change: Partial<FormState>): void {
    this.#state = { ...this.#state, ...change };
    for (const listener of [...this.#listeners]) listener();
  }
}

/** The media type of a form data file in `format`. */
function mimeOf(format: ExportChoice["format"]): string {
  if (format === "json") return "application/json";
  return format === "xfdf" ? "application/vnd.adobe.xfdf" : "application/vnd.fdf";
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
