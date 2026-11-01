// Records without their `undefined` members: the SDK's option objects go through it before they
// reach the WebAssembly or cross to the worker.

/** `record` without its `undefined` members, so the result holds under `exactOptionalPropertyTypes`. */
export function defined<T extends object>(record: T): { [K in keyof T]?: Exclude<T[K], undefined> } {
  return Object.fromEntries(Object.entries(record).filter(([, v]) => v !== undefined)) as {
    [K in keyof T]?: Exclude<T[K], undefined>;
  };
}
