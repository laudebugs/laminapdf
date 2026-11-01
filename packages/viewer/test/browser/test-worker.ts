// The document worker the RPC tests start: the viewer's own methods, plus hooks the tests drive the
// queue with. None of them ship: the package's worker (src/worker/entry.ts) serves only
// viewerMethods.
import type { RenderRequest } from "@laminapdf/render";
import { serve } from "@laminapdf/sdk/worker/server";
import { viewerMethods } from "../../src/worker/methods.js";

/** Every render's scale, in the order the renders started (the priority tests read it). */
const renderLog: number[] = [];
const { render } = viewerMethods;
if (render === undefined) throw new Error("the viewer has no render method");

serve({
  methods: {
    ...viewerMethods,
    render: {
      ...render,
      run: (params, context) => {
        renderLog.push((params[0] as RenderRequest).scale);
        return render.run(params, context);
      },
    },
    // Blocks the worker synchronously for `ms`, as a long render does.
    __busy: {
      run: ([ms]) => {
        const until = performance.now() + (ms as number);
        while (performance.now() < until) {
          // Busy.
        }
        return undefined;
      },
    },
    __renderLog: { run: () => [...renderLog] },
    // An uncaught error: the page sees the worker's `error` event, a crash.
    __crash: {
      run: () => {
        setTimeout(() => {
          throw new Error("crash requested by a test");
        });
        return new Promise(() => {});
      },
    },
  },
});
