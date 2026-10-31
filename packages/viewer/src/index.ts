// `@laminapdf/viewer` (ADR 0009): the web reader's viewer core. Part 1 (C3a): the document worker and
// its typed RPC. The virtualiser, tiles and zoom are C3b.
export type { RestartReason, ViewerCallOptions, ViewerEvent, ViewerEvents, ViewerOptions } from "./client.js";
export { DEFAULT_TIMEOUTS, ViewerClient } from "./client.js";
export type {
  BackendInit,
  BackendName,
  MethodName,
  Methods,
  OpenedDocument,
  Params,
  Priority,
  RenderedBitmap,
  Result,
  SourceInit,
  ViewerOpenOptions,
} from "./protocol.js";
export { PRIORITY, renderKey } from "./protocol.js";
export type { ByteSource } from "./source.js";
export { blobSource, bufferSource, readAll } from "./source.js";
