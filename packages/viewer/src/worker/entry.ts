// The viewer's document worker (C3a): `@lamina/pdf/worker`'s server with the viewer's methods
// added, so it speaks the same protocol (handshake, queue, cancellation, errors, crash reports).
// Started by ViewerClient (`new Worker(new URL("./worker/entry.js", import.meta.url))`).
import { type ExtensionMethod, serve } from "@lamina/pdf/worker/server";
import type { RenderRequest } from "@lamina/render";
import type { BackendInit, SourceInit, ViewerOpenOptions } from "../protocol.js";
import { closeDocument, current, initBackend, openDocument } from "./document.js";
import { render, renderLog, reply } from "./render.js";

const methods: Record<string, ExtensionMethod> = {
  initBackend: { run: ([init]) => initBackend(init as BackendInit) },
  openDocument: {
    run: ([source, options], { signal }) => openDocument(source as SourceInit, options as ViewerOpenOptions, signal),
  },
  closeDocument: { run: () => closeDocument() },
  pageInfo: { run: ([index]) => current().model.pageInfo(index as number) },
  render: {
    run: ([request], { signal }) => render(request as RenderRequest, signal),
    reply: (tile) => reply(tile as Parameters<typeof reply>[0]),
  },
  __busy: {
    run: ([ms]) => {
      const until = performance.now() + (ms as number);
      while (performance.now() < until) {
        // Blocks the worker, as a long synchronous render does.
      }
      return undefined;
    },
  },
  __renderLog: { run: () => [...renderLog] },
};

serve({ methods });
