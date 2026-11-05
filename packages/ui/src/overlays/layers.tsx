// The app's one layer stack, and the scope that puts a layer on it while the layer is open. Each
// scope tells the layers opened inside it (through context, which crosses portals) that it is
// their parent, so the stack's order follows how the layers were opened.

import { createContext, type ReactNode, useContext, useEffect, useRef, useState } from "react";
import { type Layer, LayerStack } from "./layer-stack.js";

const STACK = new LayerStack();
const ParentLayer = createContext<Layer | undefined>(undefined);

// Escape that no layer handled itself (React Aria's overlays prevent the default when they do).
function onKeyDown(event: KeyboardEvent): void {
  if (event.key !== "Escape" || event.defaultPrevented || event.isComposing) return;
  if (STACK.escape()) event.preventDefault();
}

/** Props of `LayerScope`. */
interface LayerScopeProps {
  /** Closes the layer. */
  dismiss: () => void;
  /** False while the layer asks a required question. */
  dismissable?: boolean;
  /** The layer's content. */
  children: ReactNode;
}

/**
 * Inside a `LayerScope`: a function telling whether a layer opened from that scope, such as a menu
 * in a sheet, is open now. Effects register layers, so read it after they have run.
 */
export function useLayerOpenedHere(): () => boolean {
  const layer = useContext(ParentLayer);
  return () => layer !== undefined && STACK.hasOpenedFrom(layer);
}

/** Keeps a layer on the stack for as long as it is mounted, and is the parent of layers opened inside it. */
export function LayerScope({ dismiss, dismissable = true, children }: LayerScopeProps): ReactNode {
  const parent = useContext(ParentLayer);
  const latest = useRef({ dismiss, dismissable });
  latest.current = { dismiss, dismissable };
  // One layer object for the scope's life, so a change of `dismissable` never moves it in the stack.
  const [layer] = useState<Layer>(() => ({
    dismiss: () => latest.current.dismiss(),
    get dismissable() {
      return latest.current.dismissable;
    },
    parent,
  }));
  useEffect(() => {
    const remove = STACK.open(layer);
    if (STACK.size === 1) document.addEventListener("keydown", onKeyDown);
    return () => {
      remove();
      if (STACK.size === 0) document.removeEventListener("keydown", onKeyDown);
    };
  }, [layer]);
  return <ParentLayer.Provider value={layer}>{children}</ParentLayer.Provider>;
}
