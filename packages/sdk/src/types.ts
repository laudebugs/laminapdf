// The records of the API: the tsify types `laminapdf-wasm` generates (ADR 0016 §7), re-exported
// under @laminapdf/sdk's names. They are the Rust façade's records, camelCase; only `OpenOptions`
// and `Color` are written here.
import type {
  AddAnnot as AddAnnotJs,
  AddLink as AddLinkJs,
  AfRelationshipInit,
  AlignInit,
  AnnotColor as AnnotColorJs,
  AnnotCommon as AnnotCommonJs,
  AnnotSpec as AnnotSpecJs,
  AttachmentInit,
  BuildOptionsInit,
  ChildItems as ChildItemsJs,
  DestinationUse as DestinationUseJs,
  DestinationView as DestinationViewJs,
  DirectionInit,
  DocumentInfoJs,
  DocumentMetadata as DocumentMetadataJs,
  EditedJs,
  ExplicitDestination as ExplicitDestinationJs,
  ExtensionPropertyJs,
  FieldEditJs,
  FitInit,
  InitialViewJs,
  LimitsInit,
  LineBoxJs,
  LinkTargetInit,
  MarkupKind as MarkupKindJs,
  MetadataInit,
  MetadataKey as MetadataKeyJs,
  MetadataPatch as MetadataPatchJs,
  NamedDestinationJs,
  NoteIcon as NoteIconJs,
  OpenAt as OpenAtJs,
  Op as OpJs,
  OutlineEntryInit,
  OutlineNodeJs,
  OutlineStyle as OutlineStyleJs,
  OutlineTarget as OutlineTargetJs,
  OverflowInit,
  PageAnnotationJs,
  PageInfoJs,
  PageLabelInit,
  PageLabelRange as PageLabelRangeJs,
  PageLinkJs,
  PlannedSaveJs,
  ProfileInit,
  ProgressJs,
  PropertiesJs,
  RectJs,
  ReferenceEditJs,
  RenameLinks as RenameLinksJs,
  RenderModeInit,
  RevertedJs,
  SaveOptionsInit,
  SaveReportJs,
  SetInitialView as SetInitialViewJs,
  StructureKidJs,
  StructureNodeJs,
  TargetJs,
  TargetViewJs,
  TextLayoutJs,
  TextRunJs,
  ThreadJs,
  UndoOutcomeJs,
  ViewerPreferencesInit,
  ViolationJs,
  XmpLangText as XmpLangTextJs,
  XmpTextJs,
  XmpValue as XmpValueJs,
} from "@laminapdf/wasm/text";

/**
 * An edit (`{ op, args }`): `setMetadata`, `removeMetadata`, the XMP edits (`setXmpProperty`,
 * `removeXmpProperty`, `replaceXmpPacket`), `setPageLabels`, the page ops (`insertBlankPages`, `deletePages`,
 * `movePages`, `rotatePages`, `reversePages`, `duplicatePages`, `insertPagesFrom`, `replacePages`,
 * `rotatePage`), the bookmark edits
 * (`addOutlineItem`, `updateOutlineItem`, `moveOutlineItem`, `deleteOutlineItem`), the named-destination edits
 * (`setNamedDestination`, `renameNamedDestination`, `deleteNamedDestination`), `setInitialView`, the link
 * edits (`addLink`, `updateLink`, `deleteLink`), the annotation edits (`addAnnot`, `updateAnnot`,
 * `deleteAnnot`), or the deprecated `setInfo` (use
 * `setMetadata`, which keeps the Info dictionary and the XMP metadata in step).
 *
 * @since 0.2.0
 */
export type Op = OpJs;

/**
 * A page of the document and how to show it, in the page's user space (PDF points from the
 * bottom-left, y up): what a bookmark or a named destination goes to. Make one from a place in
 * the viewer with {@link Document.destinationAt}.
 *
 * @since 0.3.0
 */
export type ExplicitDestination = ExplicitDestinationJs;

/**
 * How an {@link ExplicitDestination} shows its page, in user space, by `kind`: `"xyz"` puts
 * (`left`, `top`) at the top-left at `zoom`, `"fit"` fits the page, `"fitH"` its width from
 * `top`, `"fitV"` its height from `left`, `"fitR"` a rectangle, and `"fitB"`, `"fitBH"`,
 * `"fitBV"` the same for the page's drawn area. `null` keeps the viewer's current value.
 *
 * @since 0.3.0
 */
