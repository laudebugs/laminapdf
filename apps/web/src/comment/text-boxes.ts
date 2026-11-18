// Comment mode's text boxes (Paper C03): a new text box, callout or typewriter text opened for
// typing where its tool placed it, an existing one opened by a double-click or Enter, and the one
// edit each typing session makes. The box typed into is text-editor.ts's; what the session writes
// is text-session.ts's. Before the first box is made, the document moves into a copy in the text
// module (`editInTextCopy`), so a document opened before that module loaded takes text boxes and
// fallback fonts; the bundled Arabic face is added to the document the first time a box needs it.
// The inspector's text fields change the box being typed into, which its session then writes, or
// the selected box as an edit of its own.

import arabicFontUrl from "@laminapdf/fonts/pdf/noto-sans-arabic-regular.ttf?url";
import type { PageAnnotation, RichText, TextBox } from "@laminapdf/sdk";
import type { CommentColour, TextKind } from "@laminapdf/ui/editing";
import {
  type AnnotationGesture,
  type AnnotationLayer,
  type PageFrame,
  pageToUser,
  type Viewer,
} from "@laminapdf/viewer";
import type { Engine } from "../session.ts";
import { colourValue, type Point } from "./marks.ts";
import { needsFallback } from "./rich-text.ts";
import type { EndReason, TextEditor } from "./text-editor.ts";
import {
  editOf,
  finishedEdit,
  placedBox,
  placedCallout,
  type TextContext,
  type TextEdit,
  type TextLook,
  type TextNow,
} from "./text-session.ts";

/** What the inspector shows of the box being typed into, or of the selected box. */
export interface TextState {
  readonly kind: TextKind;
  readonly look: TextLook;
  /** At the caret while typing; for every span otherwise. */
  readonly bold: boolean;
  readonly italic: boolean;
  /** Some characters need the fallback font. */
  readonly fallback: boolean;
  /** The box held styles outside the subset. */
  readonly simplified: boolean;
  /** Whether the box is being typed into (the inspector shows Done editing). */
  readonly typing: boolean;
  /** The annotation's ID; undefined for a box not yet made. */
  readonly id: string | undefined;
}

/** What text boxes need of Comment mode. */
export interface TextBoxHost {
  readonly engine: Engine;
  readonly viewer: Viewer;
  layer(): AnnotationLayer | undefined;
  frame(page: number): Promise<PageFrame | undefined>;
  /** Whether the person answered the name question, asked before the first comment. */
  nameAsked(): Promise<boolean>;
  author(): string | undefined;
  /** The colour new boxes take, from the text tools' remembered style. */
  colour(): CommentColour;
  /** The text state changed: the inspector shows it. */
  changed(state: TextState | undefined): void;
  /** Selects annotation `id` on `page` once the layer has read it. */
  select(page: number, id: string): void;
  /** Gives the page the focus, where Escape and the arrow keys act on the selection. */
  focusPage(): void;
}

/** The look a new box takes: 12 pt Helvetica, left aligned, in the tool's colour, a 1 pt border but for typewriter text. */
function newLook(kind: TextKind, colour: CommentColour): TextLook {
  return {
    font: "helvetica",
    size: 12,
    align: "left",
    colour: colourValue(colour, "line"),
    border: kind === "typewriter" ? 0 : 1,
    ...(kind === "callout" ? { end: "openArrow" } : {}),
  };
}

/** The bundled Arabic face's bytes, fetched once (a failed fetch is tried again). */
let arabicFont: Promise<Uint8Array> | undefined;
function readArabicFont(): Promise<Uint8Array> {
  arabicFont ??= fetch(arabicFontUrl)
    .then((answer) => {
      if (!answer.ok) throw new Error(`the fallback font answered ${answer.status}`);
      return answer.arrayBuffer();
    })
    .then((bytes) => new Uint8Array(bytes));
  arabicFont.catch(() => {
    arabicFont = undefined;
  });
  return arabicFont;
}

