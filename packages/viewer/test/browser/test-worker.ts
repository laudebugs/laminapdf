// The document worker the RPC tests start: the viewer's own methods, plus hooks the tests drive the
// queue with. None of them ship: the package's worker (src/worker/entry.ts) serves only
// viewerMethods.
import type { RenderDocument, RenderRequest } from "@laminapdf/render";
import { serve } from "@laminapdf/sdk/worker/server";
import { crashMethod } from "@laminapdf/test-utils/worker-hooks";
import { current } from "../../src/worker/document.js";
import { viewerMethods } from "../../src/worker/methods.js";

/** A number per render document the worker has drawn from, in the order the edit tests saw them. */
const renderDocuments = new WeakMap<RenderDocument, number>();
let nextRenderDocument = 1;

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
    // Blocks the worker synchronously for `ms`, as a long render does, and answers with the wall
    // clock when the hold ends, so a test can tell whether what it sent meanwhile arrived in time.
    __busy: {
      run: ([ms]) => {
        const until = performance.now() + (ms as number);
        while (performance.now() < until) {
          // Busy.
        }
        return Date.now();
      },
    },
    __renderLog: { run: () => [...renderLog] },
    // Which render document draws the open document's pages: a new number after an edit reopened it.
    __renderDocument: {
      run: () => {
        const { render: drawing } = current();
        if (!renderDocuments.has(drawing)) renderDocuments.set(drawing, nextRenderDocument++);
        return renderDocuments.get(drawing);
      },
    },
    __crash: crashMethod,
  },
});
