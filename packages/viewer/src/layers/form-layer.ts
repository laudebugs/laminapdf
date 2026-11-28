// Fill & Sign's form layer: a native control over each widget of an interactive form, in
// the page's box, so it follows the page through zoom and rotation as the other layers do. Text
// fields are `<input>` or `<textarea>`, check boxes and radio buttons native inputs, list and
// combo boxes `<select>` (a combo box that takes typing is an `<input>` with a `<datalist>`).
//
// The layer only shows and edits: what a change means is its host's. It hands each field visit's
// value over once (`onCommit`): a text field when it loses focus, a choice the moment it is
// made. Each change is compared with the value the field last had, as read or as handed over, so
// a control kept through a redraw never hands over a stale change. Fields the host guards (a
// signature locks them, or the file is certified) ask the host before their first change
// (`guard`), and a refused change goes back to the value the field had.

import type { FieldValue, FormField, FormWidget } from "@laminapdf/sdk";
import type { Rotation } from "../layout/page-layout.js";
import type { PageLayer } from "../view/viewer.js";
import { placeBox, zoomView } from "./box.js";

/** `/Ff` bits the layer reads (ISO 32000-2 §12.7.5.3, Tables 226, 228 and 230). */
const MULTILINE = 1 << 12;
const PASSWORD = 1 << 13;
const COMB = 1 << 24;
const EDIT = 1 << 18;
const MULTI_SELECT = 1 << 21;
/** Annotation `/F` bits that keep a widget off the page (ISO 32000-2 §12.5.3, Table 167). */
const HIDDEN = 1 << 1;
const NO_VIEW = 1 << 5;

/** The least size of a control's target, in CSS px (WCAG 2.5.8). */
const MIN_TARGET = 24;

/**
 * A control's look in page space, in points: it covers its widget, so the value the page draws
 * doesn't show twice while it is filled, in the page's colours rather than the interface's.
 */
const PAGE_LOOK = {
  fontSize: "10px",
  color: "#000",
  border: "1px solid #8a8174",
  borderRadius: "0",
  padding: "0 2px",
} as const;

/** A control's background: the page's white, or the field tint while fields are highlighted. */
function backgroundOf(highlight: boolean): string {
  return highlight ? "var(--paper-field-tint, #edf2f3)" : "#fff";
}

/** The kinds of field a person fills here: not push buttons (which are pressed) or signatures. */
const FILLABLE: ReadonlySet<FormField["kind"]> = new Set(["text", "checkBox", "radioGroup", "comboBox", "listBox"]);

/** Whether a field of `kind` is one a person fills in Fill & Sign. */
function isFillable(kind: FormField["kind"]): boolean {
  return FILLABLE.has(kind);
}

/** Whether `widget` shows on its page: neither hidden nor kept off the screen by its flags. */
function isShown(widget: FormWidget): boolean {
  return widget.page !== undefined && widget.rect !== undefined && (widget.flags & (HIDDEN | NO_VIEW)) === 0;
}

/** A field's name as a person reads it: its tooltip (`/TU`), else its name. */
function fieldLabel(field: FormField): string {
  return field.tooltip ?? field.name;
}

/** What the form layer needs of the viewer it sits in. */
export interface FormViewer {
  /** Each page's size in points, by page index. */
  pageSizes(): readonly ({ readonly width: number; readonly height: number } | undefined)[];
  /** The view now: the pages' rotation, in degrees. */
  getState(): { readonly rotation: number };
}

/** How a field may be changed, as its host decides. */
export type FieldAccess =
  /** Freely. */
  | { readonly kind: "open" }
  /** Not at all: `/Ff` read-only, or a kind this layer doesn't fill (push buttons, signatures). */
  | { readonly kind: "read-only" }
  /** Only once the host agrees: a signature locks it (`reason` names which). */
  | { readonly kind: "locked"; readonly reason: string }
  /** Freely, but the first change asks the host (a certified file). */
  | { readonly kind: "guarded" };