export type DestinationView = DestinationViewJs;

/**
 * Where a bookmark or a link goes: `{ destination }` (a page), `{ named }` (a named
 * destination) or `{ uri }` (an `http`, `https` or `mailto` address).
 *
 * @since 0.3.0
 */
export type OutlineTarget = OutlineTargetJs;

/**
 * An `addLink` edit's arguments: the `page`, its `rect` and `quads` in user space, where it goes
 * (`target`), its description (`contents`) and, in a tagged document, the marked content it
 * covers (`marked`). {@link Document.linkOnArea} and {@link Document.linkOnText} make one from
 * page space.
 *
 * @since 0.3.0
 */
export type AddLink = AddLinkJs;

/**
 * An `addAnnot` edit's arguments: the `page`, the mark (`spec`: a text markup, a note or a caret,
 * in user space), its `common` entries, its `nm` and, in a tagged document, the marked content
 * under it (`marked`). {@link Document.markText}, {@link Document.noteAt} and
 * {@link Document.caretAt} make one from page space.
 *
 * @since 0.3.0
 */
export type AddAnnot = AddAnnotJs;

/**
 * What an annotation edit draws: `{ type: "textMarkup", kind, quads }`, `{ type: "note", at, icon,
 * open }` or `{ type: "caret", rect, symbol }`, in user space.
 *
 * @since 0.3.0
 */
export type AnnotSpec = AnnotSpecJs;

/**
 * The entries every new annotation has: its description or comment (`contents`), `author`,
 * `subject`, dates (`created`, `modified`), `color`, `opacity`, `flags`, and `groupWith`, an
 * annotation on the same page to group it with.
 *
 * @since 0.3.0
 */
export type AnnotCommon = AnnotCommonJs;

/**
 * An annotation's colour as PDF stores it: `[gray]`, `[r, g, b]` or `[c, m, y, k]`, each from 0
 * to 1.
 *
 * @since 0.3.0
 */
export type AnnotColor = AnnotColorJs;

/**
 * A text mark: `"highlight"`, `"underline"`, `"strikeOut"` or `"squiggly"`.
 *
 * @since 0.3.0
 */
export type MarkupKind = MarkupKindJs;

/**
 * A note's icon: `"note"`, `"comment"`, `"key"`, `"help"`, `"paragraph"`, `"newParagraph"` or
 * `"insert"`.
 *
 * @since 0.3.0
 */
export type NoteIcon = NoteIconJs;

/**
 * A bookmark's look: its `color` (`"#rrggbb"`, or `null` for the default black), `bold` and
 * `italic`.
 *
 * @since 0.3.0
 */
export type OutlineStyle = OutlineStyleJs;

/**
 * What deleting a bookmark does with its children: `"delete"` them with it, or `"promote"` them
 * into its place.
 *
 * @since 0.3.0
 */
export type ChildItems = ChildItemsJs;

/**
 * Whether renaming a named destination renames the bookmarks and links that use it too
 * (`"follow"`, the default) or leaves them naming the old name (`"keep"`).
 *
 * @since 0.3.0
 */
export type RenameLinks = RenameLinksJs;

/**
 * Something that still names a destination an edit deleted: a bookmark (`"outlineItem"`), a
 * link on a page (`"link"`) or what the file shows when it opens (`"openAction"`).
 *
 * @since 0.3.0
 */
export type DestinationUse = DestinationUseJs;

/**
 * A named destination: a name links and bookmarks jump to, and where it goes (absent when it
 * names no page of this document).
 *
 * @since 0.3.0
 */
export type NamedDestination = NamedDestinationJs;
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
 * A value an XMP property can be set to, tagged with its type: `{ type: "text", value: "…" }`,
 * `uri`, `integer`, `real`, `boolean`, `date` (a PDF date, `D:20261110093000Z`), `langAlt` (a
 * list of {@link XmpLangText}), or `seq` and `bag` (lists of simple values of one type).
 *
 * @since 0.3.0
 */
export type XmpValue = XmpValueJs;
/**
 * One language's text in an XMP language alternative: `{ lang: "x-default", text: "…" }`.
 *
 * @since 0.3.0
 */
export type XmpLangText = XmpLangTextJs;
/**
 * The document's XMP metadata as text: `status` (`absent`, `parsed` or `malformed`), the
 * packet's `xml` with the edits made so far, and the PDF/A level it claims (`pdfa`, e.g. `"2B"`).
 *
 * @since 0.3.0
 */
