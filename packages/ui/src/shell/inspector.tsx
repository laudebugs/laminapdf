// The inspector (UI/UX plan §5.1, design language §8.3): the complementary landmark docked on the
// right, which opens on a selection and never covers the canvas: the canvas gives it the width
// instead. A selection's inspector is a layer, so Escape deselects; it shows in focus mode too (R16).
// A panel opened on purpose (repair details) closes on Escape from inside it, or with its own button.
// Given a `label`, the inspector is the landmark itself; without one, its content brings its own
// (`RepairDetails` is an `aside`), so the page never nests two complementary landmarks.

import type { KeyboardEvent, ReactNode } from "react";
import { LayerScope } from "../overlays/layers.js";
import { useStoreState } from "../state/hooks.js";
import type { ShellLayout } from "./layout.js";

/** Props of `Inspector`. */
export interface InspectorProps {
  readonly layout: ShellLayout;
  /** The landmark's name, the panel's title ("Highlight"); leave it out when the content is a landmark itself. */
  readonly label?: string;
  /** Clears the app's own selection when Escape deselects (the canvas's selected object). */
  readonly onDeselect?: () => void;
  /** What it shows. */
  readonly children: ReactNode;
}

/** The docked inspector, while `layout` says it is open; draws nothing otherwise. */
export function Inspector({ layout, label, onDeselect, children }: InspectorProps): ReactNode {
  const content = useStoreState(layout, (s) => s.inspector);
  if (content === undefined) return null;
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
  const deselect = () => {
    onDeselect?.();
    layout.deselect();
  };
  return content.selection ? <LayerScope dismiss={deselect}>{panel}</LayerScope> : panel;
}
