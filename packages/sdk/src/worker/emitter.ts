// The typed event emitter the worker client uses, exported from `@laminapdf/sdk/worker` for the
// packages that wrap the client (the viewer's ViewerClient).

/**
 * Listeners by event, for the events in `E` (event name → the data a listener gets). A listener
 * that throws is reported asynchronously and doesn't stop the others.
 *
 * @since 0.2.0
 */
export class Emitter<E extends object> {
  readonly #listeners = new Map<keyof E, Set<(data: never) => void>>();

  /**
   * Listens for `event`; returns a function that stops listening.
   *
   * @since 0.2.0
   */
  on<K extends keyof E>(event: K, listener: (data: E[K]) => void): () => void {
    let set = this.#listeners.get(event);
    if (set === undefined) {
      set = new Set();
      this.#listeners.set(event, set);
    }
    const l = listener as (data: never) => void;
    set.add(l);
    return () => set.delete(l);
  }

  /**
   * Calls every listener of `event` with `data`.
   *
   * @since 0.2.0
   */
  emit<K extends keyof E>(event: K, data: E[K]): void {
    for (const listener of this.#listeners.get(event) ?? []) {
      try {
        (listener as (d: E[K]) => void)(data);
      } catch (e) {
        queueMicrotask(() => {
          throw e;
        });
      }
    }
  }
}
