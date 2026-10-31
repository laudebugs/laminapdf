// The records of the API: the tsify types `lamina-wasm` generates (ADR 0016 §7), re-exported
// under @laminapdf/sdk's names. They are the Rust façade's records, camelCase; only `OpenOptions`
// and `Color` are written here.
import type {
  AlignInit,
  BuildOptionsInit,
  DirectionInit,
  DocumentInfoJs,
  EditedJs,
  FitInit,
  LimitsInit,
  LineBoxJs,
  LinkTargetInit,
  MetadataInit,
  Op as OpJs,
  OutlineEntryInit,
  OverflowInit,
  PageInfoJs,
  PageLabelInit,
  ProgressJs,
  RectJs,
  RenderModeInit,
  SaveOptionsInit,
  SaveReportJs,
  TextLayoutJs,
  TextRunJs,
  UndoOutcomeJs,
  ViewerPreferencesInit,
} from "@laminapdf/wasm/text";

/** An edit (ADR 0006 §6: `{ op: "setInfo" | "rotatePage", args }`), as in `fixtures/ops/v1`. */
export type Op = OpJs;
/** An op log: `{ v: 1, ops }`. */
export interface OpLog {
  readonly v: 1;
  readonly ops: readonly Op[];
}
/** `Limits` (ADR 0003 §3); each one left out keeps its default. */
export type Limits = LimitsInit;
/** Options for {@link Document.open}. */
export interface OpenOptions {
  /** The user or owner password of an encrypted document (normalised to NFKC, ADR 0016 §7). */
  readonly password?: string;
  /** Refuse files that need repair (`strict`). */
  readonly strict?: boolean;
  /** Resource limits. */
  readonly limits?: Limits;
}
/** Options for saving. */
export type SaveOptions = SaveOptionsInit;
/** What a save wrote. */
export type SaveReport = SaveReportJs;
/** A page's facts: index, ID, displayed size in points, rotation, label. */
export type PageInfo = PageInfoJs;
/** The Info dictionary's text entries (document text). */
export type DocumentInfo = DocumentInfoJs;
/** The pages an edit touched. */
export type Edited = EditedJs;
/** How an undo or redo went. */
export type UndoOutcome = UndoOutcomeJs;
/** A save job's progress. */
export type Progress = ProgressJs;
/** Options for a new document: version, fixed file ID (32 hex digits), producer, creation date. */
export type BuildOptions = BuildOptionsInit;
/** Where a link or outline item goes. */
export type LinkTarget = LinkTargetInit;
/** How a destination shows its page. */
export type Fit = FitInit;
/** An outline (bookmarks) item with its children. */
export type OutlineEntry = OutlineEntryInit;
/** A page-label range, by its first page. */
export type PageLabel = PageLabelInit;
/** The document's metadata (Info and XMP). */
export type Metadata = MetadataInit;
/** How viewers present the document. */
export type ViewerPreferences = ViewerPreferencesInit;
/** A rectangle returned by the builder (page space). */
export type BoxRect = RectJs;
/** What a text run measures. */
export type TextRunInfo = TextRunJs;
/** How a text box laid its text out. */
export type TextLayout = TextLayoutJs;
/** A line of a text box. */
export type LineBox = LineBoxJs;
/** Text alignment in a text box. */
export type Align = AlignInit;
/** Paragraph direction. */
export type Direction = DirectionInit;
/** What happens to text that doesn't fit a text box. */
export type Overflow = OverflowInit;
/** Text rendering mode. */
export type RenderMode = RenderModeInit;

/**
 * A colour: `[gray]`, `[r, g, b]` or `[c, m, y, k]` with components from 0 to 1, or `"#rrggbb"`
 * / `"#rgb"` (DeviceRGB).
 */
export type Color =
  | string
  | readonly [number]
  | readonly [number, number, number]
  | readonly [number, number, number, number];
/** A point in page space (points from the top-left of the displayed page, y down; ADR 0027). */
export type Point = readonly [x: number, y: number];
/** A rectangle in page space: left, top, width, height. */
export type Rect = readonly [x: number, y: number, width: number, height: number];
