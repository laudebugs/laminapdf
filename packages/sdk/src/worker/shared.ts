// What both sides of the worker protocol use: which buffers a message can move, options without
// their undefined keys, and the event emitter (which `@laminapdf/sdk/worker` exports, for the
// packages that extend the worker).

/** The buffer of `value` if transferring it moves exactly `value` (not a larger or shared buffer). */
export function whole(value: unknown): ArrayBuffer[] {
  if (value instanceof ArrayBuffer) return [value];
  if (!ArrayBuffer.isView(value)) return [];
  const { buffer } = value;
  return buffer instanceof ArrayBuffer && value.byteOffset === 0 && value.byteLength === buffer.byteLength
    ? [buffer]
    : [];
}

/** `record` without its `undefined` members, so the result holds under `exactOptionalPropertyTypes`. */
export function defined<T extends object>(record: T): { [K in keyof T]?: Exclude<T[K], undefined> } {
  return Object.fromEntries(Object.entries(record).filter(([, v]) => v !== undefined)) as {
    [K in keyof T]?: Exclude<T[K], undefined>;
  };
}

/**
 * Listeners by event, for the events in `E` (event name → the data a listener gets). A listener
 * that throws is reported asynchronously and doesn't stop the others.
 */
export class Emitter<E extends object> {
  readonly #listeners = new Map<keyof E, Set<(data: never) => void>>();

  /** Listens for `event`; returns a function that stops listening. */
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

  /** Calls every listener of `event` with `data`. */
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
