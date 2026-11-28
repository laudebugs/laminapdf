// Edit text as Edit mode's controller uses it (Paper A04): the pointer, the keys, what the layer
// draws and what the Paragraph inspector does. The picks and the typing are `TextEditor`'s; this
// joins them to the viewer, the engine's plan and the fonts the session holds.

import type { AnnotFont, ContentTextRun, Op } from "@laminapdf/sdk";
import { laterMessage } from "@laminapdf/ui/editing";
import type { Viewer } from "@laminapdf/viewer";
import type { DocumentFonts } from "../fonts/document-fonts.ts";
import type { Engine } from "../session.ts";
import type { Box, Point } from "./geometry.ts";
import type { RunEditorView, TextView } from "./layer.ts";
import { TextEditor } from "./text-edit.ts";
import { cssFamilyOf, fontFacts, runBox, type StyleChange } from "./text-runs.ts";

/** What Edit text needs of Edit mode. */
export interface TextToolHost {
  readonly viewer: Viewer;
  readonly engine: Engine;
  readonly fonts: DocumentFonts;
  readonly pageId: (page: number) => string;
  readonly apply: (ops: readonly Op[], label: string, pages: readonly number[]) => Promise<boolean>;
  readonly applyNow: (ops: readonly Op[], label: string, pages: readonly number[]) => Promise<boolean>;
  readonly enqueue: <T>(work: () => Promise<T>) => Promise<T>;
  /** The pick or the typing changed. */
  readonly changed: () => void;
  /** A pick was made: take the selection off the other kinds of object. */
  readonly clearOthers: () => void;
  /** Says something short that the inspector doesn't hold. */
  readonly say: (message: string) => void;
  /** Switches to Select with the block of the page's own content around `box` picked. */
  readonly selectBlock: (page: number, box: Box) => void;
}

/** Edit text over one open document. */
export class TextTool {
  readonly editor: TextEditor;
  readonly #host: TextToolHost;

  constructor(host: TextToolHost) {
    this.#host = host;
    this.editor = new TextEditor({
      textRuns: (page) => host.engine.textRuns(page),
      plan: (page, run, text, fonts) => host.engine.planEditText(page, run, text, fonts as AnnotFont[] | undefined),
      chain: (primary, text) => host.fonts.forText(primary, text),
      pageId: host.pageId,
      apply: host.apply,
      applyNow: host.applyNow,
      enqueue: host.enqueue,
      changed: host.changed,
      clearOthers: host.clearOthers,
      focusPage: () => host.viewer.element.focus({ preventScroll: true }),
      say: host.say,
    });
  }

  /** The pages worth having read: the one in view and its neighbours. */
  #pagesInView(): number[] {
    const current = this.#host.viewer.getState().currentPage;
    return [current - 1, current, current + 1].filter((page) => page >= 0 && this.#host.engine.getState().pages[page]);
  }

  /** Reads the pages in view that aren't read yet. */
  prefetch(): void {
    for (const page of this.#pagesInView()) void this.editor.ensure(page);
  }

  // The pointer.

  /** A press with the Edit text tool. */
  press(page: number, point: Point, shift: boolean): void {
    void this.editor.press(page, point, shift);
  }

  /** The pointer moved with the tool on. */
  hover(page: number, point: Point): void {
    if (!this.editor.has(page)) {
      void this.editor.ensure(page);
      return;
    }
    this.editor.hoverAt(page, point);
  }

  /** A double-click with Select: whether it landed on a line of the page's own text. */
  doubleClick(page: number, point: Point): Promise<boolean> {
    return this.editor.open(page, point);
  }

  /** Edit text in a block of the page's own content: the lines in its box. */
  async editBlock(page: number, box: Box): Promise<boolean> {
    return this.editor.editWithin(page, box);
  }

  // The keys.

  /**
   * A key with the Edit text tool on and nothing typed into: `[` and `]` step through the lines,
   * Enter types into the picked one. Whether the key was this tool's.
   */
  key(event: KeyboardEvent): boolean {
    const page = this.editor.pick?.page ?? this.#host.viewer.getState().currentPage;
    if (event.key === "[" || event.key === "]") {
      const step = event.key === "]" ? 1 : -1;
      void this.editor.ensure(page).then(() => this.editor.cycle(page, step));
      return true;
    }
    if (event.key === "Enter" && this.editor.pick !== undefined) {
      this.editor.editPicked();
      return true;
    }
    return false;
  }

  // The box typed into.

  /** The box's keys: Enter and Tab save and finish or move on, Escape gives up. */
  onKey(key: "enter" | "escape" | "tab" | "shift-tab"): void {
    const page = this.editor.typing?.page;
    if (key === "escape") this.editor.cancel();
    else if (key === "enter") void this.editor.commit("enter");
    else if (page !== undefined) void this.editor.step(page, key === "tab" ? 1 : -1);
  }

  /** The box lost the focus: to the inspector, which keeps it open; anywhere else it is saved. */
  onBlur(next: Element | null): void {
    if (next?.closest(".paper-content-inspector, [role=dialog], [role=menu]") != null) return;
    void this.editor.commit("away");
  }

  // The Paragraph inspector.

  /** Restyles the picked lines. */
  style(change: StyleChange): Promise<boolean> {
    return this.editor.style(change);
  }

  /** Sets the picked lines in registry font `id`. */
  async setFont(id: string): Promise<boolean> {
    return this.editor.style({ font: await this.#host.fonts.name(id) });
  }

  /** Switches to Select with the block around the picked line picked, to move or delete it. */
  selectBlock(): void {
    const pick = this.editor.pick;
    const [first] = pick?.runs ?? [];
    if (pick !== undefined && first !== undefined) this.#host.selectBlock(pick.page, runBox(first));
  }

  // What the layer draws.

  /** Edit text's marks: outlines of the editable lines while the tool is on, the pick and the box typed into. */
  view(toolOn: boolean): TextView {
    const pages = toolOn ? this.#pagesInView().filter((page) => this.editor.has(page)) : [];
    const typing = this.editor.typing;
    const pick = this.editor.pick;
    const hover = this.editor.hover;
    return {
      outlines: new Map(pages.map((page) => [page, this.editor.outlines(page)])),
      picked: (pick?.runs ?? [])
        .filter((run) => run !== typing?.run)
        .map((run) => ({ page: pick?.page ?? 0, box: runBox(run) })),
      ...(toolOn && hover !== undefined && !pick?.runs.includes(hover.run)
        ? { hover: { page: hover.page, box: runBox(hover.run) } }
        : {}),
      ...(typing === undefined ? {} : { editor: editorView(typing.page, typing.run, typing.text, typing.version) }),
    };
  }
}

/** The box typed into for `run`, in the run's look. */
function editorView(page: number, run: ContentTextRun, text: string, version: number): RunEditorView {
  const { bold, italic } = fontFacts(run.fontName);
  return {
    page,
    box: runBox(run),
    text,
    version,
    label: laterMessage("content.text.editor", { page: page + 1, line: run.line + 1 }),
    family: cssFamilyOf(run.fontName),
    size: run.fontSize,
    bold,
    italic,
  };
}
