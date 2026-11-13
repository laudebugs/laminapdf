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
  DocumentMetadata as DocumentMetadataJs,
  EditedJs,
  FitInit,
  LimitsInit,
  LineBoxJs,
  LinkTargetInit,
  MetadataInit,
  MetadataKey as MetadataKeyJs,
  MetadataPatch as MetadataPatchJs,
  Op as OpJs,
  OutlineEntryInit,
  OutlineNodeJs,
  OverflowInit,
  PageInfoJs,
  PageLabelInit,
  PageLinkJs,
  PlannedSaveJs,
  ProfileInit,
  ProgressJs,
  PropertiesJs,
  RectJs,
  ReferenceEditJs,
  RenderModeInit,
  RevertedJs,
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
  ViolationJs,
} from "@laminapdf/wasm/text";

/**
 * An edit (`{ op, args }`): `setMetadata`, `removeMetadata`, the page ops (`insertBlankPages`,
 * `deletePages`, `movePages`, `rotatePages`, `reversePages`, `rotatePage`), or the deprecated
 * `setInfo` (use `setMetadata`, which keeps the Info dictionary and the XMP metadata in step).
 *
 * @since 0.2.0
 */
export type Op = OpJs;
/**
 * Changes to the document's metadata, written to its Info dictionary and its XMP metadata
 * together: a value sets the field, `null` removes it, a missing key leaves it alone. Dates are
 * PDF dates (`D:20261110093000Z`).
 *
 * @since 0.2.0
 */
export type MetadataPatch = MetadataPatchJs;
/**
 * A metadata field: `title`, `authors`, `subject`, `keywords`, `creator`, `producer`, `created`, `modified`.
 *
 * @since 0.2.0
 */
export type MetadataKey = MetadataKeyJs;
/**
 * A document's metadata, read from its Info dictionary and its XMP metadata together. When they
 * disagree, the values come from XMP in a PDF 2.0 or PDF/A file and otherwise from the side
 * modified last; `disagreements` names the fields that differ.
 *
 * @since 0.2.0
 */
export type DocumentMetadata = DocumentMetadataJs;
/**
 * How a save would go, before it happens: what restricts it (`constraint`, the signatures and
 * the edits they forbid), the mode it resolves to or the code it would be refused with,
 * what it does with the save-time metadata, and the PDF/A and PDF/UA claims the edits break.
 *
 * @since 0.2.0
 */
export type PlannedSave = PlannedSaveJs;
/**
 * A change a signature's restriction forbids.
 *
 * @since 0.2.0
 */
export type Violation = ViolationJs;
/**
 * An op log: `{ v: 1, ops }`.
 *
 * @since 0.2.0
 */
export interface OpLog {
  readonly v: 1;
  readonly ops: readonly Op[];
}
/**
 * Resource limits for reading untrusted files; each one left out keeps its default.
 *
 * @since 0.2.0
 */
export type Limits = LimitsInit;
/**
 * Options for {@link Document.open}.
 *
 * @since 0.2.0
 */
export interface OpenOptions {
  /** The user or owner password of an encrypted document (tried as given, then in its NFKC form). */
  readonly password?: string;
  /** Refuse files that need repair (`strict`). */
  readonly strict?: boolean;
  /** Resource limits. */
  readonly limits?: Limits;
}
/**
 * Options for saving.
 *
 * @since 0.2.0
 */
export type SaveOptions = SaveOptionsInit;
/**
 * What a save wrote.
 *
 * @since 0.2.0
 */
export type SaveReport = SaveReportJs;
/**
 * A page's facts: index, ID, displayed size in points, rotation, label.
 *
 * @since 0.2.0
 */
export type PageInfo = PageInfoJs;
/**
 * The Info dictionary's text entries (document text).
 *
 * @since 0.2.0
 */
export type DocumentInfo = DocumentInfoJs;
/**
 * An element of a tagged document's structure tree: its standard type (`H1`, `P`, `LI`,
 * `Figure` …), alternate text, language, actual text, and kids in reading order (document text).
 *
 * @since 0.2.0
 */
export type StructureNode = StructureNodeJs;
/**
 * A kid of a {@link StructureNode}: an element, or marked content `mcid` on page `page`.
 *
 * @since 0.2.0
 */
export type StructureKid = StructureKidJs;
/**
 * What an edit did: the pages it touched (`touchedPages`, IDs), whether it changed anything,
 * the page IDs in their new order when it changed the page list (`pageOrder`), and what a page
 * deletion did to the references to the deleted pages (`references`).
 *
 * @since 0.2.0
 */
export type Edited = EditedJs;
/**
 * A reference to a deleted page (an outline item, a link, a named destination, the open action,
 * a structure element or kid, or a form field's widget) and what the deletion did to it:
 * `retargeted` to the page `to`, or `removed`.
 *
 * @since 0.2.0
 */
export type ReferenceEdit = ReferenceEditJs;
/**
 * How an undo or redo went.
 *
 * @since 0.2.0
 */
export type UndoOutcome = UndoOutcomeJs;
/**
 * What an undo or redo did: how it went (`outcome`) and the pages to draw again (`touchedPages`, IDs).
 *
 * @since 0.2.0
 */
