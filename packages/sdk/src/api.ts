// The public API every export condition shares; each entry adds its own `init` and `loadText`
// (src/index.ts for browsers, Deno and the default; src/entry/ for Node and workerd).
export type {
  ArtifactKind,
  BlendMode,
  BuilderOptions,
  FillRule,
  FlowOptions,
  FlowReport,
  FlowTemplate,
  LineCap,
  LineJoin,
  MarkdownFonts,
  MarkdownOptions,
  MarkdownReport,
  PageRegion,
  PageSizeName,
  PageTemplate,
  StrokeStyle,
  TableScope,
  Tag,
  TagName,
  TagSpec,
  Templated,
  TextBoxOptions,
  TextOptions,
} from "./builder.js";
export { Builder, PAGE_SIZES, Page, PendingPath, Surface, TAG_NAMES } from "./builder.js";
export { Document, PageRef, SaveJob } from "./document.js";
export type { Diagnostic, ErrorKind, LaminaPdfWarning, Location } from "./errors.js";
export { isLaminaPdfError, LaminaPdfError } from "./errors.js";
export type {
  Cell,
  Chunk,
  ChunkOptions,
  Coordinates,
  Dropped,
  DropReason,
  ExtractedBlock,
  ExtractedPage,
  ExtractedSource,
  ExtractFormat,
  Extraction,
  ExtractOptions,
  ExtractResult,
  Flags,
  Generator,
  ReadingOrder,
  Region,
  Span,
  SplitOn,
  StructRef,
  StructSource,
  TableDetector,
  TableMode,
  TextLayer,
  TextLayerStatus,
  TextRange,
} from "./extract.js";
export type {
  ExtractJobOptions,
  ExtractStep,
  HostModels,
  LayoutRegion,
  ModelInfo,
  ModelRequest,
  ModelResult,
  Models,
  OcrWord,
  RegionLabel,
} from "./extract-job.js";
export { ExtractJob, extractWithModels } from "./extract-job.js";
export type { FontInfo, Standard14 } from "./font.js";
export { Font, Image } from "./font.js";
export type { ErrorCode, LimitName, WarningCode } from "./generated/codes.js";
export type * from "./generated/document.js";
export type { InitOptions, WasmMemory, WasmSource } from "./init.js";
export { isExtractLoaded, isInitialized, isTextLoaded, reset, wasmMemory } from "./init.js";
export type { Leak } from "./internal/handle.js";
export { onLeak } from "./internal/handle.js";
export type { Transform } from "./matrix.js";
export { Matrix } from "./matrix.js";
export type { PathBuilder } from "./path.js";
export type {
  PageImageReason,
  Reflow,
  ReflowBlock,
  ReflowChecks,
  ReflowKind,
  ReflowRefusal,
  ReflowSource,
} from "./reflow.js";
export type { HttpRangeOptions } from "./source/http.js";
export { httpRangeSource } from "./source/http.js";
export type { ByteRange, RangeOptions, RangeSource } from "./source/range.js";
export { callbackSource } from "./source/range.js";
export type { GlyphQuad, QuadPoint, TextGlyph, TextOptions as PageTextOptions } from "./text.js";
export { PageText, PageTextRun } from "./text.js";
export type { InfoPatch, Rotation } from "./transaction.js";
export {
  type DeletePagesOptions,
  type InsertBlankPagesOptions,
  type MetadataEditOptions,
  type MetadataRemoveOptions,
  type PagePosition,
  type PageSize,
  type ReferencePolicy,
  type RotateMode,
  type RotatePagesOptions,
  Transaction,
  type XmpEditOptions,
} from "./transaction.js";
export type {
  AddLink,
  AfRelationship,
  Align,
  AttachmentOptions,
  BoxRect,
  BuildOptions,
  ChildItems,
  Color,
  DestinationUse,
  DestinationView,
  Direction,
  DocumentInfo,
  DocumentMetadata,
  DocumentProperties,
  Edited,
  ExplicitDestination,
  ExtensionProperty,
  FieldEdit,
  Fit,
  Limits,
  LineBox,
  LinkTarget,
  Metadata,
  MetadataKey,
  MetadataPatch,
  NamedDestination,
  Op,
  OpenOptions,
  OpLog,
  OutlineEntry,
  OutlineNode,
  OutlineStyle,
  OutlineTarget,
  Overflow,
  PageAnnotation,
  PageInfo,
  PageLabel,
  PageLabelRange,
  PageLink,
  PlannedSave,
  Point,
  Profile,
  Progress,
  Rect,
  ReferenceEdit,
  RenameLinks,
  RenderMode,
  Reverted,
  SaveOptions,
  SaveReport,
  StructureKid,
  StructureNode,
  Target,
  TargetView,
  TextLayout,
  TextRunInfo,
  UndoOutcome,
  ViewerPreferences,
  Violation,
  XmpLangText,
  XmpText,
  XmpValue,
} from "./types.js";