/** The words the layer's controls carry. */
export interface FormLayerWords {
  /** A list or combo box's entry for no value. */
  readonly noChoice: string;
}

/** `FormLayer`'s options. */
export interface FormLayerOptions {
  /** Field `field` was given `value` in one visit; the host applies it as one step. */
  readonly onCommit: (field: FormField, value: FieldValue) => void;
  /**
   * Whether a guarded or locked field may change: asked once, before its first change. A
   * refusal puts the field back as it was.
   */
  readonly guard?: (field: FormField) => Promise<boolean>;
  /** A field's control took focus. */
  readonly onFocus?: (field: FormField, widget: FormWidget) => void;
  /** Enter in a single-line text field: the host moves on (P01). */
  readonly onEnter?: (field: FormField, widget: FormWidget) => void;
  /** Escape in a field: the host gives the focus back to the page. */
  readonly onEscape?: (field: FormField, widget: FormWidget) => void;
  /** A push button was pressed: the host runs its actions. The page already draws the button. */
  readonly onPress?: (field: FormField, widget: FormWidget) => void;
  readonly words: FormLayerWords;
}

/** One mounted page's controls. */
interface PageForm {
  readonly slot: HTMLElement;
  readonly box: HTMLDivElement;
  size: { readonly width: number; readonly height: number } | undefined;
}

/** A field's text value, or "" when it has none. */
function textOf(value: FieldValue | undefined): string {
  return value?.type === "text" ? value.value : "";
}

/** A button field's on state, or null when it is off. */
function stateOf(value: FieldValue | undefined): string | null {
  return value?.type === "button" ? value.value : null;
}

/** A choice field's chosen exports. */
function choicesOf(value: FieldValue | undefined): readonly string[] {
  if (value?.type === "choice") return value.value;
  if (value?.type === "text" && value.value !== "") return [value.value];
  return [];
}

/** Whether `access` keeps a field from changing now: read-only, or locked until the host agrees. */
function frozen(access: FieldAccess): boolean {
  return access.kind === "read-only" || access.kind === "locked";
}

/** Keys that type into a text field: a locked one asks before any of them. */
function types(event: KeyboardEvent): boolean {
  if (event.ctrlKey || event.metaKey || event.altKey) return false;
  return event.key.length === 1 || event.key === "Backspace" || event.key === "Delete";
}

/** Whether two values are the same, so a visit that changed nothing commits nothing. */
function sameValue(a: FieldValue | undefined, b: FieldValue): boolean {
  if (b.type === "text") return textOf(a) === b.value;
  if (b.type === "button") return stateOf(a) === b.value;
  if (b.type === "choice") {
    const was = choicesOf(a);
    return was.length === b.value.length && was.every((v, i) => v === b.value[i]);
  }
  return a === undefined;
}

/**
 * The widget whose control holds `element`, the focused one: a check box's or radio button's focus
 * is on the input inside its label, which carries the widget's ID.
 */
function focusedWidget(element: Element | null): string | undefined {
  return element?.closest<HTMLElement>("[data-widget-id]")?.dataset.widgetId;
}

/**
 * Calls `end` with what was typed in `el` when a visit to it ends with its text changed: on
 * `change`, and on `blur` too, since Firefox fires no `change` for an input with a list or one
 * whose focus moves away in a key handler. Each text is handed over once; a text `end` refuses
 * (the host said no) can be typed again. A control taken out of the page by a remount hands
 * nothing over: it goes back in, and its visit goes on.
 */
function onVisitEnd(
  el: HTMLInputElement | HTMLTextAreaElement,
  shown: string,
  end: (typed: string) => Promise<boolean>,
): void {
  let handed = shown;
  const done = () => {
    if (!el.isConnected || el.value === handed) return;
    handed = el.value;
    void end(el.value).then((taken) => {
      if (!taken) handed = el.value;
    });
  };
  el.addEventListener("change", done);
  el.addEventListener("blur", done);
}

