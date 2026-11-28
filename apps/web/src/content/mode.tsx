// Edit mode (Paper A01, A02), registered while a document is open: its toolbar for adding text,
// pictures and shapes, the inspector of what was added, the symbol picker and the notice for a file
// that isn't a picture, and the handlers of its commands. The controller does the work; this is
// what shows it.

import {
  type CommandRegistry,
  formatBytes,
  Inspector,
  type ModeRegistry,
  type Platform,
  type ShellLayout,
  type ToastQueue,
  useStoreState,
} from "@laminapdf/ui";
import {
  AltTextDialog,
  blockedReason,
  ContentInspector,
  type ContentShape,
  ContentToolbar,
  laterMessage,
  registerContentCommands,
  SymbolPicker,
  type SymbolSection,
  UnsupportedImageDialog,
} from "@laminapdf/ui/editing";
import type { Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useState } from "react";
import { toolsOf } from "../edit-tools.tsx";
import { findSymbols, SYMBOL_GROUPS } from "../fonts/symbols.ts";
import { PageMarksController } from "../page-marks/controller.ts";
import { PageMarksDialogs } from "../page-marks/dialogs.tsx";
import type { Engine } from "../session.ts";
import { ContentController, hintFor } from "./controller.ts";
import { inspectorProps, textLook } from "./inspector-props.ts";
import { nameOf } from "./objects.ts";

/** Edit mode's id among the modes. */
const EDIT = "edit";

/** Each viewer's controller, once Edit mode has registered for it. */
const controllers = new WeakMap<Viewer, ContentController>();

/** Each viewer's page marks, with its content controller. */
const markControllers = new WeakMap<Viewer, PageMarksController>();

/** The symbol font's id and what the picker says of fetching it. */
const SYMBOL_FONT = "noto-sans-symbols-2";

/** Props of `ContentMode`. */
export interface ContentModeProps {
  readonly engine: Engine;
  readonly viewer: Viewer;
  readonly modes: ModeRegistry;
  readonly registry: CommandRegistry;
  readonly layout: ShellLayout;
  readonly platform: Platform;
  readonly toasts: ToastQueue;
}

/** Edit mode, registered while a document is open. */
export function ContentMode({
  engine,
  viewer,
  modes,
  registry,
  layout,
  platform,
  toasts,
}: ContentModeProps): ReactNode {
  const [controller, setController] = useState<ContentController | undefined>(undefined);
  const [marks, setMarks] = useState<PageMarksController | undefined>(undefined);
  useEffect(() => {
    const controller = new ContentController(engine, viewer, toolsOf(viewer.edits), layout, { platform, toasts });
    controllers.set(viewer, controller);
    const pageMarks = new PageMarksController(engine, viewer, controller, { platform, toasts });
    markControllers.set(viewer, pageMarks);
    const unregister = modes.register({
      id: EDIT,
      title: laterMessage("command.mode.edit"),
      label: laterMessage("mode.edit"),
      icon: "edit-text",
      toolbar: () => <ContentBar controller={controller} />,
      enter: () => controller.enter(),
      leave: () => controller.leave(),
    });
    const commands = registerContentCommands(registry);
    setController(controller);
    setMarks(pageMarks);
    return () => {
      setController(undefined);
      setMarks(undefined);
      commands();
      controller.leave();
      unregister();
      controllers.delete(viewer);
      markControllers.delete(viewer);
    };
  }, [engine, viewer, modes, registry, layout, platform, toasts]);
  if (controller === undefined || marks === undefined) return null;
  return (
    <>
      <Dialogs controller={controller} />
      <PageMarksDialogs controller={marks} />
    </>
  );
}

/** The toolbar. */
function ContentBar({ controller }: { controller: ContentController }): ReactNode {
  const tool = useStoreState(controller, (s) => s.tool);
  const shape = useStoreState(controller, (s) => s.shape);
  const selected = useStoreState(controller, (s) => s.selected);
  return (
    <>
      <ContentToolbar tool={tool} shape={shape} hint={hintFor(tool, shape)} />
      {/* What the handles show, for a screen reader: which object `[` and `]` landed on. */}
      <span className="paper-visually-hidden" role="status">
        {selected === undefined
          ? ""
          : laterMessage("content.object.selected", {
              kind: laterMessage(`content.${nameOf(selected)}` as "content.object.text"),
            })}
      </span>
    </>
  );
}

/** The symbol font while its file is still to be fetched (its size is what the picker names); `known` is only what makes this read again. */
function symbolFont(registry: ContentController["registry"], known: number) {
  return known < 0 ? undefined : registry.toFetch([SYMBOL_FONT])[0];
}

/** The symbol picker's groups, in the catalogue's words. */
function sections(): SymbolSection[] {
  return SYMBOL_GROUPS.map((group) => ({
    id: group.id,
    label: laterMessage(`content.symbols.${group.id}`),
    symbols: group.symbols,
  }));
}

