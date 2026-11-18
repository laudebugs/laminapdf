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
import { type CommentColour, laterMessage, type TextKind } from "@laminapdf/ui/editing";
import {
  type AnnotationGesture,
  type AnnotationLayer,
  type PageFrame,
  pageToUser,
  type Viewer,
} from "@laminapdf/viewer";
import type { Engine } from "../session.ts";
import { colourValue, type Point } from "./marks.ts";
import { kindName, updateAnnotOp } from "./ops.ts";
import { needsFallback } from "./rich-text.ts";
import type { EndReason, TextEditor } from "./text-editor.ts";
import {
  editOf,
  finishedEdit,
  placedBox,
  placedCallout,
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

/** A change to a box's look: `fill: undefined` takes the fill away. */
export type LookChange = { readonly [K in keyof TextLook]?: TextLook[K] | undefined };

/** `look` with `change` made. */
function withChange(look: TextLook, change: LookChange): TextLook {
  const next: Record<string, unknown> = { ...look };
  for (const [key, value] of Object.entries(change)) {
    if (value === undefined) delete next[key];
    else next[key] = value;
  }
  return next as unknown as TextLook;
}

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
  /** The fallback font's blob ID in this document, once added. */
  #fallback: Promise<string> | undefined;
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

  /** Whether a box is being typed into. */
  get typing(): boolean {
    return this.#editor.editing !== undefined;
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
    this.#begin(edit, this.#host.layer()?.startGesture(page), false);
  }

  /** Opens the existing text box `annotation` on `page` for typing, hidden in the page while it is typed into. */
  async openExisting(page: number, annotation: PageAnnotation): Promise<void> {
    const { client } = await this.#host.engine.view();
    const box = await client.textBox(annotation.id).catch(() => undefined);
    if (box === undefined) return;
    void this.prepare().catch(() => undefined);
    this.#begin(editOf(annotation.id, box), this.#host.layer()?.startGesture(page, [annotation]), box.simplified);
  }

  #begin(edit: TextEdit, gesture: AnnotationGesture | undefined, simplified: boolean): void {
    this.#editor.end("away");
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
  async ended(edit: TextEdit, now: TextNow, reason: EndReason): Promise<void> {
    const gesture = this.#gesture;
    this.#gesture = undefined;
    this.#set(undefined);
    try {
      const made = await this.#edit(edit, now);
      if (made === undefined || gesture === undefined) {
        await gesture?.cancel();
        if (edit.id !== undefined && reason !== "away") this.#host.select(edit.page, edit.id);
        return;
      }
      const result = await gesture.commit(made.ops, { label: made.label });
      const id = result.createdAnnotations?.[0] ?? edit.id;
      if (id !== undefined && reason !== "away" && made.ops[0]?.op !== "deleteAnnot") this.#host.select(edit.page, id);
    } catch {
      // A failed edit is said by the session's failure toast; the gesture showed the page as it was.
      await gesture?.cancel();
    }
  }

  /** The edit `now` makes of `edit`, in user space through its page's frame, the fallback font named when needed. */
  async #edit(edit: TextEdit, now: TextNow) {
    const frame = await this.#host.frame(edit.page);
    const pageId = this.#host.engine.getState().pages[edit.page]?.id;
    if (frame === undefined || pageId === undefined) return undefined;
    await this.prepare();
    const fallback = needsFallback(now.text) ? await this.#fallbackFont() : undefined;
    return finishedEdit(edit, now, {
      pageId,
      toUser: (point) => pageToUser(frame, point),
      rotation: frame.rotate,
      fallback,
      author: this.#host.author(),
      created: new Date(),
    });
  }

  /** The fallback font's blob ID, added to the document the first time a box needs it. */
  #fallbackFont(): Promise<string> {
    this.#fallback ??= readArabicFont().then((bytes) => this.#host.engine.addBlob(bytes));
    this.#fallback.catch(() => {
      this.#fallback = undefined;
    });
    return this.#fallback;
  }

  /** The inspector's look for the box: the box typed into takes it now, the selected box as an edit. */
  setLook(change: LookChange): void {
    const look = this.#editor.look();
    if (look !== undefined) {
      this.#editor.setLook(withChange(look, change));
      this.#refresh();
      return;
    }
    const selected = this.#selected;
    if (selected === undefined) return;
    const before = editOf(selected.id, selected.box);
    const made = finishedEdit(
      before,
      {
        text: before.text,
        rect: before.rect,
        ...(before.line === undefined ? {} : { line: before.line }),
        look: withChange(before.look, change),
      },
      this.#context(),
    );
    if (made !== undefined) void this.#apply(made.ops, made.label);
  }

  /** Bold or italic: on the selected text while typing, else on every span of the selected box. */
  toggle(style: "bold" | "italic"): void {
    if (this.typing) {
      this.#editor.toggle(style);
      return;
    }
    const selected = this.#selected;
    if (selected === undefined) return;
    const on = !allOf(selected.box.spec.text, style);
    const text = selected.box.spec.text.map((p) => p.map((s) => ({ ...s, [style]: on })));
    const kind = kindName("FreeText").toLocaleLowerCase();
    void this.#apply(
      [updateAnnotOp(selected.id, { spec: { type: "freeText", text } } as Parameters<typeof updateAnnotOp>[1])],
      laterMessage("comment.step.change", { kind }),
    );
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

  /** Bold and italic moved with the caret, or the look changed: the inspector follows. */
  refresh(): void {
    this.#refresh();
  }

  #refresh(): void {
    const editing = this.#editor.editing;
    const look = this.#editor.look();
    if (editing === undefined || look === undefined || this.#state === undefined) return;
    this.#set({ ...this.#state, look, ...this.#editor.styleAtCaret() });
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
    const { client } = await this.#host.engine.view();
    const box = await client.textBox(id).catch(() => undefined);
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

  /** The context of an edit to the selected box (its page's frame isn't needed: nothing moves). */
  #context() {
    return {
      pageId: "",
      toUser: ([x, y]: readonly [number, number]) => ({ x, y }),
      rotation: 0,
      fallback: undefined,
      author: this.#host.author(),
      created: new Date(),
    };
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
