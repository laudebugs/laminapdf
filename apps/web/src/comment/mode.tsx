// The Comment mode in the reader (Paper C01, C02): registered while a document is open, with its
// commands; while it is the mode, the annotation layer over the pages, the tools, the toolbar
// under the top bar with the banner on a file whose certification forbids comments and the
// covered-text notice, the inspector for the selection, and the question for the name comments
// carry. Loaded with later.ts once a document is open, so none of it is in the startup script.

import type { PageAnnotation } from "@laminapdf/sdk";
import {
  Button,
  type CommandRegistry,
  Dialog,
  IconButton,
  Inspector,
  type ModeRegistry,
  Notice,
  type ShellLayout,
  TextInput,
  useStoreState,
} from "@laminapdf/ui";
import {
  AnnotationInspector,
  type AnnotationInspectorProps,
  type CommentColour,
  type CommentTool,
  CommentToolbar,
  FILL_COLOURS,
  LINE_COLOURS,
  laterMessage,
  registerCommentCommands,
  restrictionsOf,
  SHAPE_KINDS,
  type ShapeKind,
  SignatureDetails,
  type TextAlignment,
  type TextKind,
} from "@laminapdf/ui/editing";
import type { Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useMemo, useRef, useState } from "react";
import { toolsOf } from "../edit-tools.tsx";
import type { SaveFlow } from "../save-flow.ts";
import type { Engine } from "../session.ts";
import { CommentController } from "./controller.ts";
import { colourName, colourValue } from "./marks.ts";
import { kindName, kindOf } from "./ops.ts";
import { familyOf, type StyleKey, styleKey, takesWidth } from "./styles.ts";
import type { TextState } from "./text-boxes.ts";

const controllers = new WeakMap<Viewer, CommentController>();

/** The Comment controller of the document `viewer` shows, once made. */
function controllerOf(viewer: Viewer): CommentController | undefined {
  return controllers.get(viewer);
}

/** The swatches of a colour family. */
function swatches(family: "fill" | "line") {
  return family === "fill"
    ? FILL_COLOURS.map((id) => ({ id, label: laterMessage(`colour.${id}`), color: `annotation-${id}` as const }))
    : LINE_COLOURS.map((id) => ({ id, label: laterMessage(`colour.${id}`), color: `content-line-${id}` as const }));
}

/**
 * The handlers of Comment's commands for the document `viewer` shows. A tool command switches to
 * Comment first, so the palette's Highlight text works in any mode. Each finds the mode's
 * controller when it runs: the controller is made once the document's worker is ready.
 */
