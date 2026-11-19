// Fill & Sign (Paper P01, E8e): the form layer over the open document, the order Tab visits its
// fields in, and what a signature lets change. The controller reads the form through the
// document's worker, gives the layer its fields with what each may do, applies each field visit
// as one `setFieldValue` step, and keeps the state the toolbar, the Fields tab and the status bar
// show.
//
// What a signature allows comes from one `wouldViolate` probe over every fillable field: a
// certification that permits no change (DocMDP P1) or usage rights that don't cover filling
// guard every field, so the first change asks; a FieldMDP lock or a signature field's `/Lock`
// locks the fields it names, which draw read-only and ask before any change.

import type { FieldValue, FormField, FormWidget, Op, Violation } from "@laminapdf/sdk";
import { laterMessage } from "@laminapdf/ui/editing";
import { type FieldAccess, FormLayer, type Viewer, type ViewerClient } from "@laminapdf/viewer";
import type { Engine } from "../session.ts";

/** The kinds a person fills here; push buttons (E8i) and signatures (E10) aren't. */
const FILLABLE = new Set<FormField["kind"]>(["text", "checkBox", "radioGroup", "comboBox", "listBox"]);

/** One stop of the form's tab order: a widget, its field and its page. */
export interface TabStop {
  readonly widget: string;
  readonly field: string;
  readonly page: number;
}

/** Why the form asks before changes: a banner while filling. */
export interface FormBanner {
  readonly kind: "p1" | "usageRights";
  readonly violations: readonly Violation[];
}

/** A change waiting for the person's answer: the conflict dialog. */
export interface FormQuestion {
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
  /** The field focused now. */
  readonly focused: string | undefined;
  readonly banner: FormBanner | undefined;
  /** A signature allows filling: the status bar says how saving keeps it valid. */
  readonly signed: boolean;
  readonly question: FormQuestion | undefined;
  readonly xfa: boolean;
  readonly needAppearances: boolean;
}

/** Whether `field` holds a value. */
export function isFilled(field: FormField): boolean {
  const value = field.value;
  if (value === undefined) return false;
  if (value.type === "text") return value.value !== "";
  if (value.type === "button") return value.value !== null;
  if (value.type === "choice") return value.value.length > 0;
  return false;
}

/** Required fields filled and in all. */
export function requiredCount(fields: readonly FormField[]): { filled: number; total: number } {
  const required = fields.filter((f) => f.required && FILLABLE.has(f.kind));
  return { filled: required.filter(isFilled).length, total: required.length };
}

/**
 * The form's tab order across pages: each page's widgets in the order the page asks for, a radio
 * group once (arrow keys move within it), only the kinds a person fills.
 */
export function tabStops(
  fields: readonly FormField[],
  pages: ReadonlyMap<number, readonly { readonly widget: string; readonly field: string }[]>,
): TabStop[] {
  const byId = new Map(fields.map((f) => [f.id, f]));
  const stops: TabStop[] = [];
  const seen = new Set<string>();
  for (const page of [...pages.keys()].sort((a, b) => a - b)) {
    for (const { widget, field } of pages.get(page) ?? []) {
      const f = byId.get(field);
      if (f === undefined || !FILLABLE.has(f.kind)) continue;
      if (f.kind === "radioGroup" && seen.has(f.id)) continue;
      seen.add(f.id);
      stops.push({ widget, field, page });
    }
  }
  return stops;
}

/** The banner a probe's `violations` call for: a certification that permits no change, or usage rights that don't cover filling. */
function bannerFrom(violations: readonly Violation[]): FormBanner | undefined {
  if (violations.some((v) => v.restriction === "docMdp" && v.permission === 1)) return { kind: "p1", violations };
  if (violations.some((v) => v.restriction === "usageRights")) return { kind: "usageRights", violations };
  return undefined;
}

/** The fields a signature locks, by field ID, each with the words that name the signature. */
function locksFrom(fields: readonly FormField[], violations: readonly Violation[]): Map<string, string> {
  const names = new Map(fields.map((f) => [f.id, f.name]));
  const locks = new Map<string, string>();
  for (const v of violations) {
    const locking = v.restriction === "fieldMdp" || v.restriction === "signedWidget";
    if (!locking || v.field === undefined) continue;
    const signature = v.signature.field === undefined ? undefined : names.get(v.signature.field);
    locks.set(
      v.field,
      signature === undefined ? laterMessage("form.locked.unnamed") : laterMessage("form.locked", { signature }),
    );
  }
  return locks;
}

