// The inspector (UI/UX plan §5.1, design language §8.3): the complementary landmark docked on the
// right, which opens on a selection and never covers the canvas: the canvas gives it the width
// instead. A selection's inspector is a layer, so Escape deselects; it is the one panel focus mode
// shows (ADR 0024 §1: a selection brings the inspector back). A panel opened on purpose (repair
// details) hides in focus mode, and closes on Escape from inside it or with its own button.
// Given a `label`, the inspector is the landmark itself; without one, its content brings its own
// (`RepairDetails` is an `aside`), so the page never nests two complementary landmarks.

import { type KeyboardEvent, type ReactNode, useEffect } from "react";
import { message } from "../messages.js";
import { LayerScope } from "../overlays/layers.js";
import { BottomSheet } from "../overlays/sheet.js";
import { useStoreState } from "../state/hooks.js";
import type { ShellLayout } from "./layout.js";
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
  const content = useStoreState(layout, (s) => s.inspector);
  const focus = useStoreState(layout, (s) => s.focus);
  const phone = usePhone();
  useEffect(() => (onDeselect === undefined ? undefined : layout.onDeselect(onDeselect)), [layout, onDeselect]);
  if (content === undefined || (focus && !content.selection)) return null;
  if (phone) {
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
    if (event.key !== "Escape" || event.defaultPrevented || content.selection) return;
    event.preventDefault();
    layout.closeInspector();
  };
  // The panel itself takes Escape, so a key pressed in any of its controls closes it, as in a dialog.
  const panel =
    label === undefined ? (
      // biome-ignore lint/a11y/noStaticElementInteractions: Escape anywhere inside the panel closes it.
      <div className="paper-inspector" onKeyDown={onKeyDown}>
        {children}
      </div>
    ) : (
      <aside className="paper-inspector" aria-label={label} onKeyDown={onKeyDown}>
        {children}
      </aside>
    );
  return content.selection ? <LayerScope dismiss={() => layout.deselect()}>{panel}</LayerScope> : panel;
}