export function commentActions(viewer: Viewer, modes: ModeRegistry) {
  const on = (run: (controller: CommentController) => void) => () => {
    const controller = controllerOf(viewer);
    if (controller !== undefined) run(controller);
  };
  const tool = (name: CommentTool) => (args?: unknown) => {
    if (modes.getState().current !== "comment") modes.switchTo("comment");
    const shape = name === "shape" && SHAPE_KINDS.includes(args as ShapeKind) ? (args as ShapeKind) : undefined;
    on((controller) => controller.use(name, shape))();
  };
  const text = (kind: TextKind) => () => {
    if (modes.getState().current !== "comment") modes.switchTo("comment");
    on((controller) => controller.use("text", undefined, kind))();
  };
  const texts = (run: (texts: NonNullable<CommentController["texts"]>, args: unknown) => void) =>
    withArgs((c, args) => {
      if (c.texts !== undefined) run(c.texts, args);
    });
  function withArgs(run: (controller: CommentController, args: unknown) => void) {
    return (args: unknown): void => on((controller) => run(controller, args))();
  }
  const selection = (controller: CommentController) => controller.getState().selection;
  const family = (controller: CommentController) => kindOf(selection(controller)[0]?.subtype ?? "").family;
  return {
    "comment.select": tool("select"),
    "comment.highlight": tool("highlight"),
    "comment.underline": tool("underline"),
    "comment.strike": tool("strike"),
    "comment.caret": tool("caret"),
    "comment.note": tool("note"),
    "comment.ink": tool("ink"),
    "comment.shape": tool("shape"),
    "comment.text-box": text("textBox"),
    "comment.callout": text("callout"),
    "comment.typewriter": text("typewriter"),
    "comment.set-colour": withArgs((c, args) => c.setStyle({ colour: args as CommentColour })),
    "comment.width": withArgs((c, args) => c.setStyle({ width: Number(args) })),
    "comment.set-author": on((c) => c.askAuthor()),
    "annot.set-colour": withArgs((c, args) => c.changes.setColour(args as CommentColour, family(c))),
    "annot.set-fill": withArgs((c, args) => c.setFill(args as CommentColour)),
    "annot.set-width": withArgs((c, args) => c.changes.setWidth(Number(args))),
    "annot.set-opacity": withArgs((c, args) => c.changes.setOpacity(Number(args))),
    "annot.set-border-style": withArgs((c, args) => c.changes.setBorder(args as "solid" | "dashed" | "cloudy")),
    "annot.set-line-ends": on((c) => c.changes.setEnds(["none", "openArrow"])),
    "annot.set-smoothing": on((c) =>
      c.changes.setSmoothing(!(c.changes.known(selection(c)[0]?.id ?? "").smoothing ?? true)),
    ),
    "annot.delete": on((c) => c.changes.delete()),
    "annot.toggle-bold": texts((t) => t.toggle("bold")),
    "annot.toggle-italic": texts((t) => t.toggle("italic")),
    "annot.set-alignment": texts((t, args) => t.setLook({ align: args as TextAlignment })),
    "annot.set-font": texts((t, args) => t.setLook({ font: String(args) })),
    "annot.set-font-size": texts((t, args) => t.setLook({ size: Number(args) })),
    "annot.done-editing": texts((t) => t.done()),
  } as const;
}

/** Props of `CommentMode`. */
export interface CommentModeProps {
  readonly engine: Engine;
  readonly viewer: Viewer;
  readonly modes: ModeRegistry;
  readonly registry: CommandRegistry;
  readonly layout: ShellLayout;
  readonly flow: SaveFlow;
}

/** The Comment mode, registered while a document is open. */
export function CommentMode({ engine, viewer, modes, registry, layout, flow }: CommentModeProps): ReactNode {
  const [controller, setController] = useState<CommentController | undefined>(undefined);
  // The banner's copy goes through saving as it is now; a new flow keeps the mode and its state.
  const saving = useMemo(() => ({ flow }), []);
  useEffect(() => {
    saving.flow = flow;
  }, [saving, flow]);
  useEffect(() => {
    const controller = new CommentController(engine, viewer, toolsOf(viewer.edits), layout);
    controllers.set(viewer, controller);
    const unregister = modes.register({
      id: "comment",
      title: laterMessage("command.mode.comment"),
      label: laterMessage("mode.comment"),
      icon: "comment",
      toolbar: () => <CommentBar controller={controller} saving={saving} />,
      enter: () => controller.enter(),
      leave: () => controller.leave(),
    });
    const commands = registerCommentCommands(registry);
    setController(controller);
    return () => {
      setController(undefined);
      commands();
      controller.leave();
      unregister();
      controllers.delete(viewer);
    };
  }, [engine, viewer, modes, registry, layout, saving]);
  return controller === undefined ? null : <AuthorDialog controller={controller} />;
}

