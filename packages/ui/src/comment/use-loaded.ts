// A value a dialog reads from outside while it is open, again when what it depends on changes
// (the Markdown preview, an import's summary): loading, ready or failed, and the answer of an
// older read never replaces a newer one's.

import { useEffect, useState } from "react";

/** A read's state. */
export type Loaded<T> =
  | { readonly state: "loading" }
  | { readonly state: "ready"; readonly value: T }
  | { readonly state: "failed"; readonly error: unknown };

/** The loading state, for a read not started. */
const LOADING: Loaded<never> = { state: "loading" };

/**
 * Runs `load` while `enabled`, and again whenever `key` changes. A new read starts from loading.
 * `load` is called by the effect only, so it may close over what `key` stands for.
 */
export function useLoaded<T>(enabled: boolean, key: string, load: () => Promise<T>): Loaded<T> {
  const [result, setResult] = useState<{ readonly key: string; readonly loaded: Loaded<T> } | undefined>(undefined);
  // biome-ignore lint/correctness/useExhaustiveDependencies: `key` stands for everything `load` reads.
  useEffect(() => {
    if (!enabled) return;
    let current = true;
    load().then(
      (value) => current && setResult({ key, loaded: { state: "ready", value } }),
      (error: unknown) => current && setResult({ key, loaded: { state: "failed", error } }),
    );
    return () => {
      current = false;
    };
  }, [enabled, key]);
  return enabled && result?.key === key ? result.loaded : LOADING;
}
