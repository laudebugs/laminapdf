// The public API every export condition shares; each entry adds its own `init` and `loadText`
// (src/index.ts for browsers, Deno and the default; src/entry/ for Node and workerd).
export type {
  BlendMode,
  BuilderOptions,
  FillRule,
  PageSizeName,
  StrokeStyle,
  TextBoxOptions,
  TextOptions,
} from "./builder.js";
export { Builder, PAGE_SIZES, Page, PendingPath, Surface } from "./builder.js";
export type { ErrorCode, LimitName, WarningCode } from "./codes.generated.js";
export { Document, PageRef, SaveJob } from "./document.js";
export type { Diagnostic, ErrorKind, LaminaWarning, Location } from "./errors.js";
export { isLaminaError, LaminaError } from "./errors.js";
export type { FontInfo, Standard14 } from "./font.js";
export { Font, Image } from "./font.js";
export type { InitOptions, WasmMemory, WasmSource } from "./init.js";
export { isInitialized, isTextLoaded, wasmMemory } from "./init.js";
export type { Transform } from "./matrix.js";
export { Matrix } from "./matrix.js";
export type { PathBuilder } from "./path.js";
export type { InfoPatch, Rotation } from "./transaction.js";
export { Transaction } from "./transaction.js";
export type {
  Align,
  BoxRect,
  BuildOptions,
  Color,
  Direction,
  DocumentInfo,
  Edited,
  Fit,
  Limits,
  LineBox,
  LinkTarget,
  Metadata,
  Op,
  OpenOptions,
  OpLog,
  OutlineEntry,
  Overflow,
  PageInfo,
  PageLabel,
  Point,
  Progress,
  Rect,
  RenderMode,
  SaveOptions,
  SaveReport,
  TextLayout,
  TextRunInfo,
  UndoOutcome,
  ViewerPreferences,
} from "./types.js";
