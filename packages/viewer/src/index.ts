// `@laminapdf/viewer` (ADR 0009): the web reader's viewer core: the document worker and its typed
// RPC, and the viewer over them (virtualiser, canvas pool, progressive tiles, anchored zoom).

export type { CachedBitmap } from "./canvas/cache.js";
export { BitmapCache, DEFAULT_CACHE_BYTES } from "./canvas/cache.js";
export type { CanvasPoolOptions } from "./canvas/pool.js";
export { CanvasPool, DEFAULT_POOL } from "./canvas/pool.js";
export type { RestartReason, ViewerCallOptions, ViewerEvent, ViewerEvents, ViewerOptions } from "./client.js";
export { DEFAULT_TIMEOUTS, ViewerClient } from "./client.js";
export type { LayoutOptions, PageSize, Rotation } from "./layout/page-layout.js";
export { CSS_PX_PER_PT, PageLayout } from "./layout/page-layout.js";
export type {
  BackendInit,
  BackendName,
  MethodName,
  Methods,
  OpenedDocument,
  Params,
  Priority,
  RenderedBitmap,
  RenderKeyFields,
  Result,
  SourceInit,
  ViewerOpenOptions,
} from "./protocol.js";
export { PRIORITY, renderKey, renderKeyFields } from "./protocol.js";
export { LOW_MAX_WIDTH, lowScale, TILE_CSS, tilesFor } from "./render/tiles.js";
export type { ByteSource } from "./source.js";
export { blobSource, bufferSource, readAll } from "./source.js";
export type {
  CreateViewerOptions,
  PageLayer,
  PageRenderState,
  RenderStats,
  ViewerEventMap,
  ViewState,
} from "./view/viewer.js";
export { createViewer, Viewer } from "./view/viewer.js";
export type { DocumentPoint, Scroll, Viewport, Zoom, ZoomMode } from "./zoom/zoom.js";
export { MAX_ZOOM, MIN_ZOOM, percentOfScale, pointAt, scaleFor, scaleOfPercent, scrollToPoint } from "./zoom/zoom.js";
