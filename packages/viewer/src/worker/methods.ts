// The viewer's methods on the document worker: the extension `entry.ts` serves, and test
// workers too, which add their own hooks on top so none ships.

import type { RenderRequest } from "@laminapdf/render";
import type { ExtensionMethod } from "@laminapdf/sdk/worker/server";
import type { FindOptions } from "../find/fold.js";
import type { BackendInit, PrintOptions, SourceInit, ViewerOpenOptions } from "../protocol.js";
import {
  closeDocument,
  current,
  initBackend,
  keepDocument,
  openDocument,
  textModel,
  workerMemory,
} from "./document.js";
import { find, forgetSearch } from "./find.js";
import { forgetLinks, namedDestination, outline, pageLinks } from "./navigation.js";
import { networkLog } from "./network.js";
import { fileBytes, printPage, replyBytes } from "./print.js";
import { render, reply } from "./render.js";
import { forgetText, pageText, readingOrder, replyText, structure } from "./text.js";

/** The viewer's worker methods, by name (see `Methods` in protocol.ts). */
export const viewerMethods: Readonly<Record<string, ExtensionMethod>> = {
  initBackend: { run: ([init]) => initBackend(init as BackendInit) },
  openDocument: {
    run: ([source, options], { signal }) => openDocument(source as SourceInit, options as ViewerOpenOptions, signal),
  },
  keepDocument: { run: ([id]) => keepDocument(id as number) },
  closeDocument: {
    run: () => {
      forgetText();
      forgetSearch();
      forgetLinks();
      return closeDocument();
    },
  },
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
  networkLog: { run: () => networkLog() },
  pageText: {
    run: ([index], { signal }) => pageText(index as number, signal),
    reply: (text) => replyText(text as Parameters<typeof replyText>[0]),
  },
  structure: { run: (_params, { signal }) => structure(signal) },
  readingOrder: { run: ([index], { signal }) => readingOrder(index as number, signal) },
  find: {
    run: ([query, options, pages], { signal }) =>
      find(query as string, options as FindOptions, pages as readonly number[], signal),
  },
  outline: { run: (_params, { signal }) => outline(signal) },
  namedDestination: { run: ([name], { signal }) => namedDestination(name as string, signal) },
  pageLinks: { run: ([index], { signal }) => pageLinks(index as number, signal) },
  properties: {
    run: async (_params, { signal }) => {
      const model = await textModel(signal);
      return model.ensure(() => model.properties(), { signal });
    },
  },
  printPage: {
    run: ([index, options], { signal }) => printPage(index as number, options as PrintOptions, signal),
  },
  fileBytes: {
    run: (_params, { signal }) => fileBytes(signal),
    reply: (bytes) => replyBytes(bytes as Uint8Array),
  },
};
