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
  SHAPE_KINDS,
  type ShapeKind,
  SignatureDetails,
} from "@laminapdf/ui/editing";
import type { Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useRef, useState } from "react";
import { toolsOf } from "../edit-tools.tsx";
import { restrictionsOf } from "../organise.tsx";
import type { SaveFlow } from "../save-flow.ts";
import type { Engine } from "../session.ts";
import {
  CommentController,
  familyOf,
  kindName,
  type StyleKey,
  styleKey,
  takesWidth,
} from "./controller.ts";
import { colourName, patchType } from "./marks.ts";

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
  const withArgs =
    (run: (controller: CommentController, args: unknown) => void) =>
    (args: unknown): void =>
      on((controller) => run(controller, args))();
  const selection = (controller: CommentController) => controller.getState().selection;
  const family = (controller: CommentController): "fill" | "line" => {
    const [first] = selection(controller);
    return first?.subtype === "Highlight" || first?.subtype === "Text" ? "fill" : "line";
  };
  return {
    "comment.select": tool("select"),
    "comment.highlight": tool("highlight"),
    "comment.underline": tool("underline"),
    "comment.strike": tool("strike"),
    "comment.caret": tool("caret"),
    "comment.note": tool("note"),
    "comment.ink": tool("ink"),
    "comment.shape": tool("shape"),
    "comment.set-colour": withArgs((c, args) => c.setStyle({ colour: args as CommentColour })),
    "comment.width": withArgs((c, args) => c.setStyle({ width: Number(args) })),
    "comment.set-author": on((c) => c.askAuthor()),
    "annot.set-colour": withArgs((c, args) => c.setColour(args as CommentColour, family(c))),
    "annot.set-fill": withArgs((c, args) => c.setFill(args as CommentColour)),
    "annot.set-width": withArgs((c, args) => c.setWidth(Number(args))),
    "annot.set-opacity": withArgs((c, args) => c.setOpacity(Number(args))),
    "annot.set-border-style": withArgs((c, args) => c.setBorder(args as "solid" | "dashed" | "cloudy")),
    "annot.set-line-ends": on((c) => c.setEnds(["none", "openArrow"])),
    "annot.set-smoothing": on((c) => c.setSmoothing(!(c.known(selection(c)[0]?.id ?? "").smoothing ?? true))),
    "annot.delete": on((c) => c.deleteSelection()),
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
  useEffect(() => {
    const controller = new CommentController(engine, viewer, toolsOf(viewer.edits), layout);
    controllers.set(viewer, controller);
    const unregister = modes.register({
      id: "comment",
      title: laterMessage("command.mode.comment"),
      label: laterMessage("mode.comment"),
      icon: "comment",
      toolbar: () => <CommentBar controller={controller} flow={flow} />,
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
  }, [engine, viewer, modes, registry, layout, flow]);
  return controller === undefined ? null : <AuthorDialog controller={controller} />;
}

/** The toolbar, then the banner on a file whose certification forbids comments and the covered-text notice. */
function CommentBar({ controller, flow }: { controller: CommentController; flow: SaveFlow }): ReactNode {
  const state = useStoreState(controller, (s) => s);
  const key = styleKey(state.tool, state.shape);
  const style = key === undefined ? undefined : state.styles[key];
  return (
    <div className="web-comment-bar">
      <CommentToolbar
        tool={state.tool}
        shape={state.shape}
        colours={key === undefined ? [] : swatches(familyOf(key))}
        colour={style?.colour}
        onColour={(colour) => controller.setStyle({ colour })}
        width={key !== undefined && takesWidth(key) ? style?.width : undefined}
        onWidth={(width) => controller.setStyle({ width })}
        count={state.count}
      />
      <p className="paper-visually-hidden" role="status">
        {hint(state.tool, key)}
      </p>
      {state.banner === undefined ? null : (
        <section className="web-comment-banner" aria-label={laterMessage("comment.banner")}>
          <Notice tone="danger" title={laterMessage("comment.certified.title")}>
            {laterMessage(
              state.banner.kind === "usageRights"
                ? "comment.usageRights"
                : state.banner.kind === "p1"
                  ? "comment.certified.p1"
                  : "comment.certified.p2",
            )}
          </Notice>
          <Button size={30} onPress={() => void flow.copy()}>
            {flow.copyTitle()}
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

/** The one-line hint for the tool on (C01's status bar hint). */
function hint(tool: CommentTool, key: StyleKey | undefined): string {
  if (tool === "select") return laterMessage("comment.hint.select");
  if (tool === "highlight" || tool === "underline" || tool === "strike") return laterMessage("comment.hint.text");
  if (tool === "caret" || tool === "note" || tool === "ink") return laterMessage(`comment.hint.${tool}`);
  return laterMessage(key === "polygon" || key === "polyline" ? "comment.hint.poly" : "comment.hint.shape");
}

/** Asks for the name comments carry, the first time a tool is chosen. */
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
      <TextInput label={laterMessage("comment.author.name")} value={name} onChange={setName} autoFocus />
    </Dialog>
  );
}

/** The inspector for Comment's selection, while the shell's layout holds it (C02). */
export function CommentInspector({ viewer, layout }: { viewer: Viewer; layout: ShellLayout }): ReactNode {
  const controller = controllerOf(viewer);
  if (controller === undefined) return null;
  return (
    <Inspector layout={layout} label={laterMessage("inspector.label")} onDeselect={() => undefined}>
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
  const page = useStoreState(controller, (s) => s.page);
  const noteFocus = useStoreState(controller, (s) => s.noteFocus);
  const note = useRef<HTMLTextAreaElement | null>(null);
  useEffect(() => {
    if (noteFocus > 0) note.current?.focus();
  }, [noteFocus]);
  const [first] = selection;
  if (first === undefined) return null;
  return (
    <AnnotationInspector
      {...inspectorProps(controller, selection, first)}
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
  const family: "fill" | "line" = first.subtype === "Highlight" || first.subtype === "Text" ? "fill" : "line";
  const shared = {
    colours: swatches(family),
    onDelete: () => controller.deleteSelection(),
    ...((first.flags & (1 << 7)) === 0 ? {} : { locked: laterMessage("inspector.locked") }),
  };
  if (selection.length > 1) {
    return {
      ...shared,
      title: laterMessage("inspector.items", { count: selection.length }),
      colourLabel: laterMessage("inspector.colour"),
      colour: { value: undefined, onChange: (colour) => controller.setColour(colour, family) },
      opacity: { value: undefined, onChange: (o) => controller.setOpacity(o) },
      deleteLabel: laterMessage("inspector.delete.many", { count: selection.length }),
    };
  }
  const kind = kindName(first.subtype);
  const drawn = ["shape", "line", "poly"].includes(patchType(first.subtype) ?? "");
  const author = [first.author ?? laterMessage("inspector.author.unknown"), madeAt(first.created)]
    .filter((part) => part !== undefined)
    .join(" · ");
  return {
    ...shared,
    ...drawingFields(controller, first),
    title: kind,
    colourLabel: laterMessage(drawn ? "inspector.lineColour" : "inspector.colour"),
    colour: { value: colourName(first.color, family), onChange: (colour) => controller.setColour(colour, family) },
    opacity: { value: Math.round((first.opacity ?? 1) * 100), onChange: (o) => controller.setOpacity(o) },
    note: { value: first.contents ?? "", onCommit: (text) => controller.setNote(text) },
    author,
    deleteLabel: laterMessage("inspector.delete", { kind: kind.toLocaleLowerCase() }),
  };
}

/** A drawing's own fields: fill, width, border, line endings, smoothing, as its kind has them. */
function drawingFields(
  controller: CommentController,
  first: PageAnnotation,
): Pick<AnnotationInspectorProps, "fill" | "width" | "border" | "cloudy" | "ends" | "smoothing"> {
  const type = patchType(first.subtype);
  const known = controller.known(first.id);
  const closed = type === "shape" || first.subtype === "Polygon";
  const open = type === "line" || first.subtype === "PolyLine";
  return {
    ...(closed
      ? { fill: { colours: swatches("fill"), value: known.fill, onChange: (fill) => controller.setFill(fill) } }
      : {}),
    ...(type === undefined ? {} : { width: { value: known.width, onChange: (w) => controller.setWidth(w) } }),
    ...(closed || open ? { border: { value: known.border, onChange: (b) => controller.setBorder(b) } } : {}),
    cloudy: closed,
    ...(open ? { ends: { value: known.ends, onChange: (ends) => controller.setEnds(ends) } } : {}),
    ...(type === "ink" ? { smoothing: { value: known.smoothing, onChange: (s) => controller.setSmoothing(s) } } : {}),
  };
}