/** The toolbar, then the banner on a file whose certification forbids comments and the covered-text notice. */
function CommentBar({ controller, saving }: { controller: CommentController; saving: { flow: SaveFlow } }): ReactNode {
  const state = useStoreState(controller, (s) => s);
  const key = styleKey(state.tool, state.shape);
  const style = key === undefined ? undefined : state.styles[key];
  const copy = saving.flow.copyTitle().replace(/…$/, "");
  return (
    <div className="web-comment-bar" data-cover-checks={state.checks}>
      <CommentToolbar
        tool={state.tool}
        shape={state.shape}
        text={state.text}
        colours={key === undefined ? [] : swatches(familyOf(key))}
        colour={style?.colour}
        onColour={(colour) => controller.setStyle({ colour })}
        width={key !== undefined && takesWidth(key) ? style?.width : undefined}
        onWidth={(width) => controller.setStyle({ width })}
        count={state.count}
        hint={hint(state.tool, key, state.text)}
      />
      {state.banner === undefined ? null : (
        <section className="web-comment-banner" aria-label={laterMessage("comment.banner")}>
          <Notice tone="danger" title={laterMessage("comment.certified.title")}>
            {laterMessage(BANNER_WORDS[state.banner.kind], { copy })}
          </Notice>
          <Button size={30} onPress={() => void saving.flow.copy()}>
            {saving.flow.copyTitle()}
          </Button>
          <SignatureDetails restrictions={restrictionsOf(state.banner.violations)} />
        </section>
      )}
      {state.covered ? (
        <div className="web-comment-covered" role="status">
          <Notice tone="warning" title={laterMessage("comment.covered")} />
          <IconButton
            icon="close"
            label={laterMessage("comment.covered.dismiss")}
            size={26}
            onPress={() => controller.dismissCovered()}
          />
        </div>
      ) : null}
    </div>
  );
}

/** The banner's words for each signature that forbids comments. */
const BANNER_WORDS = {
  p1: "comment.certified.p1",
  p2: "comment.certified.p2",
  usageRights: "comment.usageRights",
} as const;

/** The one-line hint for the tool on (C01's status bar hint). */
function hint(tool: CommentTool, key: StyleKey | undefined, text: TextKind): string {
  if (tool === "select") return laterMessage("comment.hint.select");
  if (tool === "highlight" || tool === "underline" || tool === "strike") return laterMessage("comment.hint.text");
  if (tool === "caret" || tool === "note" || tool === "ink") return laterMessage(`comment.hint.${tool}`);
  if (tool === "text") return laterMessage(`comment.hint.${text}`);
  return laterMessage(key === "polygon" || key === "polyline" ? "comment.hint.poly" : "comment.hint.shape");
}

/** Asks for the name comments carry, the first time a mark is made; the mark waits for the answer. */
function AuthorDialog({ controller }: { controller: CommentController }): ReactNode {
  const asking = useStoreState(controller, (s) => s.askingAuthor);
  const [name, setName] = useState(() => controller.author() ?? "");
  return (
    <Dialog
      isOpen={asking}
      onOpenChange={(open) => {
        if (!open) controller.setAuthor(undefined);
      }}
      title={laterMessage("comment.author.title")}
      isRequired
      onEscape={() => controller.setAuthor("")}
      size="form"
      actions={
        <>
          <Button onPress={() => controller.setAuthor("")}>{laterMessage("comment.author.skip")}</Button>
          <Button variant="primary" onPress={() => controller.setAuthor(name)}>
            {laterMessage("comment.author.save")}
          </Button>
        </>
      }
    >
      <p>{laterMessage("comment.author.description")}</p>
      {/* A form, so Enter in the field uses the name, as the password dialog's does. */}
      <form
        onSubmit={(event) => {
          event.preventDefault();
          controller.setAuthor(name);
        }}
      >
        <TextInput label={laterMessage("comment.author.name")} value={name} onChange={setName} autoFocus />
      </form>
    </Dialog>
  );
}

/** The inspector for Comment's selection, while the shell's layout holds it (C02). */
export function CommentInspector({ viewer, layout }: { viewer: Viewer; layout: ShellLayout }): ReactNode {
  const controller = controllerOf(viewer);
  if (controller === undefined) return null;
  return (
    <Inspector layout={layout} label={laterMessage("inspector.label")} onDeselect={() => controller.clearSelection()}>
      <InspectorBody controller={controller} />
    </Inspector>
  );
}

