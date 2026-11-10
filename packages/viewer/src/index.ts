// `@laminapdf/viewer` (ADR 0009): the web reader's viewer core: the document worker and its typed
// RPC (ViewerClient), and the viewer over them (createViewer). The parts inside (the layout, the
// canvas pool, the bitmap cache, the tile and zoom maths) are not exported: the README describes
// them, and their unit tests import them by path within the package.

export type { CanvasPoolOptions } from "./canvas/pool.js";
export type { RestartReason, ViewerCallOptions, ViewerEvent, ViewerEvents, ViewerOptions } from "./client.js";
export { ViewerClient } from "./client.js";
export type { FindController, FindResult, FindState } from "./find/controller.js";
export type { FindOptions } from "./find/fold.js";
export type { FindSnippet } from "./find/page.js";
export type {
  HistoryEntry,
  HistoryPlace,
  HistoryState,
  NavigationHistory,
  NavigationHistoryOptions,
} from "./history/history.js";
export type { CopyMode } from "./layers/copy.js";
export type { LinkLayerOptions, LinkWords } from "./layers/link-layer.js";
export { type ExternalLink, externalLink, type OpenableLink } from "./layers/link-url.js";
export type { TextLayerOptions, TextSelection } from "./layers/text-layer.js";
export type { PageSize, Rotation } from "./layout/page-layout.js";
export { type NetworkSink, watchNetwork } from "./network/watch.js";
export type {
  BackendInit,
  BackendName,
  EditResult,
  FindPageResult,
  LoadProgress,
  MethodName,
  Methods,
  NetworkRecord,
  NetworkSource,
  OpenedDocument,
  PageTextData,
  Params,
  PolicyViolation,
  PrintedPage,
  PrintOptions,
  Priority,
  ReadingBlock,
  RemoteSource,
  RenderedBitmap,
  Result,
  SourceInit,
  StructureData,
  TextRunData,
  ViewerOpenOptions,
  WorkerMemory,
  WorkerNetworkLog,
} from "./protocol.js";
export { CACHE_FOLDER } from "./source/opfs.js";
export { ThumbnailStrip, type ThumbnailStripOptions } from "./thumbnails/strip.js";
export type {
  CreateViewerOptions,
  InvalidatedPage,
  PageLayer,
  PageRegionOptions,
  PageRenderState,
  RenderStats,
  ViewerEventMap,
  ViewState,
} from "./view/viewer.js";
export { createViewer, Viewer } from "./view/viewer.js";
export type { DocumentPoint, Point, Zoom, ZoomMode } from "./zoom/zoom.js";
export { MAX_ZOOM, MIN_ZOOM } from "./zoom/zoom.js";
