// Edit mode (Paper A01, A02), registered while a document is open: its toolbar for adding text,
// pictures and shapes, the inspector of what was added, the symbol picker and the notice for a file
// that isn't a picture, and the handlers of its commands. The controller does the work; this is
// what shows it.

import {
  type CommandRegistry,
  Inspector,
  type ModeRegistry,
  type Platform,
  type ShellLayout,
  type ToastQueue,
  useStoreState,
} from "@laminapdf/ui";
import {
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
import type { Engine } from "../session.ts";
import { ContentController, hintFor } from "./controller.ts";
import { inspectorProps, textLook } from "./inspector-props.ts";

/** Edit mode's id among the modes. */
export const EDIT = "edit";

/** Each viewer's controller, once Edit mode has registered for it. */
const controllers = new WeakMap<Viewer, ContentController>();

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
  useEffect(() => {
    const controller = new ContentController(engine, viewer, toolsOf(viewer.edits), layout, { platform, toasts });
    controllers.set(viewer, controller);
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
    return () => {
      setController(undefined);
      commands();
      controller.leave();
      unregister();
      controllers.delete(viewer);
    };
  }, [engine, viewer, modes, registry, layout, platform, toasts]);
  return controller === undefined ? null : <Dialogs controller={controller} />;
}

/** The toolbar. */
function ContentBar({ controller }: { controller: ContentController }): ReactNode {
  const tool = useStoreState(controller, (s) => s.tool);
  const shape = useStoreState(controller, (s) => s.shape);
  return <ContentToolbar tool={tool} shape={shape} hint={hintFor(tool, shape)} />;
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
  // Read again once a font has been fetched (`known` changes), when its cost is gone.
  const known = useStoreState(controller, (s) => s.known);
  const font = known >= 0 ? controller.registry.toFetch([SYMBOL_FONT])[0] : undefined;
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
            : laterMessage("content.symbols.cost", { font: font.family, size: `${Math.round(font.bytes / 1000)} KB` })
        }
      />
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
  return {
    "tool.select": on((c) => c.useTool("select")),
    "tool.add-text": on((c) => c.useTool("text")),
    "tool.image": on((c) => c.useTool("image")),
    "tool.shape": (args?: unknown) =>
      on((c) => c.useTool("shape", args === undefined ? undefined : (args as ContentShape)))(),
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
