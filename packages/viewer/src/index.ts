// `@laminapdf/viewer` (ADR 0009): the web reader's viewer core: the document worker and its typed
// RPC (ViewerClient), and the viewer over them (createViewer). The parts inside (the layout, the
// canvas pool, the bitmap cache, the tile and zoom maths) are not exported: the README describes
// them, and their unit tests import them by path within the package.

export type { CanvasPoolOptions } from "./canvas/pool.js";
export type { RestartReason, ViewerCallOptions, ViewerEvent, ViewerEvents, ViewerOptions } from "./client.js";
export { ViewerClient } from "./client.js";
export type { Rotation } from "./layout/page-layout.js";
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
export type {
  CreateViewerOptions,
  PageLayer,
  PageRenderState,
  RenderStats,
  ViewerEventMap,
  ViewState,
} from "./view/viewer.js";
export { createViewer, Viewer } from "./view/viewer.js";
export type { DocumentPoint, Point, Zoom, ZoomMode } from "./zoom/zoom.js";
export { MAX_ZOOM, MIN_ZOOM } from "./zoom/zoom.js";