/** Each field's access from a probe's `violations`, and the banner they call for. */
export function accessFrom(
  fields: readonly FormField[],
  violations: readonly Violation[],
): { access: Map<string, FieldAccess>; locks: Map<string, string>; banner: FormBanner | undefined } {
  const banner = bannerFrom(violations);
  const locks = locksFrom(fields, violations);
  const access = new Map<string, FieldAccess>();
  for (const [field, reason] of locks) access.set(field, { kind: "locked", reason });
  if (banner !== undefined) {
    for (const f of fields) if (!access.has(f.id)) access.set(f.id, { kind: "guarded" });
  }
  return { access, locks, banner };
}

/** The value a field has, as `setFieldValue` takes it. */
function currentValue(field: FormField): FieldValue {
  return field.value ?? { type: "clear" };
}

/**
 * The ops that ask what the signatures allow: each fillable field given the value it has. A file
 * with none still asks, with a metadata change, so a certification that permits no change shows
 * its banner on entering Fill & Sign (P01).
 */
function probeOps(fillable: readonly FormField[]): Op[] {
  if (fillable.length === 0) return [{ op: "setInfo", args: { patch: { title: "" } } } as Op];
  return fillable.map((f) => ({ op: "setFieldValue", args: { field: f.id, value: currentValue(f) } }) as Op);
}