/** A change to a box's look: `fill: null` takes the fill away. */
export type LookChange = Partial<Omit<TextLook, "fill">> & { readonly fill?: TextLook["fill"] | null };

/** `look` with `change` made. */
function withChange(look: TextLook, change: LookChange): TextLook {
  const { fill, ...rest } = change;
  const { fill: before, ...kept } = look;
  const next = fill === null ? undefined : (fill ?? before);
  return { ...kept, ...rest, ...(next === undefined ? {} : { fill: next }) };
}

/** The fallback font's blob ID in each document a viewer shows, once added: Comment mode left and opened again finds it. */
const fallbacks = new WeakMap<Viewer, { readonly document: number; readonly blob: Promise<string> }>();

/** Whether every span of `text` has `style`. */
const allOf = (text: RichText, style: "bold" | "italic") =>
  text.every((p) => p.every((s) => s.text === "" || s[style] === true)) && text.some((p) => p.length > 0);

/** The text boxes of one open document's Comment mode. */
export class TextBoxes {
  readonly #host: TextBoxHost;
  readonly #editor: TextEditor;
  /** The gesture hiding the box being typed into, and drawing nothing else. */
  #gesture: AnnotationGesture | undefined;
  /** The move into a text-capable copy, once started. */
  #textCopy: Promise<void> | undefined;
  /** The edit of the box ended last, until it is drawn: the next box waits for it. */
  #settling: Promise<void> = Promise.resolve();
  /** The selected box, read back, for the inspector. */
  #selected: { readonly id: string; readonly box: TextBox } | undefined;
  #state: TextState | undefined;

  constructor(host: TextBoxHost, editor: TextEditor) {
    this.#host = host;
    this.#editor = editor;
  }

  /** What the inspector shows. */
  get state(): TextState | undefined {
    return this.#state;
  }

  /** Whether a box is being typed into, or a press is ending one. */
  get typing(): boolean {
    return this.#editor.busy;
  }

  /** Starts moving the document into a text-capable copy (Comment mode opened); text boxes wait for it. */
  prepare(): Promise<void> {
    this.#textCopy ??= this.#host.engine
      .view()
      .then(({ client }) => client.editInTextCopy({ text: true }))
      .catch((e: unknown) => {
        // A copy that can't open is tried again by the next box; the edit then says why.
        this.#textCopy = undefined;
        throw e;
      });
    return this.#textCopy;
  }

