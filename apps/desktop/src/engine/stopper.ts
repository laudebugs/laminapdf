// Listening that starts asynchronously (Tauri's `listen` resolves to the unsubscribe) but may be
// stopped at once: a React effect's cleanup, or a watch dropped before it began.

/**
 * Returns a stop for `listening` that works whenever it is called: before `listening` resolves,
 * the unsubscribe runs as soon as it arrives. A failure to start is reported with `what`.
 */
export function stopper(listening: Promise<() => void>, what: string): () => void {
  let stop: (() => void) | undefined;
  let stopped = false;
  listening.then(
    (unsubscribe) => {
      if (stopped) unsubscribe();
      else stop = unsubscribe;
    },
    (error: unknown) => console.error(`${what} failed`, error),
  );
  return () => {
    stopped = true;
    stop?.();
  };
}