/** The time an annotation was made, in the reader's words: "29 Oct 2026, 09:12". */
function madeAt(date: string | undefined): string | undefined {
  const m = date?.match(/^D:(\d{4})(\d{2})?(\d{2})?(\d{2})?(\d{2})?(\d{2})?/);
  if (m === null || m === undefined) return undefined;
  const [, y, mo = "01", d = "01", h = "00", mi = "00", s = "00"] = m;
  const when = new Date(Date.UTC(Number(y), Number(mo) - 1, Number(d), Number(h), Number(mi), Number(s)));
  return when.toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });
}

/** The inspector's fields for the selection. */
function InspectorBody({ controller }: { controller: CommentController }): ReactNode {
  const selection = useStoreState(controller, (s) => s.selection);
  // What the session learns of an annotation's drawing shows at once.
  useStoreState(controller, (s) => s.known);
  const page = useStoreState(controller, (s) => s.page);
  const noteFocus = useStoreState(controller, (s) => s.noteFocus);
  const textBox = useStoreState(controller, (s) => s.textBox);
  const note = useRef<HTMLTextAreaElement | null>(null);
  useEffect(() => {
    if (noteFocus > 0) note.current?.focus();
  }, [noteFocus]);
  const [first] = selection;
  // A new box being typed into has no annotation yet: the inspector shows its text fields alone.
  const props =
    first === undefined || (textBox !== undefined && textBox.id === undefined)
      ? textBox === undefined
        ? undefined
        : newBoxProps(controller, textBox)
      : inspectorProps(controller, selection, first);
  if (props === undefined) return null;
  return (
    <AnnotationInspector
      {...props}
      {...(page === undefined ? {} : { context: laterMessage("inspector.page", { page: page + 1 }) })}
      noteRef={note}
    />
  );
}

/** What the inspector shows for `selection`, whose first annotation is `first`. */
function inspectorProps(
  controller: CommentController,
  selection: readonly PageAnnotation[],
  first: PageAnnotation,
): Omit<AnnotationInspectorProps, "noteRef"> {
  const { family } = kindOf(first.subtype);
  const changes = controller.changes;
  const shared = {
    colours: swatches(family),
    onDelete: () => changes.delete(),
    ...((first.flags & (1 << 7)) === 0 ? {} : { locked: laterMessage("inspector.locked") }),
  };
  if (selection.length > 1) {
    return {
      ...shared,
      title: laterMessage("inspector.items", { count: selection.length }),
      colourLabel: laterMessage("inspector.colour"),
      colour: { value: undefined, onChange: (colour) => changes.setColour(colour, family) },
      opacity: { value: undefined, onChange: (o) => changes.setOpacity(o) },
      deleteLabel: laterMessage("inspector.delete.many", { count: selection.length }),
    };
  }
  const textBox = controller.getState().textBox;
  if (first.subtype === "FreeText" && textBox !== undefined && textBox.id === first.id) {
    return {
      ...shared,
      ...textBoxProps(controller, textBox),
      opacity: { value: Math.round((first.opacity ?? 1) * 100), onChange: (o) => changes.setOpacity(o) },
      author: authorOf(first),
      onChangeAuthor: () => controller.askAuthor(),
    };
  }
  const kind = kindName(first.subtype);
  const drawn = ["shape", "line", "poly"].includes(kindOf(first.subtype).patch ?? "");
  const author = authorOf(first);
  return {
    ...shared,
    ...drawingFields(controller, first),
    title: kind,
    colourLabel: laterMessage(drawn ? "inspector.lineColour" : "inspector.colour"),
    colour: { value: colourName(first.color, family), onChange: (colour) => changes.setColour(colour, family) },
    opacity: { value: Math.round((first.opacity ?? 1) * 100), onChange: (o) => changes.setOpacity(o) },
    note: { value: first.contents ?? "", onCommit: (text) => changes.setNote(text) },
    author,
    onChangeAuthor: () => controller.askAuthor(),
    deleteLabel: laterMessage("inspector.delete", { kind: kind.toLocaleLowerCase() }),
  };
}