  /** Opens a new box of `kind` from a press at `from` and a release at `to` on `page`. */
  async openNew(page: number, kind: TextKind, from: Point, to: Point): Promise<void> {
    // The name question takes the focus, so it is asked before the box opens.
    if (!(await this.#host.nameAsked())) return;
    const placed = kind === "callout" ? placedCallout(from, to) : { rect: placedBox(kind, from, to) };
    const edit: TextEdit = {
      page,
      kind,
      id: undefined,
      rect: placed.rect,
      ...("line" in placed ? { line: placed.line } : {}),
      text: [[]],
      look: newLook(kind, this.#host.colour()),
      fallbacks: [],
    };
    void this.prepare().catch(() => undefined);
    await this.#endOpen();
    this.#begin(edit, this.#host.layer()?.startGesture(page), false);
  }

  /**
   * Ends the box open now and waits for its edit to be drawn: a gesture's end shows every
   * annotation of its page again, so the next box's gesture starts only after it.
   */
  async #endOpen(): Promise<void> {
    this.#editor.end("away");
    await this.#settling;
  }

  /** Opens the existing text box `annotation` on `page` for typing, hidden in the page while it is typed into. */
  async openExisting(page: number, annotation: PageAnnotation): Promise<void> {
    const box = await this.#readBox(annotation.id);
    if (box === undefined) return;
    void this.prepare().catch(() => undefined);
    await this.#endOpen();
    this.#begin(editOf(annotation.id, box), this.#host.layer()?.startGesture(page, [annotation]), box.simplified);
  }

  /** Text box `id` read back; a failed read is reported, and nothing opens. */
  async #readBox(id: string): Promise<TextBox | undefined> {
    try {
      const { client } = await this.#host.engine.view();
      return await client.textBox(id);
    } catch (e) {
      reportError(e);
      return undefined;
    }
  }

  #begin(edit: TextEdit, gesture: AnnotationGesture | undefined, simplified: boolean): void {
    this.#gesture = gesture;
    if (!this.#editor.open(edit)) {
      void gesture?.cancel();
      this.#gesture = undefined;
      return;
    }
    this.#set({
      kind: edit.kind,
      look: edit.look,
      ...this.#editor.styleAtCaret(),
      fallback: needsFallback(edit.text),
      simplified,
      typing: true,
      id: edit.id,
    });
  }

  /** The editor ended a box: its edit, then the box shown by the page again (text-editor.ts waits for this). */
  ended(edit: TextEdit, now: TextNow, reason: EndReason): Promise<void> {
    const settled = this.#settle(edit, now, reason);
    this.#settling = settled;
    return settled;
  }

  async #settle(edit: TextEdit, now: TextNow, reason: EndReason): Promise<void> {
    const gesture = this.#gesture;
    this.#gesture = undefined;
    this.#set(undefined);
    // Escape and Done editing leave the box selected and the page focused, so Escape again deselects.
    if (reason !== "away") this.#host.focusPage();
    try {
      const made = await this.#edit(edit, now);
      if (made === undefined) {
        await gesture?.cancel();
        if (edit.id !== undefined && reason !== "away") this.#host.select(edit.page, edit.id);
        return;
      }
      const result =
        gesture === undefined
          ? await this.#host.viewer.edits.apply(made.ops, made.label)
          : await gesture.commit(made.ops, { label: made.label });
      const id = result.createdAnnotations?.[0] ?? edit.id;
      if (id !== undefined && reason !== "away" && made.ops[0]?.op !== "deleteAnnot") this.#host.select(edit.page, id);
    } catch {
      // A failed edit is said by the session's failure toast; the gesture showed the page as it was.
      await gesture?.cancel();
    }
  }

  /** The edit `now` makes of `edit`, in user space through its page's frame, the fallback font named when needed. */
  async #edit(edit: TextEdit, now: TextNow) {
    await this.prepare();
    const fallback = needsFallback(now.text) ? await this.#fallbackFont() : undefined;
    const context = await this.#context(edit.page, fallback);
    return context === undefined ? undefined : finishedEdit(edit, now, context);
  }

  /** What an edit of a box on `page` needs: the page's ID and frame, the fallback font, the author. */
  async #context(page: number, fallback: string | undefined): Promise<TextContext | undefined> {
    const frame = await this.#host.frame(page);
    const pageId = this.#host.engine.getState().pages[page]?.id;
    if (frame === undefined || pageId === undefined) return undefined;
    return {
      pageId,
      toUser: (point) => pageToUser(frame, point),
      rotation: frame.rotate,
      fallback,
      author: this.#host.author(),
      created: new Date(),
    };
  }

  /** The fallback font's blob ID, added to the document the first time a box needs it. */
  async #fallbackFont(): Promise<string> {
    const { client } = await this.#host.engine.view();
    const document = client.document?.id ?? 0;
    const known = fallbacks.get(this.#host.viewer);
    if (known?.document === document) return known.blob;
    const blob = readArabicFont().then((bytes) => this.#host.engine.addBlob(bytes));
    fallbacks.set(this.#host.viewer, { document, blob });
    blob.catch(() => {
      if (fallbacks.get(this.#host.viewer)?.blob === blob) fallbacks.delete(this.#host.viewer);
    });
    return blob;
  }

  /** The inspector's look for the box: the box typed into takes it now, the selected box as an edit. */
  setLook(change: LookChange): void {
    const look = this.#editor.look();
    if (look !== undefined) {
      this.#editor.setLook(withChange(look, change));
      this.refresh();
      return;
    }
    void this.#changeSelected((before) => ({
      text: before.text,
      rect: before.rect,
      ...(before.line === undefined ? {} : { line: before.line }),
      look: withChange(before.look, change),
    }));
  }

  /** Makes the selected box what `now` says it becomes, as one edit through its page's frame. */
  async #changeSelected(now: (before: TextEdit) => TextNow): Promise<void> {
    const selected = this.#selected;
    if (selected === undefined) return;
    const before = editOf(selected.id, selected.box);
    const context = await this.#context(before.page, undefined);
    const made = context === undefined ? undefined : finishedEdit(before, now(before), context);
    if (made !== undefined) await this.#apply(made.ops, made.label);
  }

  /** Bold or italic: on the selected text while typing, else on every span of the selected box. */
  toggle(style: "bold" | "italic"): void {
    if (this.#editor.editing !== undefined) {
      this.#editor.toggle(style);
      return;
    }
    const box = this.#selected?.box;
    if (box === undefined) return;
    const on = !allOf(box.spec.text, style);
    void this.#changeSelected((before) => ({
      text: before.text.map((p) => p.map((s) => ({ ...s, [style]: on }))),
      rect: before.rect,
      ...(before.line === undefined ? {} : { line: before.line }),
    }));
  }

  /** Drops the new box being typed into, with no edit (the inspector's Delete before it is made). */
  cancel(): void {
    this.#editor.cancel();
    void this.#gesture?.cancel();
    this.#gesture = undefined;
    this.#set(undefined);
  }

  /** Ends typing into the box (Done editing): its edit is made and the box stays selected. */
  done(): void {
    this.#editor.end("done");
  }

  /** Bold and italic moved with the caret, the text or the look changed: the inspector follows. */
  refresh(): void {
    const look = this.#editor.look();
    const text = this.#editor.text();
    if (look === undefined || text === undefined || this.#state === undefined) return;
    this.#set({ ...this.#state, look, fallback: needsFallback(text), ...this.#editor.styleAtCaret() });
  }

  /** The selection changed: one text box selected is read back for the inspector. */
  async selected(selection: readonly PageAnnotation[]): Promise<void> {
    if (this.typing) return;
    const [first] = selection;
    if (selection.length !== 1 || first?.subtype !== "FreeText") {
      this.#selected = undefined;
      this.#set(undefined);
      return;
    }
    await this.#read(first.id);
  }

  /** Reads the selected box `id` back, and shows it in the inspector. */
  async #read(id: string): Promise<void> {
    const box = await this.#readBox(id);
    if (box === undefined || this.typing) return;
    this.#selected = { id, box };
    const edit = editOf(id, box);
    this.#set({
      kind: edit.kind,
      look: edit.look,
      bold: allOf(box.spec.text, "bold"),
      italic: allOf(box.spec.text, "italic"),
      fallback: needsFallback(box.spec.text) || (box.spec.fallbacks ?? []).length > 0,
      simplified: box.simplified,
      typing: false,
      id,
    });
  }

  /** Mode left: the box typed into is ended, as a press elsewhere would. */
  leave(): void {
    this.#editor.end("away");
    this.#selected = undefined;
    this.#set(undefined);
  }

  async #apply(ops: Parameters<Viewer["edits"]["apply"]>[0], label: string): Promise<void> {
    try {
      await this.prepare();
      await this.#host.viewer.edits.apply(ops, label);
      const selected = this.#selected;
      if (selected !== undefined) await this.#read(selected.id);
    } catch {
      // A failed edit is said by the session's failure toast.
    }
  }

  #set(state: TextState | undefined): void {
    this.#state = state;
    this.#host.changed(state);
  }
}
