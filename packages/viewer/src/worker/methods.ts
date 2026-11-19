// The viewer's methods on the document worker: the extension `entry.ts` serves, and test
// workers too, which add their own hooks on top so none ships.

import type { RenderRequest } from "@laminapdf/render";
import type { AttachmentFile, ContentsPlacement, Op, OutlineTarget, SaveOptions, TargetView } from "@laminapdf/sdk";
import type { ExtensionMethod } from "@laminapdf/sdk/worker/server";
import type { FindOptions } from "../find/fold.js";
import type {
  BackendInit,
  EditedCopy,
  ExportedPage,
  ExportedTiff,
  ExportPageOptions,
  ExportTiffOptions,
  ExtractedPictures,
  ExtractPicturesOptions,
  ExtractTextOptions,
  ImagesToPdfOptions,
  ImageToPdf,
  InspectionTab,
  MergeFilesRequest,
  PartsOutput,
  PrintOptions,
  ReplayedStep,
  SavedCopy,
  SourceInit,
  StreamRead,
  ViewerOpenOptions,
  WrittenParts,
} from "../protocol.js";
import { commentThreads, forgetAnnotations, pageAnnotations, soundAsWav, textBox } from "./annotations.js";
import { addBlob, readAttachmentBytes, readAttachmentLimit, readAttachments, readCollection } from "./attachments.js";
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
  copyWithEdit,
  createContentsPage,
  editHistory,
  editInTextCopy,
  goToStep,
  planSave,
  readContentsListing,
  readLabelRanges,
  readMetadata,
  readPageBoxes,
  readXmpPacket,
  restoreEdits,
  stepEdit,
  wouldViolate,
} from "./edit.js";
import { exportPage } from "./export.js";
import { exportTiff } from "./export-tiff.js";
import {
  exportSummary,
  extractPictures,
  extractText,
  forgetSummary,
  imagesToPdf,
  linksAndBookmarks,
} from "./extraction.js";
import { find, forgetSearch } from "./find.js";
import { forgetForm, formFields, tabOrder } from "./forms.js";
import { inspectionSummary, inspectionTab, objectDetail, objectReferrers, streamRead } from "./inspect.js";
import {
  destinationAt,
  forgetLinks,
  initialView,
  linkOnArea,
  linkOnText,
  namedDestination,
  namedDestinations,
  outline,
  pageLinks,
  threads,
} from "./navigation.js";
import { networkLog } from "./network.js";
import { mergeFiles, writeParts } from "./parts.js";
import { fileBytes, printPage, replyBytes } from "./print.js";
import { render, reply } from "./render.js";
import { addSource, closeSource, previewSource, renderSource } from "./sources.js";
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
      forgetAnnotations();
      forgetForm();
      forgetSummary();
      return closeDocument();
    },
  },
  applyEdit: {
    run: ([ops, id, label], { signal }) =>
      applyEdit(ops as readonly Op[], id as number, signal, label as string | undefined),
  },
  editInTextCopy: {
    run: ([text, id]) => editInTextCopy(text as boolean, id as number).then(() => undefined),
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
  xmpPacket: { run: ([id], { signal }) => readXmpPacket(id as number, signal) },
  attachments: { run: ([id], { signal }) => readAttachments(id as number, signal) },
  attachmentBytes: {
    run: ([attachment, id], { signal }) => readAttachmentBytes(attachment as string, id as number, signal),
    reply: (file) => {
      const { bytes, checksum } = file as AttachmentFile;
      const [own, transfer] = replyBytes(bytes);
      return [{ bytes: own, checksum }, transfer];
    },
  },
  collection: { run: ([id], { signal }) => readCollection(id as number, signal) },
  attachmentLimit: { run: ([id]) => readAttachmentLimit(id as number) },
  addBlob: { run: ([bytes, id]) => addBlob(bytes as Uint8Array, id as number) },
  pageBoxes: {
    run: ([index, id], { signal }) => readPageBoxes(index as number, id as number, signal),
  },
  labelRanges: { run: ([id], { signal }) => readLabelRanges(id as number, signal) },
  contentsListing: {
    run: ([placement, id, font], { signal }) =>
      readContentsListing(placement as ContentsPlacement, id as number, signal, font as Uint8Array | undefined),
  },
  createContentsPage: {
    run: ([placement, font, id, label], { signal }) =>
      createContentsPage(
        placement as ContentsPlacement,
        font as Uint8Array,
        id as number,
        signal,
        label as string | undefined,
      ),
  },
  planSave: {
    run: ([options, id], { signal }) => planSave(options as SaveOptions, id as number, signal),
  },
  wouldViolate: {
    run: ([ops, id], { signal }) => wouldViolate(ops as readonly Op[], id as number, signal),
  },
  copyEdits: {
    run: ([options, id], { signal }) => copyEdits(options as SaveOptions, id as number, signal),
    reply: (copy) => {
      const { token, bytes, report } = copy as SavedCopy;
      const [own, transfer] = replyBytes(bytes);
      return [{ token, bytes: own, report }, transfer];
    },
  },
  copyWithEdit: {
    run: ([ops, options, id], { signal }) =>
      copyWithEdit(ops as readonly Op[], options as SaveOptions, id as number, signal),
    reply: (copy) => {
      const { bytes, report, history } = copy as EditedCopy;
      const [own, transfer] = replyBytes(bytes);
      return [{ bytes: own, report, history }, transfer];
    },
  },
  adoptEdits: {
    run: ([token, id]) => adoptEdits(token as number, id as number),
  },
  restoreEdits: {
    run: ([steps, undone, id, sources, blobs], { signal }) =>
      restoreEdits(
        steps as readonly ReplayedStep[],
        undone as number,
        id as number,
        signal,
        (sources as readonly Uint8Array[] | undefined) ?? [],
        (blobs as readonly Uint8Array[] | undefined) ?? [],
      ),
  },
  previewSource: {
    run: ([bytes, password]) => previewSource(bytes as Uint8Array, password as string | undefined),
  },
  renderSource: {
    run: ([key, page, scale], { signal }) => renderSource(key as number, page as number, scale as number, signal),
    reply: (tile) => reply(tile as Parameters<typeof reply>[0]),
  },
  closeSource: { run: ([key]) => closeSource(key as number) },
  addSource: {
    run: ([bytes, password, id], { signal }) =>
      addSource(bytes as Uint8Array, password as string | undefined, id as number, signal),
  },
  writeParts: {
    run: ([ranges, output, id], context) =>
      writeParts(ranges as readonly (readonly string[])[], output as PartsOutput, id as number, context),
    // Called without a coalescing key, so one caller takes the buffers: moved, never copied.
    reply: (parts) => {
      const written = parts as WrittenParts;
      const buffers = "zip" in written ? [written.zip.buffer] : written.files.map((file) => file.buffer);
      return [written, buffers];
    },
  },
  mergeFiles: {
    run: ([request], context) => mergeFiles(request as MergeFilesRequest, context),
    reply: (bytes) => replyBytes(bytes as Uint8Array),
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
  initialView: { run: (_params, { signal }) => initialView(signal) },
  threads: { run: (_params, { signal }) => threads(signal) },
  namedDestinations: { run: (_, { signal }) => namedDestinations(signal) },
  destinationAt: { run: ([page, view], { signal }) => destinationAt(page as number, view as TargetView, signal) },
  linkOnArea: {
    run: ([page, area, target], { signal }) =>
      linkOnArea(page as number, area as readonly [number, number, number, number], target as OutlineTarget, signal),
  },
  linkOnText: {
    run: ([page, lines, target], { signal }) =>
      linkOnText(
        page as number,
        lines as readonly (readonly [number, number, number, number])[],
        target as OutlineTarget,
        signal,
      ),
  },
  pageLinks: { run: ([index], { signal }) => pageLinks(index as number, signal) },
  pageAnnotations: { run: ([index], { signal }) => pageAnnotations(index as number, signal) },
  formFields: { run: (_, { signal }) => formFields(signal) },
  tabOrder: { run: ([index], { signal }) => tabOrder(index as number, signal) },
  textBox: { run: ([id], { signal }) => textBox(id as string, signal) },
  commentThreads: { run: (_params, { signal }) => commentThreads(signal) },
  soundAsWav: {
    run: ([id], { signal }) => soundAsWav(id as string, signal),
    reply: (wav) => replyBytes(wav as Uint8Array),
  },
  inspectionSummary: { run: (_params, { signal }) => inspectionSummary(signal) },
  inspectionTab: { run: ([tab], { signal }) => inspectionTab(tab as InspectionTab, signal) },
  objectDetail: { run: ([num], { signal }) => objectDetail(num as number, signal) },
  objectReferrers: { run: ([num], { signal }) => objectReferrers(num as number, signal) },
  streamRead: {
    run: ([num, view, max], { signal }) => streamRead(num as number, view as "raw" | "decoded", max as number, signal),
    reply: (read) => {
      if (read === undefined) return [read, []];
      const { bytes, total } = read as StreamRead;
      const [own, transfer] = replyBytes(bytes);
      return [{ bytes: own, total }, transfer];
    },
  },
  properties: {
    run: async (_params, { signal }) => {
      const model = await textModel(signal);
      return model.ensure(() => model.properties(), { signal });
    },
  },
  printPage: {
    run: ([index, options], { signal }) => printPage(index as number, options as PrintOptions, signal),
  },
  exportTiff: {
    run: ([indexes, options], { signal }) =>
      exportTiff(indexes as readonly number[], options as ExportTiffOptions, signal),
    reply: (tiff) => {
      const exported = tiff as ExportedTiff;
      return [exported, exported.files.map((file) => file.buffer)];
    },
  },
  exportPage: {
    run: ([index, options], { signal }) => exportPage(index as number, options as ExportPageOptions, signal),
    reply: (page) => {
      const exported = page as ExportedPage;
      return [exported, [exported.image.buffer]];
    },
  },
  extractText: {
    run: ([options], { signal }) => extractText(options as ExtractTextOptions, signal),
  },
  extractPictures: {
    run: ([options], { signal }) => extractPictures(options as ExtractPicturesOptions, signal),
    // The pictures' buffers are the SDK's copies or PNGs made for them: moved, never copied (each once).
    reply: (read) => {
      const pictures = read as ExtractedPictures;
      return [pictures, [...new Set(pictures.pictures.map((picture) => picture.bytes.buffer))]];
    },
  },
  exportSummary: { run: (_params, { signal }) => exportSummary(signal) },
  linksAndBookmarks: { run: ([format], { signal }) => linksAndBookmarks(format as "json" | "csv", signal) },
  imagesToPdf: {
    run: ([images, options], { signal }) =>
      imagesToPdf(images as readonly ImageToPdf[], options as ImagesToPdfOptions, signal),
    reply: (bytes) => [bytes, [(bytes as Uint8Array<ArrayBuffer>).buffer]],
  },
  fileBytes: {
    run: (_params, { signal }) => fileBytes(signal),
    reply: (bytes) => replyBytes(bytes as Uint8Array),
  },
};