/** "R. Osei · 29 Oct 2026, 09:12". */
function authorOf(a: PageAnnotation): string {
  return [a.author ?? laterMessage("inspector.author.unknown"), madeAt(a.created)]
    .filter((part) => part !== undefined)
    .join(" · ");
}

/** The inspector of a text box, callout or typewriter text (C03), being typed into or selected. */
function textBoxProps(
  controller: CommentController,
  state: TextState,
): Omit<AnnotationInspectorProps, "noteRef" | "opacity" | "onDelete" | "colours"> {
  const texts = controller.texts;
  const { look, kind } = state;
  const set = (change: Parameters<NonNullable<typeof texts>["setLook"]>[0]) => texts?.setLook(change);
  const name = laterMessage(`comment.text.${kind}`);
  return {
    title: name,
    colourLabel: laterMessage("inspector.textColour"),
    colour: { value: colourName(look.colour, "line"), onChange: (c) => set({ colour: colourValue(c, "line") }) },
    ...(kind === "typewriter"
      ? {}
      : {
          fill: {
            colours: swatches("fill"),
            value: look.fill === undefined ? "none" : colourName(look.fill, "fill"),
            onChange: (f) => set(f === "none" ? { fill: undefined } : { fill: colourValue(f, "fill") }),
          },
        }),
    text: {
      font: { value: look.font, onChange: (font) => set({ font }) },
      size: { value: look.size, onChange: (size) => set({ size }) },
      bold: { value: state.bold, onChange: () => texts?.toggle("bold") },
      italic: { value: state.italic, onChange: () => texts?.toggle("italic") },
      align: { value: look.align, onChange: (align) => set({ align }) },
      ...(kind === "typewriter" ? {} : { border: { value: look.border, onChange: (border) => set({ border }) } }),
      ...(kind === "callout" ? { end: { value: look.end as never, onChange: (end) => set({ end }) } } : {}),
      fallback: state.fallback,
      simplified: state.simplified,
      ...(state.typing ? { onDone: () => texts?.done() } : {}),
    },
    deleteLabel: laterMessage("inspector.delete", { kind: name.toLocaleLowerCase() }),
  };
}

/** The inspector of a new box being typed into: its text fields; Delete drops the box. */
function newBoxProps(controller: CommentController, state: TextState): Omit<AnnotationInspectorProps, "noteRef"> {
  return {
    ...textBoxProps(controller, state),
    colours: swatches("line"),
    opacity: { value: 100, onChange: () => undefined },
    onDelete: () => controller.texts?.cancel(),
  };
}

/** A drawing's own fields: fill, width, border, line endings, smoothing, as its kind has them. */
function drawingFields(
  controller: CommentController,
  first: PageAnnotation,
): Pick<AnnotationInspectorProps, "fill" | "width" | "border" | "cloudy" | "ends" | "smoothing"> {
  const { patch, closed = false, ends = false } = kindOf(first.subtype);
  const { changes } = controller;
  const known = changes.known(first.id);
  const drawsWidth = patch !== undefined && patch !== "caret";
  return {
    ...(closed
      ? { fill: { colours: swatches("fill"), value: known.fill, onChange: (fill) => controller.setFill(fill) } }
      : {}),
    ...(drawsWidth ? { width: { value: known.width, onChange: (w) => changes.setWidth(w) } } : {}),
    ...(closed || ends ? { border: { value: known.border, onChange: (b) => changes.setBorder(b) } } : {}),
    cloudy: closed,
    ...(ends ? { ends: { value: known.ends, onChange: (e) => changes.setEnds(e) } } : {}),
    ...(patch === "ink" ? { smoothing: { value: known.smoothing, onChange: (s) => changes.setSmoothing(s) } } : {}),
  };
}
