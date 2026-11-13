// The viewer's methods on the document worker: the extension `entry.ts` serves, and test
// workers too, which add their own hooks on top so none ships.

import type { RenderRequest } from "@laminapdf/render";
import type { Op, SaveOptions, TargetView } from "@laminapdf/sdk";
import type { ExtensionMethod } from "@laminapdf/sdk/worker/server";
import type { FindOptions } from "../find/fold.js";
import type { BackendInit, PrintOptions, ReplayedStep, SavedCopy, SourceInit, ViewerOpenOptions } from "../protocol.js";
import {
  closeDocument,
  current,
  currentDocument,
  initBackend,
  openDocument,
  textModel,
  workerMemory,
} from "./document.js";
import {
  adoptEdits,
  applyEdit,
  copyEdits,
  editHistory,
  goToStep,
  planSave,
  readMetadata,
  restoreEdits,
  stepEdit,
} from "./edit.js";
import { find, forgetSearch } from "./find.js";
import { destinationAt, forgetLinks, namedDestination, namedDestinations, outline, pageLinks } from "./navigation.js";
import { networkLog } from "./network.js";
import { fileBytes, printPage, replyBytes } from "./print.js";
import { render, reply } from "./render.js";
import { forgetText, pageText, readingOrder, reflow, replyText, structure } from "./text.js";

/** The viewer's worker methods, by name (see `Methods` in protocol.ts). */
export const viewerMethods: Readonly<Record<string, ExtensionMethod>> = {
  initBackend: { run: ([init]) => initBackend(init as BackendInit) },
  openDocument: {
    run: ([source, options, ticket], { signal }) =>
      openDocument(source as SourceInit, options as ViewerOpenOptions, signal, ticket as number | undefined),
  },
  currentDocument: { run: () => currentDocument() },
  closeDocument: {
    run: () => {
      forgetText();
      forgetSearch();
      forgetLinks();
      return closeDocument();
    },
  },
  applyEdit: {
    run: ([ops, id, label], { signal }) =>
      applyEdit(ops as readonly Op[], id as number, signal, label as string | undefined),
  },
  undoEdit: {
    run: ([id], { signal }) => stepEdit(-1, id as number, signal),
  },
  redoEdit: {
    run: ([id], { signal }) => stepEdit(1, id as number, signal),
  },
  goToStep: {
    run: ([position, id], { signal }) => goToStep(position as number, id as number, signal),
  },
  editHistory: {
    run: ([id]) => editHistory(id as number),
  },
  metadata: { run: ([id], { signal }) => readMetadata(id as number, signal) },
  planSave: {
    run: ([options, id], { signal }) => planSave(options as SaveOptions, id as number, signal),
  },
  copyEdits: {
    run: ([options, id], { signal }) => copyEdits(options as SaveOptions, id as number, signal),
    reply: (copy) => {
      const { token, bytes, report } = copy as SavedCopy;
      const [own, transfer] = replyBytes(bytes);
      return [{ token, bytes: own, report }, transfer];
    },
  },
  adoptEdits: {
    run: ([token, id]) => adoptEdits(token as number, id as number),
  },
  restoreEdits: {
    run: ([steps, undone, id], { signal }) =>
      restoreEdits(steps as readonly ReplayedStep[], undone as number, id as number, signal),
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
  reflow: { run: (_params, { signal }) => reflow(signal) },
  readingOrder: { run: ([index], { signal }) => readingOrder(index as number, signal) },
  find: {
    run: ([query, options, pages], { signal }) =>
      find(query as string, options as FindOptions, pages as readonly number[], signal),
  },
  outline: { run: (_params, { signal }) => outline(signal) },
  namedDestination: { run: ([name], { signal }) => namedDestination(name as string, signal) },
  namedDestinations: { run: (_, { signal }) => namedDestinations(signal) },
  destinationAt: { run: ([page, view], { signal }) => destinationAt(page as number, view as TargetView, signal) },
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
