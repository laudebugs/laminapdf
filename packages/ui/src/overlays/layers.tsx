// The app's one layer stack, and the hook that puts a layer on it while the layer is open.

import { useEffect, useRef } from "react";
import { LayerStack } from "./layer-stack.js";

const STACK = new LayerStack();

// Escape that no layer handled itself (React Aria's overlays prevent the default when they do).
function onKeyDown(event: KeyboardEvent): void {
  if (event.key !== "Escape" || event.defaultPrevented || event.isComposing) return;
  if (STACK.escape()) event.preventDefault();
}

/** Puts a layer on the stack while `isOpen`; `dismiss` closes it when Escape reaches it. */
function useLayer(isOpen: boolean, dismiss: () => void, dismissable = true): void {
  const latest = useRef(dismiss);
  latest.current = dismiss;
  useEffect(() => {
    if (!isOpen) return;
    const remove = STACK.open({ dismiss: () => latest.current(), dismissable });
    if (STACK.size === 1) document.addEventListener("keydown", onKeyDown);
    return () => {
      remove();
      if (STACK.size === 0) document.removeEventListener("keydown", onKeyDown);
    };
  }, [isOpen, dismissable]);
}

/** Registers the layer it is rendered in for as long as it is mounted. */
export function LayerRegistration({
  dismiss,
  dismissable = true,
}: {
  dismiss: () => void;
  dismissable?: boolean;
}): null {
  useLayer(true, dismiss, dismissable);
  return null;
}
