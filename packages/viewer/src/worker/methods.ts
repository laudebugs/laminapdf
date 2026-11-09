// The viewer's methods on the document worker: the extension `entry.ts` serves, and test
// workers too, which add their own hooks on top so none ships.

import type { RenderRequest } from "@laminapdf/render";
import type { ExtensionMethod } from "@laminapdf/sdk/worker/server";
import type { BackendInit, SourceInit, ViewerOpenOptions } from "../protocol.js";
import { closeDocument, current, initBackend, openDocument, workerMemory } from "./document.js";
import { render, reply } from "./render.js";
import { pageText, replyText, structure } from "./text.js";

/** The viewer's worker methods, by name (see `Methods` in protocol.ts). */
export const viewerMethods: Readonly<Record<string, ExtensionMethod>> = {
  initBackend: { run: ([init]) => initBackend(init as BackendInit) },
  openDocument: {
    run: ([source, options], { signal }) => openDocument(source as SourceInit, options as ViewerOpenOptions, signal),
  },
  closeDocument: { run: () => closeDocument() },
  pageInfo: {
    run: ([index], { signal }) => {
      const { model } = current();
      return model.ensure(() => model.pageInfo(index as number), { signal });
    },
  },
  render: {
    run: ([request, id], { signal }) => render(request as RenderRequest, id as number, signal),
    reply: (tile) => reply(tile as Parameters<typeof reply>[0]),
  },
  memory: { run: () => workerMemory() },
  pageText: {
    run: ([index], { signal }) => pageText(index as number, signal),
    reply: (text) => replyText(text as Parameters<typeof replyText>[0]),
  },
  structure: { run: (_params, { signal }) => structure(signal) },
};
