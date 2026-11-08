// React's view of the stores the chrome reads: the viewer's (page, zoom, rotation) and the reader
// controller's lifecycle. Both are external stores with getState/subscribe, read through
// useSyncExternalStore with a selector, so a component re-renders only when its slice changes,
// not on every scroll frame.

import { useCallback, useEffect, useState, useSyncExternalStore } from "react";
import type { ReaderController } from "./reader-controller.js";
import type { ReaderState } from "./reader-machine.js";

/** A store React can subscribe to: the shape of `@laminapdf/viewer`'s viewer and of the controller. */
export interface ExternalStore<State> {
  /** The current state; the same object until it changes. */
  getState(): State;
  /** Calls `listener` after each change; returns the function that stops it. */
  subscribe(listener: () => void): () => void;
}

/**
 * The part of `store`'s state that `select` picks. Return a primitive or a value that stays the
 * same object while unchanged: a new object each call would re-render on every update.
 */
export function useStoreState<State, Slice>(store: ExternalStore<State>, select: (state: State) => Slice): Slice {
  // Called as methods: the viewer's are class methods that need their `this`.
  const subscribe = useCallback((listener: () => void) => store.subscribe(listener), [store]);
  return useSyncExternalStore(
    subscribe,
    () => select(store.getState()),
    () => select(store.getState()),
  );
}

/** The reader's lifecycle state. */
export function useReaderState(controller: ReaderController): ReaderState {
  return useStoreState(controller, (state) => state);
}

/** Whether `query` matches now, kept up to date. */
export function useMediaQuery(query: string): boolean {
  const [matches, setMatches] = useState(() => typeof matchMedia === "function" && matchMedia(query).matches);
  useEffect(() => {
    if (typeof matchMedia !== "function") return;
    const list = matchMedia(query);
    const update = () => setMatches(list.matches);
    update();
    list.addEventListener("change", update);
    return () => list.removeEventListener("change", update);
  }, [query]);
  return matches;
}