export type Reverted = RevertedJs;
/**
 * A save job's progress.
 *
 * @since 0.2.0
 */
export type Progress = ProgressJs;
/**
 * Options for a new document: version, fixed file ID (32 hex digits), producer, creation date.
 *
 * @since 0.2.0
 */
export type BuildOptions = BuildOptionsInit;
/**
 * A conformance profile: `PDF/UA-1` (accessible), `PDF/A-2b`, `PDF/A-2u` or `PDF/A-3b` (archival).
 *
 * @since 0.2.0
 */
export type Profile = ProfileInit;
/**
 * How an attached file relates to the document (`/AFRelationship`).
 *
 * @since 0.2.0
 */
export type AfRelationship = AfRelationshipInit;
/**
 * An attached file's relationship, description and modification date.
 *
 * @since 0.2.0
 */
export type AttachmentOptions = AttachmentInit;
/**
 * Where a link or outline item goes.
 *
 * @since 0.2.0
 */
export type LinkTarget = LinkTargetInit;
/**
 * How a destination shows its page.
 *
 * @since 0.2.0
 */
export type Fit = FitInit;
/**
 * An outline (bookmarks) item with its children.
 *
 * @since 0.2.0
 */
export type OutlineEntry = OutlineEntryInit;

/**
 * One item of a document's outline (its bookmarks), with where it goes and its children.
 *
 * @since 0.2.0
 */
export type OutlineNode = OutlineNodeJs;

/**
 * One link on a page, in page space: its `rect` (`[x0, y0, x1, y1]`), the `quads` that follow it
 * when the file gives them (eight numbers per area), its `contents` (a description), and what it
 * does by `kind`: `"dest"` and `"named"` go to a `target` in this document (absent when it names
 * no page here), `"uri"` to a `uri` as the file stores it, `"remote"` to another `file`,
 * `"named-action"` runs a named action (`NextPage`, `PrevPage`, `FirstPage`, `LastPage`), and
 * `"unsupported"` names an `action` a reader never runs (`Launch`, `JavaScript`…).
 *
 * @since 0.2.0
 */
export type PageLink = PageLinkJs;

/**
 * What a properties view shows about a document: its Info entries and dates (ISO 8601), the
 * XMP packet's text, version, pages, file size, linearization and tagging, its `encryption`
 * (with the author's `permissions`, a request rather than a lock), the `fonts` its pages use,
 * the `signatures` present (not verified) and what it `contains` besides its pages.
 *
 * @since 0.2.0
 */
export type DocumentProperties = PropertiesJs;

/**
 * A place in a document: a 0-based page index and how to show it.
 *
 * @since 0.2.0
 */
export type Target = TargetJs;

/**
 * How a {@link Target} shows its page, in page space (points from the displayed page's top-left,
 * y down), by `kind`: `"at"` puts (`x`, `y`) at the viewport's top-left at `zoom`, `"fit"` fits
 * the page, `"fit-width"` its width from `y`, `"fit-height"` its height from `x`, `"fit-rect"` the
 * rectangle. A coordinate the file leaves unset is absent: keep the current one.
 *
 * @since 0.2.0
 */
export type TargetView = TargetViewJs;
/**
 * A page-label range, by its first page.
 *
 * @since 0.2.0
 */
export type PageLabel = PageLabelInit;
/**
 * The document's metadata (Info and XMP).
 *
 * @since 0.2.0
 */
export type Metadata = MetadataInit;
/**
 * How viewers present the document.
 *
 * @since 0.2.0
 */
export type ViewerPreferences = ViewerPreferencesInit;
/**
 * A rectangle returned by the builder (page space).
 *
 * @since 0.2.0
 */
export type BoxRect = RectJs;
/**
 * What a text run measures.
 *
 * @since 0.2.0
 */
export type TextRunInfo = TextRunJs;
/**
 * How a text box laid its text out.
 *
 * @since 0.2.0
 */
export type TextLayout = TextLayoutJs;
/**
 * A line of a text box.
 *
 * @since 0.2.0
 */
export type LineBox = LineBoxJs;
/**
 * Text alignment in a text box.
 *
 * @since 0.2.0
 */
export type Align = AlignInit;
/**
 * Paragraph direction.
 *
 * @since 0.2.0
 */
export type Direction = DirectionInit;
/**
 * What happens to text that doesn't fit a text box.
 *
 * @since 0.2.0
 */
export type Overflow = OverflowInit;
/**
 * Text rendering mode.
 *
 * @since 0.2.0
 */
export type RenderMode = RenderModeInit;

/**
 * A colour: `[gray]`, `[r, g, b]` or `[c, m, y, k]` with components from 0 to 1, or `"#rrggbb"`
 * / `"#rgb"` (DeviceRGB).
 *
 * @since 0.2.0
 */
export type Color =
  | string
  | readonly [number]
  | readonly [number, number, number]
  | readonly [number, number, number, number];
/**
 * A point in page space (points from the top-left of the displayed page, y down).
 *
 * @since 0.2.0
 */
export type Point = readonly [x: number, y: number];
/**
 * A rectangle in page space: left, top, width, height.
 *
 * @since 0.2.0
 */
export type Rect = readonly [x: number, y: number, width: number, height: number];