/** Fill & Sign's controller for one viewer. */
export class FormController {
  readonly #engine: Engine;
  readonly #viewer: Viewer;
  #client: ViewerClient | undefined;
  #layer: FormLayer | undefined;
  #state: FormState = {
    fields: undefined,
    order: [],
    locks: new Map(),
    highlight: true,
    focused: undefined,
    banner: undefined,
    signed: false,
    question: undefined,
    xfa: false,
    needAppearances: false,
  };
  readonly #listeners = new Set<() => void>();
  #stops: (() => void)[] = [];
  #current = false;
  /** The answer the open question waits for. */
  #answer: ((yes: boolean) => void) | undefined;
  /** The person agreed to change the certified file: no field asks again. */
  #agreedAll = false;
  /** The newest read, so an older one finishing late is dropped. */
  #reading = 0;
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

  /** Reads the form without showing it: the status bar's note in Read mode. */
  async peek(): Promise<void> {
    const { client } = await this.#engine.view();
    this.#client = client;
    await this.#refresh();
  }

  /** Fill & Sign became the mode: the layer, the keys and the form. */
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
    for (const stop of this.#stops.splice(0)) stop();
    this.#layer = undefined;
    this.#answer?.(false);
    this.#set({ focused: undefined, question: undefined });
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
      words: { required: laterMessage("form.required"), noChoice: laterMessage("form.noChoice") },
      onCommit: (field, value) => void this.#commit(field, value),
      guard: (field) => this.#guard(field),
      onFocus: (field) => this.#set({ focused: field.id }),
      onEnter: (field, widget) => this.#move(widget, 1, field),
      onEscape: () => this.#viewer.element.focus({ preventScroll: true }),
    });
    layer.setHighlight(this.#state.highlight);
    this.#layer = layer;
    const element = this.#viewer.element;
    const key = (event: KeyboardEvent) => this.#key(event);
    element.addEventListener("keydown", key, { capture: true });
    this.#stops = [
      this.#viewer.addLayer(layer),
      () => layer.destroy(),
      () => element.removeEventListener("keydown", key, { capture: true }),
      // An edit redraws the pages it touched with new appearances: read the values again.
      this.#viewer.on("invalidate", () => void this.#refresh()),
    ];
    void this.#refresh();
    // The person reads the form while the copy opens.
    void this.#prepare().catch(() => undefined);
  }

  /** Reads the form, its tab order and what the signatures allow, and shows them. */
  async #refresh(): Promise<void> {
    const client = this.#client;
    if (client === undefined) return;
    const reading = ++this.#reading;
    try {
      const form = await client.formFields();
      const fields = form.fields;
      const pages = [...new Set(fields.flatMap((f) => f.widgets.map((w) => w.page)).filter((p) => p !== undefined))];
      const orders = await Promise.all(
        pages.map(async (page) => [page, (await client.tabOrder(page)).widgets] as const),
      );
      const fillable = fields.filter((f) => FILLABLE.has(f.kind));
      const violations = await client.wouldViolate(probeOps(fillable));
      if (reading !== this.#reading) return;
      const { access, locks, banner } = accessFrom(fields, violations);
      if (this.#agreedAll) for (const [id, a] of access) if (a.kind === "guarded") access.set(id, { kind: "open" });
      const signed = fields.some((f) => f.kind === "signature" && f.signed);
      this.#layer?.setForm(fields, access);
      this.#set({
        fields,
        order: tabStops(fields, new Map(orders)),
        locks,
        banner: this.#agreedAll ? undefined : banner,
        signed: signed && banner === undefined,
        xfa: form.hasXfa,
        needAppearances: form.needAppearances,
      });
    } catch (error) {
      reportError(error);
    }
  }

  /** One field visit's value, as one undo step. */
  async #commit(field: FormField, value: FieldValue): Promise<void> {
    const op = { op: "setFieldValue", args: { field: field.id, value } } as Op;
    await this.#prepare();
    await this.#viewer.edits.apply([op], laterMessage("form.step", { field: field.tooltip ?? field.name }));
  }

  /** Asks the person before a guarded or locked field's first change (P01's conflict dialog). */
  #guard(field: FormField): Promise<boolean> {
    if (this.#agreedAll && !this.#state.locks.has(field.id)) return Promise.resolve(true);
    this.#answer?.(false);
    const kind = this.#state.locks.has(field.id) ? "locked" : "certified";
    this.#set({ question: { field, kind } });
    return new Promise((resolve) => {
      this.#answer = (yes) => {
        this.#answer = undefined;
        this.#set({ question: undefined });
        if (yes && kind === "certified") {
          this.#agreedAll = true;
          this.#set({ banner: undefined });
        }
        resolve(yes);
      };
    });
  }

  /** The conflict dialog's answer: Apply anyway (`yes`) or Cancel. */
  answer(yes: boolean): void {
    this.#answer?.(yes);
  }

  /** Tab and Shift Tab inside a field move through the form's order, across pages. */
  #key(event: KeyboardEvent): void {
    if (event.key !== "Tab" || event.altKey || event.ctrlKey || event.metaKey) return;
    const target = event.target;
    if (!(target instanceof HTMLElement)) return;
    const control = target.closest<HTMLElement>("[data-widget-id]");
    const widget = control?.dataset.widgetId;
    if (widget === undefined) return;
    const step = event.shiftKey ? -1 : 1;
    const index = this.#state.order.findIndex((s) => s.widget === widget || s.field === control?.dataset.fieldId);
    const next = this.#state.order[index + step];
    // Past the last field (or before the first), the focus leaves the page as the browser moves
    // it: never round to the start, so the form is no keyboard trap (WCAG 2.1.2).
    if (index === -1 || next === undefined) return;
    event.preventDefault();
    this.#focus(next);
  }

  /** Moves from `widget` by `step` stops. */
  #move(widget: FormWidget, step: 1 | -1, field?: FormField): void {
    const order = this.#state.order;
    const index = order.findIndex((s) => s.widget === widget.id || s.field === field?.id);
    const next = order[index + step];
    if (next !== undefined) this.#focus(next);
  }

  /** Focuses `stop`, turning to its page first when it isn't shown. */
  #focus(stop: TabStop): void {
    if (this.#layer?.focusWidget(stop.widget) !== true) {
      this.#viewer.goToPage(stop.page, { anchor: "center" });
      this.#layer?.focusWidget(stop.widget);
    }
    this.#set({ focused: stop.field });
  }

  /** Focuses the field of `stop` (a Fields-tab row). */
  focusStop(stop: TabStop): void {
    this.#focus(stop);
  }

  /** Next field or previous field, from the focused one (the first or last when none is). */
  step(step: 1 | -1): void {
    const order = this.#state.order;
    if (order.length === 0) return;
    const index = order.findIndex((s) => s.field === this.#state.focused);
    const next = index === -1 ? order[step === 1 ? 0 : order.length - 1] : order[index + step];
    if (next !== undefined) this.#focus(next);
  }

  /** The next empty required field after the focused one, round to the start. */
  nextRequired(): void {
    const { order, fields, focused } = this.#state;
    if (fields === undefined) return;
    const byId = new Map(fields.map((f) => [f.id, f]));
    const start = order.findIndex((s) => s.field === focused);
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

  #set(change: Partial<FormState>): void {
    this.#state = { ...this.#state, ...change };
    for (const listener of [...this.#listeners]) listener();
  }
}

function reportError(error: unknown): void {
  console.error("Fill & Sign failed", error);
}
