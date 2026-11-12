// The document worker the RPC tests start: the viewer's own methods, plus hooks the tests drive the
// queue with. None of them ship: the package's worker (src/worker/entry.ts) serves only
// viewerMethods.
import type { RenderDocument, RenderRequest } from "@laminapdf/render";
import { serve } from "@laminapdf/sdk/worker/server";
import { crashMethod } from "@laminapdf/test-utils/worker-hooks";
import { current, currentBackend } from "../../src/worker/document.js";
import { viewerMethods } from "../../src/worker/methods.js";

/** A number per render document the worker has drawn from, in the order the edit tests saw them. */
const renderDocuments = new WeakMap<RenderDocument, number>();
let nextRenderDocument = 1;

/** Every render's scale, in the order the renders started (the priority tests read it). */
const renderLog: number[] = [];
/** ms the next render holds the worker after drawing, still the render running (`__holdNextRender`). */
let holdNextRender = 0;
const { render } = viewerMethods;
if (render === undefined) throw new Error("the viewer has no render method");

serve({
  methods: {
    ...viewerMethods,
    render: {
      ...render,
      run: async (params, context) => {
        renderLog.push((params[0] as RenderRequest).scale);
        const drawn = await render.run(params, context);
        const hold = holdNextRender;
        holdNextRender = 0;
        const until = performance.now() + hold;
        while (performance.now() < until) {
          // Busy: the page has had this render's render-start, so it is the one running.
        }
        return drawn;
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
    // The next render holds the worker for `ms` once drawn, as a render that hangs does.
    __holdNextRender: {
      run: ([ms]) => {
        holdNextRender = ms as number;
        return undefined;
      },
    },
    // Which render document draws the open document's pages: a new number after an edit reopened it.
    __renderDocument: {
      run: () => {
        const { render: drawing } = current();
        if (!renderDocuments.has(drawing)) renderDocuments.set(drawing, nextRenderDocument++);
        return renderDocuments.get(drawing);
      },
    },
    __crash: crashMethod,
    // The backend's next open fails, as a reopen after an edit might.
    __failNextReopen: {
      run: () => {
        const backend = currentBackend();
        const open = backend.open;
        backend.open = () => {
          backend.open = open;
          throw new Error("the test made this reopen fail");
        };
        return undefined;
      },
    },
    // The model's next undo fails, as the undo of an edit that couldn't be shown might.
    __failNextUndo: {
      run: () => {
        const { model } = current();
        const undo = model.undo;
        model.undo = () => {
          model.undo = undo;
          throw new Error("the test made this undo fail");
        };
        return undefined;
      },
    },
  },
});