export type XmpText = XmpTextJs;
/**
 * An XMP property an edit declared in a PDF/A file's extension schemas, so the file keeps its
 * PDF/A claim: its `namespace`, `prefix`, `name` and `valueType` (`Text`, `Seq Integer`, …).
 *
 * @since 0.3.0
 */
export type ExtensionProperty = ExtensionPropertyJs;
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
 * the page IDs in their new order when it changed the page list (`pageOrder`), what a page
 * deletion did to the references to the deleted pages (`references`), and what a page
 * duplication or a page copied in did to form fields (`fields`), which copies a duplication
 * couldn't tag whole (`tagsIncomplete`), the page of each link a copied-in page lost
 * (`linksRemoved`), the copied-in pages that came without their tags (`tagsDropped`) and the
 * structure names copied-in pages brought under another name (`structureRenamed`).
 *
 * @since 0.2.0
 */
export type Edited = EditedJs;
/**
 * A reference to a deleted page (an outline item, a link, a named destination, the open action,
 * a structure element or kid, or a form field's widget) and what the deletion did to it:
 * `retargeted` to the page `to`, or `removed`.
 *
 * @since 0.3.0
 */
export type ReferenceEdit = ReferenceEditJs;
/**
 * A form field whose widget a duplicated or copied-in page showed, and what the copy did with it:
 * `renamed` (the copy's widget belongs to the new field `to`, fully named `name`), `linked` (it
 * joined the document's field `to` of the same name and shares its value) or `notCopied` (a
 * signature's widget).
 *
 * @since 0.3.0
 */
export type FieldEdit = FieldEditJs;
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
 * One item of a document's outline (its bookmarks): its `id` (what the bookmark edits take), its
 * title, where it goes (a page `target`, a `destinationName`, a `uri`, or an `otherAction` such
 * as `JavaScript`, which Lamina PDF never runs), its `color`, `bold` and `italic` style, whether it
 * shows its children (`open`) and the children.
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
 * One annotation on a page, in page space: its ID (which the `updateAnnot` and `deleteAnnot`
 * ops take), subtype, rectangle, text, author, dates, flags, colour, reply links and its own
 * geometry by `kind` (`"quads"`, `"ink"`, `"vertices"`, `"line"`, `"callout"` or `"none"`),
 * points as flat `[x, y, …]` lists.
 *
 * @since 0.3.0
 */
export type PageAnnotation = PageAnnotationJs;

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
 * How a document asks to be shown when it opens: where it `open`s by `kind` (`"default"`: its
 * first page at the reader's zoom; `"page"`: a {@link Target}; `"action"`: an `action` a reader
 * runs, by its type, which Lamina PDF never runs), the panel to show (`pageMode`), how to lay the
 * pages out (`pageLayout`), absent when the file leaves them to the reader, and the window's
 * `preferences`.
 *
 * @since 0.3.0
 */
export type InitialView = InitialViewJs;

/**
 * Changes to how a document opens, for {@link Document.setInitialView}: a value sets the field,
 * `null` removes it, a missing key leaves it alone. `open` names a page by its ID
 * ({@link PageInfo.id}) and a `zoom`: `"default"`, `"fitPage"`, `"fitWidth"` or a percentage.
 *
 * @since 0.3.0
 */
export type InitialViewPatch = SetInitialViewJs;

/**
 * The page and zoom a document opens at, in an {@link InitialViewPatch}.
 *
 * @since 0.3.0
 */
export type OpenAt = OpenAtJs;

/**
 * An article thread: its `title` (document text) and its `parts` in reading order, each a `page`
 * (0-based) and a `rect` (`[x0, y0, x1, y1]` in page space), absent when the part names a page
 * the document lacks.
 *
 * @since 0.3.0
 */
export type ArticleThread = ThreadJs;
/**
 * A page-label range, by its first page.
 *
 * @since 0.2.0
 */
export type PageLabel = PageLabelInit;
/**
 * A page-label range for {@link Document.setPageLabels}: the labels from `page` (a page ID) up
 * to the next range's page, in a `style` (`"decimal"`, `"upperRoman"`, `"lowerRoman"`,
 * `"upperLetters"`, `"lowerLetters"`; without one, the `prefix` alone), after a `prefix`,
 * numbered from `first` (1 when left out).
 *
 * @since 0.3.0
 */
export type PageLabelRange = PageLabelRangeJs;
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