/** The element of `control` that takes focus: the input inside a label or wrapper. */
function focusOf(control: HTMLElement): HTMLElement {
  return control.querySelector<HTMLElement>("input, textarea, select") ?? control;
}

/** Focuses `control` and scrolls it into view, as little as it takes. */
function reveal(control: HTMLElement): void {
  focusOf(control).focus({ preventScroll: true });
  control.scrollIntoView({ block: "nearest", inline: "nearest" });
}

/** The form layer of one viewer. Add it with `viewer.addLayer(layer)`. */
export class FormLayer implements PageLayer {
  readonly name = "form";
  readonly #viewer: FormViewer;
  readonly #options: FormLayerOptions;
  readonly #pages = new Map<number, PageForm>();
  /** Each page's widgets with their fields. */
  #byPage = new Map<number, { readonly field: FormField; readonly widget: FormWidget }[]>();
  #access = new Map<string, FieldAccess>();
  /** Fields whose first change the host agreed to, this session. */
  readonly #agreed = new Set<string>();
  /** Each field's value as last read or handed over, by field ID: what a change is compared with. */
  readonly #values = new Map<string, FieldValue | undefined>();
  #highlight = true;
  /**
   * The widget whose control takes focus once its page's controls are made (anew), and whether
   * it is brought into view then: a move to it is, a redraw after an edit isn't.
   */
  #refocus: { readonly id: string; readonly reveal: boolean } | undefined;
  /**
   * The focused control itself, kept while its page's controls are made anew (an edit to another
   * field redraws them, and remounts the page): the person may be typing in it, and a control
   * taken out of the page loses what was typed without a `change`. It goes back in its place.
   */
  #kept: HTMLElement | undefined;

  constructor(viewer: FormViewer, options: FormLayerOptions) {
    this.#viewer = viewer;
    this.#options = options;
  }