/** The symbol picker and the refused file's notice. */
function Dialogs({ controller }: { controller: ContentController }): ReactNode {
  const open = useStoreState(controller, (s) => s.symbols);
  const unsupported = useStoreState(controller, (s) => s.unsupported);
  const asking = useStoreState(controller, (s) => s.askingAlt);
  // Read again once a font has been fetched (`known` changes), when its cost is gone.
  const known = useStoreState(controller, (s) => s.known);
  const font = symbolFont(controller.registry, known);
  return (
    <>
      <SymbolPicker
        open={open}
        sections={sections()}
        find={findSymbols}
        onPick={(character) => controller.insertSymbol(character)}
        onClose={() => controller.pickSymbol(false)}
        cost={
          font?.bytes === undefined
            ? undefined
            : laterMessage("content.symbols.cost", { font: font.family, size: formatBytes(font.bytes) })
        }
      />
      <AltTextDialog file={asking} onAnswer={(alt) => controller.answerAlt(alt)} />
      <UnsupportedImageDialog file={unsupported} onClose={() => controller.dismissUnsupported()} />
    </>
  );
}

/** The inspector for what Edit mode has selected or is typing, while the shell's layout holds it. */
export function ContentInspectorView({ viewer, layout }: { viewer: Viewer; layout: ShellLayout }): ReactNode {
  const controller = controllers.get(viewer);
  if (controller === undefined) return null;
  return (
    <Inspector layout={layout} label={laterMessage("content.inspector")} onDeselect={() => controller.clearSelection()}>
      <InspectorBody controller={controller} viewer={viewer} />
    </Inspector>
  );
}

/** What the inspector holds: the box being typed into, or the selected object. */
function InspectorBody({ controller, viewer }: { controller: ContentController; viewer: Viewer }): ReactNode {
  const state = useStoreState(controller, (s) => s);
  const blocked = useStoreState(viewer.edits, (s) => s.blocked);
  const locked = blockedReason(blocked);
  const { typing, selected } = state;
  if (typing !== undefined) {
    const found = typing.id === undefined ? undefined : controller.itemsOn(typing.page).find((i) => i.id === typing.id);
    return (
      <ContentInspector
        title={laterMessage("content.object.text")}
        context={
          typing.id === undefined
            ? laterMessage("content.context.new", { page: typing.page + 1 })
            : laterMessage("content.context", { page: (found?.page ?? typing.page) + 1 })
        }
        text={textLook(controller, {
          fontId: typing.look.fontId,
          size: typing.look.size,
          bold: typing.look.bold,
          italic: typing.look.italic,
          align: typing.look.align,
          colour: undefined,
          text: typing.text,
          apply: (change) => controller.setLook(change),
          addFont: () => void controller.addFont(),
        })}
        onDone={() => controller.done()}
        {...(locked === undefined ? {} : { locked })}
      />
    );
  }
  if (selected === undefined) return null;
  return (
    <ContentInspector
      {...inspectorProps(controller, selected, {
        keepRatio: state.keepRatio,
        known: state.known,
        locked,
        addFont: () => void controller.addFont(),
      })}
    />
  );
}

/**
 * The handlers of Edit mode's commands for the document `viewer` shows. Each switches to Edit
 * mode first, so the palette's Add text works in any mode, and finds the mode's controller when it
 * runs: the controller is made once the document is open.
 */
export function contentActions(viewer: Viewer, modes: ModeRegistry) {
  const on = (run: (controller: ContentController) => void) => () => {
    if (modes.getState().current !== EDIT) modes.switchTo(EDIT);
    const controller = controllers.get(viewer);
    if (controller !== undefined) run(controller);
  };
  const marks = (run: (marks: PageMarksController) => void) => () => {
    if (modes.getState().current !== EDIT) modes.switchTo(EDIT);
    const controller = markControllers.get(viewer);
    if (controller !== undefined) run(controller);
  };
  return {
    "tool.select": on((c) => c.useTool("select")),
    "tool.add-text": on((c) => c.useTool("text")),
    "tool.image": on((c) => c.useTool("image")),
    "tool.shape": (args?: unknown) =>
      on((c) => c.useTool("shape", args === undefined ? undefined : (args as ContentShape)))(),
    "edit.add-watermark": marks((m) => m.openAdd("watermark")),
    "edit.add-header-footer": marks((m) => m.openAdd("headerFooter")),
    "edit.add-bates": marks((m) => m.openAdd("bates")),
    "edit.manage-page-marks": marks((m) => void m.openManage()),
    "edit.insert-symbol": on((c) => c.pickSymbol(true)),
    "edit.replace-image": on((c) => {
      const item = c.getState().selected;
      if (item?.kind === "image") void c.pickImage(item);
    }),
    "edit.duplicate": on((c) => c.duplicateSelection()),
    "edit.delete-object": on((c) => c.deleteSelection()),
    "edit.bring-to-front": on((c) => c.arrangeSelection("front")),
    "edit.bring-forward": on((c) => c.arrangeSelection("forward")),
    "edit.send-backward": on((c) => c.arrangeSelection("backward")),
    "edit.send-to-back": on((c) => c.arrangeSelection("back")),
  } as const;
}
