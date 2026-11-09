// The records of the API: the tsify types `laminapdf-wasm` generates (ADR 0016 §7), re-exported
// under @laminapdf/sdk's names. They are the Rust façade's records, camelCase; only `OpenOptions`
// and `Color` are written here.
import type {
  AfRelationshipInit,
  AlignInit,
  AttachmentInit,
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
  OutlineNodeJs,
  OverflowInit,
  PageInfoJs,
  PageLabelInit,
  PageLinkJs,
  ProfileInit,
  ProgressJs,
  PropertiesJs,
  RectJs,
  RenderModeInit,
  SaveOptionsInit,
  SaveReportJs,
  StructureKidJs,
  StructureNodeJs,
  TargetJs,
  TargetViewJs,
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
/**
 * An element of a tagged document's structure tree: its standard type (`H1`, `P`, `LI`,
 * `Figure` …), alternate text, language, actual text, and kids in reading order (document text).
 */
export type StructureNode = StructureNodeJs;
/** A kid of a {@link StructureNode}: an element, or marked content `mcid` on page `page`. */
export type StructureKid = StructureKidJs;
/** The pages an edit touched. */
export type Edited = EditedJs;
/** How an undo or redo went. */
export type UndoOutcome = UndoOutcomeJs;
/** A save job's progress. */
export type Progress = ProgressJs;
/** Options for a new document: version, fixed file ID (32 hex digits), producer, creation date. */
export type BuildOptions = BuildOptionsInit;
/** A conformance profile: `PDF/UA-1` (accessible), `PDF/A-2b`, `PDF/A-2u` or `PDF/A-3b` (archival). */
export type Profile = ProfileInit;
/** How an attached file relates to the document (`/AFRelationship`). */
export type AfRelationship = AfRelationshipInit;
/** An attached file's relationship, description and modification date. */
export type AttachmentOptions = AttachmentInit;
/** Where a link or outline item goes. */
export type LinkTarget = LinkTargetInit;
/** How a destination shows its page. */
export type Fit = FitInit;
/** An outline (bookmarks) item with its children. */
export type OutlineEntry = OutlineEntryInit;

/** One item of a document's outline (its bookmarks), with where it goes and its children. */
export type OutlineNode = OutlineNodeJs;

/**
 * One link on a page, in page space: its `rect` (`[x0, y0, x1, y1]`), the `quads` that follow it
 * when the file gives them (eight numbers per area), its `contents` (a description), and what it
 * does by `kind`: `"dest"` and `"named"` go to a `target` in this document (absent when it names
 * no page here), `"uri"` to a `uri` as the file stores it, `"remote"` to another `file`,
 * `"named-action"` runs a named action (`NextPage`, `PrevPage`, `FirstPage`, `LastPage`), and
 * `"unsupported"` names an `action` a reader never runs (`Launch`, `JavaScript`…).
 */
export type PageLink = PageLinkJs;

/**
 * What a properties view shows about a document: its Info entries and dates (ISO 8601), the
 * XMP packet's text, version, pages, file size, linearization and tagging, its `encryption`
 * (with the author's `permissions`, a request rather than a lock), the `fonts` its pages use,
 * the `signatures` present (not verified) and what it `contains` besides its pages.
 */
export type DocumentProperties = PropertiesJs;

/** A place in a document: a 0-based page index and how to show it. */
export type Target = TargetJs;

/**
 * How a {@link Target} shows its page, in page space (points from the displayed page's top-left,
 * y down), by `kind`: `"at"` puts (`x`, `y`) at the viewport's top-left at `zoom`, `"fit"` fits
 * the page, `"fit-width"` its width from `y`, `"fit-height"` its height from `x`, `"fit-rect"` the
 * rectangle. A coordinate the file leaves unset is absent: keep the current one.
 */
export type TargetView = TargetViewJs;
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