  /**
   * Shows `fields` (the form as read now), each changeable as `access` says (open when left out).
   * Mounted pages are drawn again; the focused control keeps the focus.
   */
  setForm(fields: readonly FormField[], access: ReadonlyMap<string, FieldAccess> = new Map()): void {
    const byPage = new Map<number, { field: FormField; widget: FormWidget }[]>();
    for (const field of fields) {
      for (const widget of field.widgets) {
        if (!isShown(widget) || widget.page === undefined) continue;
        const list = byPage.get(widget.page) ?? [];
        list.push({ field, widget });
        byPage.set(widget.page, list);
      }
    }
    this.#byPage = byPage;
    this.#access = new Map(access);
    this.#values.clear();
    for (const field of fields) this.#values.set(field.id, field.value);
    const focused = focusedWidget(document.activeElement);
    if (focused !== undefined && this.#refocus === undefined) this.#refocus = { id: focused, reveal: false };
    for (const [page, view] of this.#pages) this.#render(page, view);
  }

  /** Tints every field (P01's Highlight fields, on by default) or none. */
  setHighlight(on: boolean): void {
    this.#highlight = on;
    for (const view of this.#pages.values()) {
      view.box.classList.toggle("lamina-form-highlight", on);
      for (const el of view.box.querySelectorAll<HTMLElement>(".lamina-form-field")) {
        el.style.background = backgroundOf(on);
      }
    }
  }

  /**
   * Focuses widget `id`'s control and brings it into view, if its page is mounted; else it does
   * so once the page is.
   */
  focusWidget(id: string): boolean {
    const control = this.#control(id);
    if (control === undefined) {
      this.#refocus = { id, reveal: true };
      return false;
    }
    reveal(control);
    return true;
  }

  /** Lets a locked field change from now on: the host agreed (Apply anyway). */
  agree(field: string): void {
    this.#agreed.add(field);
    for (const view of this.#pages.values()) {
      for (const control of view.box.querySelectorAll<HTMLElement>(`[data-field-id="${CSS.escape(field)}"]`)) {
        for (const el of [control, ...control.querySelectorAll<HTMLElement>("input, textarea, select")]) {
          if (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement) el.readOnly = false;
          el.removeAttribute("aria-readonly");
        }
      }
    }
  }

  /** Clears every page: the layer is going. */
  destroy(): void {
    for (const [page, view] of [...this.#pages]) this.onUnmount(page, view.slot);
  }

  onMount(page: number, slot: HTMLElement): void {
    const box = document.createElement("div");
    box.className = "lamina-form-layer";
    box.classList.toggle("lamina-form-highlight", this.#highlight);
    Object.assign(box.style, {
      position: "absolute",
      left: "0",
      top: "0",
      transformOrigin: "0 0",
      pointerEvents: "none",
    });
    slot.append(box);
    const view: PageForm = { slot, box, size: undefined };
    this.#pages.set(page, view);
    this.#render(page, view);
  }

  onUnmount(page: number, _slot: HTMLElement): void {
    const view = this.#pages.get(page);
    if (view === undefined) return;
    const focused = view.box.contains(document.activeElement) ? focusedWidget(document.activeElement) : undefined;
    if (focused !== undefined) {
      this.#refocus = { id: focused, reveal: false };
      this.#kept = this.#controlIn(view.box, focused);
    }
    view.box.remove();
    this.#pages.delete(page);
  }

  onZoom(page: number, _slot: HTMLElement, size: { readonly width: number; readonly height: number }): void {
    zoomView(this.#pages, page, size, (p, view) => this.#place(p, view));
  }

  /** The control of widget `widget` on a mounted page, the element that takes focus. */
  #control(widget: string): HTMLElement | undefined {
    for (const view of this.#pages.values()) {
      const el = view.box.querySelector<HTMLElement>(`[data-widget-id="${CSS.escape(widget)}"]`);
      if (el !== null) return el;
    }
    return undefined;
  }

  #place(page: number, view: PageForm): void {
    const points = this.#viewer.pageSizes()[page];
    if (points === undefined || view.size === undefined) return;
    const rotation = this.#viewer.getState().rotation as Rotation;
    placeBox(view.box, points, view.size, rotation);
    const turned = rotation % 180 !== 0;
    const scale = view.size.width / (turned ? points.height : points.width);
    // A small check box or radio button still takes a 24 px target round its middle.
    const least = MIN_TARGET / scale;
    for (const el of view.box.querySelectorAll<HTMLElement>(".lamina-form-toggle, .lamina-form-push")) {
      const [x0 = 0, y0 = 0, x1 = 0, y1 = 0] = (el.dataset.box ?? "").split(" ").map(Number);
      const [w, h] = [Math.max(x1 - x0, least), Math.max(y1 - y0, least)];
      Object.assign(el.style, {
        left: `${(x0 + x1) / 2 - w / 2}px`,
        top: `${(y0 + y1) / 2 - h / 2}px`,
        width: `${w}px`,
        height: `${h}px`,
      });
    }
  }

  /** The control of widget `id` among `box`'s own children. */
  #controlIn(box: HTMLElement, id: string): HTMLElement | undefined {
    return [...box.children].find((c): c is HTMLElement => c instanceof HTMLElement && c.dataset.widgetId === id);
  }

  /** The control `view` keeps as it is made anew: the one focused in it, or one kept from an unmount. */
  #held(view: PageForm): HTMLElement | undefined {
    const focused = view.box.contains(document.activeElement) ? focusedWidget(document.activeElement) : undefined;
    return (focused === undefined ? undefined : this.#controlIn(view.box, focused)) ?? this.#kept;
  }

  #render(page: number, view: PageForm): void {
    const held = this.#held(view);
    const controls: HTMLElement[] = [];
    for (const { field, widget } of this.#byPage.get(page) ?? []) {
      // The control being filled stays as it is, with what was typed, until its visit ends.
      const control = held?.dataset.widgetId === widget.id ? held : this.#make(field, widget);
      if (control !== undefined) controls.push(control);
    }
    if (held !== undefined && controls.includes(held)) this.#kept = undefined;
    this.#replace(view.box, controls, held);
    this.#place(page, view);
    this.#focusAgain(view);
  }

  /**
   * Makes `controls` `box`'s children, in order. A held control already in `box` stays where it
   * is, and the others go round it: taking a focused control out of the page blurs it, which would
   * end its visit half typed.
   */
  #replace(box: HTMLElement, controls: readonly HTMLElement[], held: HTMLElement | undefined): void {
    const at = held === undefined || held.parentElement !== box ? -1 : controls.indexOf(held);
    if (held === undefined || at === -1) {
      box.replaceChildren(...controls);
      return;
    }
    for (const child of [...box.children]) if (child !== held) child.remove();
    held.before(...controls.slice(0, at));
    held.after(...controls.slice(at + 1));
  }

  /** Gives the focus back to the control that had it, or that a move asked for, if it is in `view`. */
  #focusAgain(view: PageForm): void {
    const refocus = this.#refocus;
    if (refocus === undefined) return;
    const el = view.box.querySelector<HTMLElement>(`[data-widget-id="${CSS.escape(refocus.id)}"]`);
    if (el === null) return;
    this.#refocus = undefined;
    if (refocus.reveal) reveal(el);
    else focusOf(el).focus({ preventScroll: true });
  }

