// Hooks a test worker adds to the worker protocol (`serve({ methods })` of
// `@laminapdf/sdk/worker/server`), shared by the packages whose browser tests start their own
// worker. Shaped like an extension method, without importing the SDK.

/** `__crash`: an uncaught error in the worker, which the page sees as the worker crashing. */
export const crashMethod = {
  run: (): Promise<never> => {
    setTimeout(() => {
      throw new Error("crash requested by a test");
    });
    return new Promise(() => {});
  },
};
