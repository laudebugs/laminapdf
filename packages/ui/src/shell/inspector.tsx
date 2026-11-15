// The inspector (UI/UX plan §5.1, design language §8.3): the complementary landmark docked on the
// right, which opens on a selection and never covers the canvas: the canvas gives it the width
// instead. A selection's inspector is a layer, so Escape deselects; it is the one panel focus mode
// shows (ADR 0024 §1: a selection brings the inspector back). A panel opened on purpose (repair
// details) hides in focus mode, and closes on Escape from inside it or with its own button.
// Given a `label`, the inspector is the landmark itself; without one, its content brings its own
// (`RepairDetails` is an `aside`), so the page never nests two complementary landmarks. Closed,
// or taken off the page by the app (ReaderShell keeps it for this), it stays for its exit with
// what it showed last, marked data-exiting and inert, while it slides and fades out.

import { type KeyboardEvent, type ReactNode, useEffect } from "react";
import { message } from "../messages.js";
import { LayerScope } from "../overlays/layers.js";
import { exitingAttributes, Presence, useExiting } from "../overlays/presence.js";
import { BottomSheet } from "../overlays/sheet.js";
import { useStoreState } from "../state/hooks.js";
import type { InspectorContent, ShellLayout } from "./layout.js";
import { usePhone } from "./phone.js";

/** Props of `Inspector`. */
export interface InspectorProps {
  readonly layout: ShellLayout;
  /** The landmark's name, the panel's title ("Highlight"); leave it out when the content is a landmark itself. */
  readonly label?: string;
  /** Clears the app's own selection when the shell deselects (Escape, leaving a mode). */
  readonly onDeselect?: () => void;
  /** What it shows. */
  readonly children: ReactNode;
}

/** The docked inspector, while `layout` says it is open; draws nothing otherwise. */
export function Inspector({ layout, label, onDeselect, children }: InspectorProps): ReactNode {
  const live = useStoreState(layout, (s) => s.inspector);
  const focus = useStoreState(layout, (s) => s.focus);
  // Taken off the page by the app, it leaves as if closed (ReaderShell keeps it for that).
  const removed = useExiting();
  useEffect(() => (onDeselect === undefined ? undefined : layout.onDeselect(onDeselect)), [layout, onDeselect]);
  const shown = live !== undefined && !(focus && !live.selection) && !removed;
  return (
    <Presence value={shown ? live : null} exit="panel">
      {(content) => (
        <InspectorPanel layout={layout} content={content} {...(label === undefined ? {} : { label })}>
          {children}
        </InspectorPanel>
      )}
    </Presence>
  );
}

/** The inspector showing `content`: docked, or a sheet on a phone; marked leaving while it goes. */
function InspectorPanel({
  layout,
  label,
  content,
  children,
}: Omit<InspectorProps, "onDeselect"> & { readonly content: InspectorContent }): ReactNode {
  const phone = usePhone();
  const exiting = useExiting();
  if (phone) {
    // A non-modal sheet has no exit of its own: it goes at once.
    if (exiting) return null;
    // A phone has no room beside the page: the inspector is a sheet over its lower half that leaves
    // the page in use above it (ADR 0024 §4: the selection stays where it can be seen).
    const close = () => (content.selection ? layout.deselect() : layout.closeInspector());
    return (
      <BottomSheet
        isOpen
        isModal={false}
        onOpenChange={(isOpen) => (isOpen ? undefined : close())}
        title={label ?? message("shell.inspector")}
        closeLabel={message("shell.inspector.close")}
        resizeLabel={message("phone.sheet.resize")}
      >
        {children}
      </BottomSheet>
    );
  }
  const onKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    if (event.key !== "Escape" || event.defaultPrevented || content.selection || exiting) return;
    event.preventDefault();
    layout.closeInspector();
  };
  // The panel itself takes Escape, so a key pressed in any of its controls closes it, as in a dialog.
  const panel =
    label === undefined ? (
      // biome-ignore lint/a11y/noStaticElementInteractions: Escape anywhere inside the panel closes it.
      <div className="paper-inspector" onKeyDown={onKeyDown} {...exitingAttributes(exiting)}>
        {children}
      </div>
    ) : (
      <aside className="paper-inspector" aria-label={label} onKeyDown={onKeyDown} {...exitingAttributes(exiting)}>
        {children}
      </aside>
    );
  // Leaving, the panel keeps its place in the tree (so its exit runs) but is off the layer stack.
  return content.selection ? (
    <LayerScope dismiss={() => layout.deselect()} away={exiting}>
      {panel}
    </LayerScope>
  ) : (
    panel
  );
}