  /** The access field `field` has now: agreed locks are open. */
  #accessOf(field: FormField): FieldAccess {
    if (!isFillable(field.kind) || field.readOnly) return { kind: "read-only" };
    const access = this.#access.get(field.id) ?? { kind: "open" };
    return this.#agreed.has(field.id) && access.kind !== "read-only" ? { kind: "open" } : access;
  }

  /** The bare control for a field of this kind, or undefined for a kind that can't be filled. */
  #controlOf(field: FormField, widget: FormWidget, access: FieldAccess): HTMLElement | undefined {
    switch (field.kind) {
      case "text":
        return this.#text(field, access);
      case "checkBox":
      case "radioGroup":
        return this.#toggle(field, widget, access);
      case "comboBox":
      case "listBox":
        return this.#choice(field, access);
      case "pushButton":
        return this.#push(field, widget);
      default:
        return undefined;
    }
  }

  /** The control for one widget, placed over its rectangle; undefined for what can't be filled. */
  #make(field: FormField, widget: FormWidget): HTMLElement | undefined {
    const [x0 = 0, y0 = 0, x1 = 0, y1 = 0] = widget.rect ?? [];
    const access = this.#accessOf(field);
    const control = this.#controlOf(field, widget, access);
    if (control === undefined) return undefined;
    control.dataset.fieldId = field.id;
    control.dataset.widgetId = widget.id;
    control.dataset.box = `${x0} ${y0} ${x1} ${y1}`;
    // A push button is a target over the page's drawing, not a field a flatten removes: it keeps its
    // own class, so the form's field controls are exactly the fields a person fills.
    if (field.kind !== "pushButton") control.classList.add("lamina-form-field");
    if (field.required) control.classList.add("lamina-form-required");
    if (access.kind === "locked") {
      control.classList.add("lamina-form-locked");
      control.title = access.reason;
    }
    Object.assign(control.style, {
      ...PAGE_LOOK,
      background: backgroundOf(this.#highlight),
      position: "absolute",
      boxSizing: "border-box",
      margin: "0",
      left: `${x0}px`,
      top: `${y0}px`,
      width: `${x1 - x0}px`,
      height: `${y1 - y0}px`,
      pointerEvents: "auto",
    });
    // A push button is only a target over the page's own drawing of it.
    if (field.kind === "pushButton") {
      Object.assign(control.style, { background: "transparent", border: "0" });
    }
    // A required field's end edge in the danger colour; a locked field's edge dashed.
    if (field.required) control.style.borderInlineEndColor = "var(--paper-danger, #b3261e)";
    if (access.kind === "locked") control.style.borderStyle = "dashed";
    const name = fieldLabel(field);
    const target = control.querySelector("input, textarea, select") ?? control;
    target.setAttribute("aria-label", name);
    if (field.required) target.setAttribute("aria-required", "true");
    target.addEventListener("focus", () => this.#options.onFocus?.(field, widget));
    target.addEventListener("keydown", (e) => {
      const key = e as KeyboardEvent;
      if (key.key === "Escape") {
        key.preventDefault();
        this.#options.onEscape?.(field, widget);
      } else if (key.key === "Enter" && target instanceof HTMLInputElement && target.type !== "checkbox") {
        key.preventDefault();
        target.blur();
        this.#options.onEnter?.(field, widget);
      }
    });
    if (access.kind === "locked") this.#lock(field, target as HTMLElement);
    return control;
  }

  /** A push button: a native button over the widget, which the host's `onPress` answers. */
  #push(field: FormField, widget: FormWidget): HTMLElement {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "lamina-form-push";
    button.addEventListener("click", () => this.#options.onPress?.(field, widget));
    return button;
  }

  /** A text field: an `<input>`, or a `<textarea>` for a multiline one. Commits on leaving it. */
  #text(field: FormField, access: FieldAccess): HTMLElement {
    const multiline = (field.flags & MULTILINE) !== 0;
    const el = multiline ? document.createElement("textarea") : document.createElement("input");
    if (el instanceof HTMLInputElement) el.type = (field.flags & PASSWORD) !== 0 ? "password" : "text";
    el.value = textOf(this.#valueOf(field));
    if (field.maxLen !== undefined) el.maxLength = field.maxLen;
    if ((field.flags & COMB) !== 0) el.classList.add("lamina-form-comb");
    el.spellcheck = false;
    el.readOnly = frozen(access);
    if (el.readOnly) el.setAttribute("aria-readonly", "true");
    onVisitEnd(el, el.value, (typed) =>
      this.#commit(field, { type: "text", value: typed }, () => {
        el.value = textOf(this.#valueOf(field));
      }),
    );
    return el;
  }

  /** A check box or one radio button: a native input in its label, over the widget. */
  #toggle(field: FormField, widget: FormWidget, access: FieldAccess): HTMLElement {
    const label = document.createElement("label");
    label.className = "lamina-form-toggle";
    const input = document.createElement("input");
    const radio = field.kind === "radioGroup";
    input.type = radio ? "radio" : "checkbox";
    if (radio) input.name = `lamina-${field.id}`;
    const on = widget.onState ?? field.onStates[0] ?? "Yes";
    input.checked = stateOf(this.#valueOf(field)) === on;
    input.dataset.onState = on;
    if (radio) {
      const option = field.options.find((o) => o.export === on)?.display ?? on;
      input.setAttribute("aria-description", option);
    }
    if (frozen(access)) input.setAttribute("aria-readonly", "true");
    input.addEventListener("click", (e) => {
      if (access.kind === "read-only") e.preventDefault();
    });
    input.addEventListener("change", () => {
      const value: FieldValue = { type: "button", value: input.checked ? on : null };
      void this.#commit(field, value, () => {
        input.checked = stateOf(this.#valueOf(field)) === on;
      });
    });
    label.append(input);
    return label;
  }

  /** A combo box that takes typing: an `<input>` with its options in a `<datalist>`. */
  #typedChoice(field: FormField, access: FieldAccess): HTMLElement {
    const chosen = choicesOf(this.#valueOf(field));
    const wrap = document.createElement("span");
    const input = document.createElement("input");
    const list = document.createElement("datalist");
    list.id = `lamina-list-${field.id}`;
    for (const option of field.options) {
      const item = document.createElement("option");
      item.value = option.display;
      list.append(item);
    }
    input.setAttribute("list", list.id);
    input.value =
      chosen[0] === undefined ? "" : (field.options.find((o) => o.export === chosen[0])?.display ?? chosen[0]);
    input.readOnly = frozen(access);
    Object.assign(input.style, { width: "100%", height: "100%", boxSizing: "border-box" });
    onVisitEnd(input, input.value, (typed) => {
      const known = field.options.find((o) => o.display === typed)?.export ?? typed;
      return this.#commit(field, typed === "" ? { type: "clear" } : { type: "choice", value: [known] }, () => {
        const now = choicesOf(this.#valueOf(field))[0];
        input.value = now === undefined ? "" : (field.options.find((o) => o.export === now)?.display ?? now);
      });
    });
    wrap.append(input, list);
    return wrap;
  }

  /** A list or combo box: a `<select>`, or for a combo box that takes typing an `<input>` with a list. */
  #choice(field: FormField, access: FieldAccess): HTMLElement {
    if (field.kind === "comboBox" && (field.flags & EDIT) !== 0) return this.#typedChoice(field, access);
    const chosen = choicesOf(this.#valueOf(field));
    const select = document.createElement("select");
    const multiple = field.kind === "listBox" && (field.flags & MULTI_SELECT) !== 0;
    select.multiple = multiple;
    if (field.kind === "listBox") select.size = Math.max(2, Math.min(field.options.length, 8));
    if (!multiple) {
      const none = document.createElement("option");
      none.value = "";
      none.textContent = this.#options.words.noChoice;
      select.append(none);
    }
    for (const option of field.options) {
      const item = document.createElement("option");
      item.value = option.export;
      item.textContent = option.display;
      item.selected = chosen.includes(option.export);
      select.append(item);
    }
    if (!multiple && chosen.length === 0) select.value = "";
    if (frozen(access)) select.setAttribute("aria-readonly", "true");
    select.addEventListener("change", () => {
      const values = [...select.selectedOptions].map((o) => o.value).filter((v) => v !== "");
      void this.#commit(field, values.length === 0 ? { type: "clear" } : { type: "choice", value: values }, () => {
        const now = choicesOf(this.#valueOf(field));
        for (const option of select.options) option.selected = now.includes(option.value);
        if (!multiple && now.length === 0) select.value = "";
      });
    });
    return select;
  }

  /**
   * A locked field refuses every change until its host agrees: the first attempt asks it. A
   * read-only input fires no `beforeinput`, so typing is caught at `keydown`; clicks at
   * `pointerdown`; a select's choice reaches `change`.
   */
  #lock(field: FormField, target: HTMLElement): void {
    const ask = async (e: Event) => {
      if (this.#agreed.has(field.id)) return;
      e.preventDefault();
      e.stopImmediatePropagation();
      if ((await this.#options.guard?.(field)) === true) {
        this.agree(field.id);
        target.focus({ preventScroll: true });
      }
    };
    target.addEventListener("beforeinput", (e) => void ask(e), { capture: true });
    target.addEventListener("pointerdown", (e) => void ask(e), { capture: true });
    target.addEventListener(
      "keydown",
      (e) => {
        const key = e as KeyboardEvent;
        if (types(key) || key.key === "ArrowDown" || key.key === "ArrowUp") void ask(e);
      },
      { capture: true },
    );
  }

  /** Field `field`'s value as last read or handed over. */
  #valueOf(field: FormField): FieldValue | undefined {
    return this.#values.has(field.id) ? this.#values.get(field.id) : field.value;
  }

  /**
   * Hands `value` to the host as one step, unless it is the value the field has; `undo` puts it
   * back on a refusal. Resolves whether the field took it (an unchanged value counts).
   */
  async #commit(field: FormField, value: FieldValue, undo: () => void): Promise<boolean> {
    if (sameValue(this.#valueOf(field), value)) return true;
    const access = this.#accessOf(field);
    if (access.kind === "read-only") {
      undo();
      return false;
    }
    if (access.kind !== "open" && !this.#agreed.has(field.id)) {
      if ((await this.#options.guard?.(field)) !== true) {
        undo();
        return false;
      }
      this.#agreed.add(field.id);
    }
    this.#values.set(field.id, value.type === "clear" ? undefined : value);
    this.#options.onCommit(field, value);
    return true;
  }
}
