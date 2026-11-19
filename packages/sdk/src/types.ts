// The records of the API: the tsify types `laminapdf-wasm` generates (ADR 0016 §7), re-exported
// under @laminapdf/sdk's names. They are the Rust façade's records, camelCase; only `OpenOptions`
// and `Color` are written here.
import type {
  AddAnnot as AddAnnotJs,
  AddedObject as AddedObjectJs,
  AddLink as AddLinkJs,
  AfRelationshipInit,
  AlignInit,
  Alt as AltJs,
  AnnotColor as AnnotColorJs,
  AnnotCommon as AnnotCommonJs,
  AnnotFont as AnnotFontJs,
  AnnotSpec as AnnotSpecJs,
  AttachFile as AttachFileJs,
  AttachmentIcon as AttachmentIconJs,
  AttachmentInit,
  AttachmentJs,
  BehaviourJs,
  Border as BorderJs,
  BuildOptionsInit,
  Callout as CalloutJs,
  ChildItems as ChildItemsJs,
  ChoiceOptionJs,
  CollectionFieldJs,
  CollectionFileJs,
  CollectionJs,
  ColorProfileJs,
  ColorProfilesJs,
  CommentJs,
  CommentPartJs,
  ConformanceJs,
  ContentEdit as ContentEditJs,
  ContentLayer as ContentLayerJs,
  ContentObjectSpec as ContentObjectSpecJs,
  ContentTextRunJs,
  CosValueJs,
  DestinationUse as DestinationUseJs,
  DestinationView as DestinationViewJs,
  DetailsJs,
  DirectionInit,
  DocumentInfoJs,
  DocumentMetadata as DocumentMetadataJs,
  DynamicStamp as DynamicStampJs,
  EditedJs,
  ExplicitDestination as ExplicitDestinationJs,
  ExtensionPropertyJs,
  ExtractedImageJs,
  FieldBehaviourJs,
  FieldCheckJs,
  FieldEditJs,
  FieldValue as FieldValueJs,
  FileAttachment as FileAttachmentJs,
  FileToAttach as FileToAttachJs,
  FitInit,
  FlattenIntent as FlattenIntentJs,
  FlattenScopeInit,
  FontJs,
  FormFieldJs,
  FormFieldsJs,
  FormScriptJs,
  FormWidgetJs,
  FreeTextIntent as FreeTextIntentJs,
  FreeText as FreeTextJs,
  HolderJs,
  ImageOptionsInit,
  InitialViewJs,
  InkStroke as InkStrokeJs,
  InspectedObjectJs,
  Leader as LeaderJs,
  LimitsInit,
  LineBoxJs,
  LineEnding as LineEndingJs,
  LinkTargetInit,
  MarkupKind as MarkupKindJs,
  MetadataInit,
  MetadataKey as MetadataKeyJs,
  MetadataPatch as MetadataPatchJs,
  NamedDestinationJs,
  NoteIcon as NoteIconJs,
  ObjectEntryJs,
  ObjectRef as ObjectRefJs,
  OpenAt as OpenAtJs,
  Op as OpJs,
  OutlineEntryInit,
  OutlineNodeJs,
  OutlineStyle as OutlineStyleJs,
  OutlineTarget as OutlineTargetJs,
  OutputIntentJs,
  OverflowInit,
  PageAnnotationJs,
  PageBoxesReadJs,
  PageInfoJs,
  PageLabelInit,
  PageLabelRange as PageLabelRangeJs,
  PageLinkJs,
  PageObjectJs,
  PathSpec as PathSpecJs,
  PlannedSaveJs,
  PolyKind as PolyKindJs,
  ProfileInit,
  ProgressJs,
  PropertiesJs,
  RectJs,
  ReferenceEditJs,
  ReferrerJs,
  RenameLinks as RenameLinksJs,
  RenderModeInit,
  RevertedJs,
  RevisionJs,
  RichText as RichTextJs,
  SaveOptionsInit,
  SaveReportJs,
  SetInitialView as SetInitialViewJs,
  ShapeKind as ShapeKindJs,
  SkippedImageJs,
  Stamp as StampJs,
  StampKind as StampKindJs,
  StampName as StampNameJs,
  StampStyle as StampStyleJs,
  StandardFamily as StandardFamilyJs,
  StrokeDash as StrokeDashJs,
  Stroke as StrokeJs,
  StructureKidJs,
  StructureNodeJs,
  TabOrderJs,
  TabStopJs,
  TargetJs,
  TargetViewJs,
  TextAlign as TextAlignJs,
  TextBoxJs,
  TextLayoutJs,
  TextRunJs,
  TextRunRef as TextRunRefJs,
  TextSpan as TextSpanJs,
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
 * `removeXmpProperty`, `replaceXmpPacket`), `removeConformanceClaims`, `setPageLabels`, the page ops (`insertBlankPages`, `deletePages`,
 * `movePages`, `rotatePages`, `reversePages`, `duplicatePages`, `insertPagesFrom`, `replacePages`,
 * `setPageBoxes`, `cropPages`, `resizePages`, `setOrientation`, `rotatePage`), the bookmark edits
 * (`addOutlineItem`, `updateOutlineItem`, `moveOutlineItem`, `deleteOutlineItem`), the named-destination edits
 * (`setNamedDestination`, `renameNamedDestination`, `deleteNamedDestination`), `setInitialView`, the link
 * edits (`addLink`, `updateLink`, `deleteLink`), the annotation edits (`addAnnot`, `updateAnnot`,
 * `deleteAnnot`, `addReply`, `setReviewState`), `flattenAnnots` (draw annotations into their
 * pages and remove them), `editContent` (move, delete, replace, crop or reorder what a page draws),
 * `editText` (change the text a run shows, in its own font), the attachment edits (`attachFile`,
 * `updateAttachment`, `deleteAttachment`), `setFieldValue` (a form field's value), or the
 * deprecated `setInfo` (use
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
 * What an annotation edit draws, in user space: `{ type: "textMarkup", kind, quads }`,
 * `{ type: "note", at, icon, open }`, `{ type: "caret", rect, symbol }`, `{ type: "ink",
 * strokes, width, smoothing }`, `{ type: "shape", kind, rect, interior, border }`,
 * `{ type: "line", from, to, ends, interior, border, leader }` or `{ type: "poly", kind,
 * vertices, ends, interior, border }`.
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
 * A drawn shape: `"square"` (a rectangle) or `"circle"` (the ellipse inside its box).
 *
 * @since 0.5.0
 */
export type ShapeKind = ShapeKindJs;

/**
 * A polygon's kind: `"polygon"` (closed) or `"polyLine"` (open).
 *
 * @since 0.5.0
 */
export type PolyKind = PolyKindJs;

/**
 * A line ending: `"none"`, `"square"`, `"circle"`, `"diamond"`, `"openArrow"`, `"closedArrow"`,
 * `"butt"`, `"rOpenArrow"`, `"rClosedArrow"` (arrowheads pointing back) or `"slash"`.
 *
 * @since 0.5.0
 */
export type LineEnding = LineEndingJs;

/**
 * An outline: its `width` in points (0 for none) and `style`: `{ type: "solid" }`,
 * `{ type: "dashed", pattern }` (dash and gap lengths in points) or `{ type: "cloudy", intensity }`
 * (from 0 to 2, for squares, circles and polygons).
 *
 * @since 0.5.0
 */
export type Border = BorderJs;

/**
 * One stroke of freehand ink: its `points` in user space and, for a pressure-sensitive pen,
 * `widths`, one per point, in points.
 *
 * @since 0.5.0
 */
export type InkStroke = InkStrokeJs;

/**
 * A line's leader lines: their `length` (positive to the left of the line, negative to the
 * right), how far they carry on past it (`extension`) and the gap before they start (`offset`),
 * in points.
 *
 * @since 0.5.0
 */
export type Leader = LeaderJs;

/**
 * A text box, callout or typewriter text: its `rect` in user space, its `text`, `font`
 * (`"helvetica"`, `"times"`, `"courier"`, or the ID of a font's bytes added with `addBlob`),
 * `fallbacks` for characters the font lacks, `size`, `align`, `intent`, `fill` and `border`.
 * The edit's `common.color` is the text's and the border's colour.
 *
 * @since 0.5.0
 */
export type FreeText = FreeTextJs;

/**
 * Text in paragraphs of spans: an array of paragraphs, each an array of
 * `{ text, bold?, italic? }`.
 *
 * @since 0.5.0
 */
export type RichText = RichTextJs;

/**
 * A run of text in one style: `text`, and `bold` or `italic` when set.
 *
 * @since 0.5.0
 */
export type TextSpan = TextSpanJs;

/**
 * A text box's font: `"helvetica"`, `"times"`, `"courier"`, or the 64-digit ID of a font's bytes
 * added with `addBlob`, which needs the text module.
 *
 * @since 0.5.0
 */
export type AnnotFont = AnnotFontJs;

/**
 * A standard 14 family a text box can use: `"helvetica"`, `"times"` or `"courier"`.
 *
 * @since 0.5.0
 */
export type StandardFamily = StandardFamilyJs;

/**
 * How a text box's lines sit in it: `"left"`, `"center"` or `"right"`.
 *
 * @since 0.5.0
 */
export type TextAlign = TextAlignJs;

/**
 * A text box read back by {@link Document.textBox}. `page` is its page, 0-based. `rect` is the
 * box its text is laid out in, `[x0, y0, x1, y1]` in page space. `line` is a callout's line, as
 * `[x, y]` points in page space from the point it points at to the box. `spec` is the box in
 * user space, which a `freeText` patch changes. `color` is the text's colour; left out means
 * black. `simplified` is true when reading dropped styles Lamina PDF doesn't keep; changing the
 * box writes it again without them.
 *
 * @since 0.7.0
 */
export type TextBox = TextBoxJs;

/**
 * What a text box is: `{ type: "textBox" }`, `{ type: "typewriter" }` (no border or fill) or
 * `{ type: "callout", line, end }`.
 *
 * @since 0.5.0
 */
export type FreeTextIntent = FreeTextIntentJs;

/**
 * Which appearance of a page flattening keeps: `"view"`, what a viewer shows on screen, or
 * `"print"`, what it prints. An annotation hidden for it stays as it is.
 *
 * @since 0.6.0
 */
export type FlattenIntent = FlattenIntentJs;

/**
 * Which annotations {@link Document.flattenAnnotations} flattens: `"all"`, every comment and
 * mark on every page; `{ pages: [0, 2] }`, those on these pages (0-based); or
 * `{ annotations: ["a12.0"] }`, these annotations. The first two leave out form fields' widgets,
 * links, pop-ups, replies (a reply goes with its comment) and locked annotations.
 *
 * @since 0.6.0
 */
export type FlattenScope = FlattenScopeInit;

/**
 * A callout's leader line: `line`, two or three points in user space from the point it points
 * at to the box, and its `end`ing.
 *
 * @since 0.5.0
 */
export type Callout = CalloutJs;

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
 * (`linksRemoved`), the copied-in pages that came without their tags (`tagsDropped`), the
 * structure names copied-in pages brought under another name (`structureRenamed`) and the pages
 * whose new crop box hides content that stays in the file (`croppedContentRemains`).
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
 * A file the document carries ({@link Document.attachments}): its `id` for the attachment edits,
 * its `key` in the document's list, `name`, `description`, `mime` type, `size` in bytes and dates
 * (`created`, `modified`, PDF dates) as the file states them, whether it states a checksum
 * (`hasChecksum`), its `relationship` to what it is associated with, whether its bytes are in the
 * document (`embedded`), and what holds it (`holders`).
 *
 * @since 0.6.0
 */
export type DocumentAttachment = AttachmentJs;
/**
 * What holds an attached file, by `kind`: `"embeddedFiles"` (the document's list, which viewers
 * show), `"document"`, `"page"` or `"annotation"` (it is associated with that, `/AF`), or
 * `"fileAttachment"` (a paperclip annotation on a page holds it), with the `page` (0-based) and
 * the `annotation`'s ID where they apply.
 *
 * @since 0.6.0
 */
export type AttachmentHolder = HolderJs;
/**
 * A portfolio ({@link Document.collection}): the columns of its `schema`, the keys its files are
 * `sort`ed by, the `view` it asks for, the `initial` file to show, and its `files`, each with its
 * `values` by column key.
 *
 * @since 0.6.0
 */
export type Collection = CollectionJs;
/**
 * A column of a portfolio's schema: its `key`, heading (`name`), what it shows (`kind`), and
 * whether it is `visible` and `editable`.
 *
 * @since 0.6.0
 */
export type CollectionField = CollectionFieldJs;
/**
 * A file of a portfolio: its `attachment` ID, its `key` in the list and its `values` by column key.
 *
 * @since 0.6.0
 */
export type CollectionFile = CollectionFileJs;
/**
 * The `attachFile` op: the file's `name`, its bytes by blob ID (`file`), its `mime` type, and its
 * `description`, dates, `attachTo` and `relationship`. Make one with {@link Document.attachFileOp}.
 *
 * @since 0.6.0
 */
export type AttachFileOp = AttachFileJs;
/**
 * How an attached file relates to what it is associated with: `"source"`, `"data"`,
 * `"alternative"` (an e-invoice's XML), `"supplement"`, `"encryptedPayload"`, `"formData"`,
 * `"schema"` or `"unspecified"`.
 *
 * @since 0.6.0
 */
export type AttachmentRelationship = NonNullable<AttachFileJs["relationship"]>;
/**
 * What an attached file is associated with, besides the document's list, which every attached
 * file is in: `"embeddedFiles"` (the list only), `"document"`, `{ page: id }` or
 * `{ annotation: id }`.
 *
 * @since 0.6.0
 */
export type AttachTarget = NonNullable<AttachFileJs["attachTo"]>;
/**
 * Options for {@link Document.attachFile}: the file's `description`, when it was made
 * (`created`) and last changed (`modified`), PDF dates from your clock, what it is associated
 * with (`attachTo`, the document's list only by default) and how (`relationship`, unspecified by
 * default). A PDF/A-3 file asks for `attachTo`, `modified` and a MIME type.
 *
 * @since 0.6.0
 */
export interface AttachFileOptions {
  readonly description?: string;
  readonly created?: string;
  readonly modified?: string;
  readonly attachTo?: AttachTarget;
  readonly relationship?: AttachmentRelationship;
}
/**
 * Changes to an attachment, for {@link Document.updateAttachment}: a new `name`, `description`
 * (`null` removes it), `mime` type or `relationship`; a missing key leaves the field alone.
 *
 * @since 0.6.0
 */
export interface AttachmentPatch {
  readonly name?: string;
  readonly description?: string | null;
  readonly mime?: string;
  readonly relationship?: AttachmentRelationship;
}
/**
 * An attached file's bytes ({@link Document.attachmentBytes}), exactly as attached, and how they
 * compare with the MD5 checksum the file states: `"matches"`, `"mismatch"` (a warning: the bytes
 * changed since the checksum was written) or `"absent"`.
 *
 * @since 0.6.0
 */
export interface AttachmentFile {
  readonly bytes: Uint8Array<ArrayBuffer>;
  readonly checksum: "absent" | "matches" | "mismatch";
}
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
 * One object a page draws, in painting order (later ones over earlier ones): its name for an
 * edit (`object`), its `kind` (`"path"`, `"text"`, `"image"`, `"form"`, `"xobject"` or
 * `"shading"`), its box in page space (`bounds`, `[x0, y0, x1, y1]`, before clipping) and, when it
 * can't be moved, deleted or reordered, why (`readOnly`, such as `"clipping-path"`).
 *
 * @since 0.5.0
 */
export type PageObject = PageObjectJs;

/**
 * An object a page draws, named so the name lasts: its 0-based `ordinal` among the page's
 * objects and the `hash` of its bytes (16 hex digits), as {@link PageObject} gives them. An edit
 * whose name no longer matches the page fails with `content-object-stale`.
 *
 * @since 0.5.0
 */
export type ObjectRef = ObjectRefJs;

/**
 * One change to an object a page draws: `{ transform: { object, matrix } }` (a matrix
 * `[a, b, c, d, e, f]` in page space), `{ delete: { object } }`, `{ replaceImage: { object, blob } }`
 * (a blob ID from {@link Document.addBlob}), `{ crop: { object, rect } }` (`[x0, y0, x1, y1]` in page
 * space) or `{ reorder: { object, to } }` (`"front"`, `"back"`, `{ before: ObjectRef }` or
 * `{ after: ObjectRef }`).
 *
 * @since 0.5.0
 */
export type ContentEdit = ContentEditJs;

/**
 * One run of text a page shows, as {@link DocumentContent.textRuns} lists it: one font and size on
 * one baseline (one marked-content sequence on a tagged page) within one text object. It has its
 * name for an `editText` op (`run`), the `text` it shows, its box in page space (`bounds`,
 * `[x0, y0, x1, y1]`), `fontName`, `fontSize`, the `line` it is on (an index into the page's lines
 * in the order the page draws them) and, when it can't be edited, why (`readOnly`, such as `"type3-font"`).
 *
 * @since 0.5.0
 */
export type ContentTextRun = ContentTextRunJs;

/**
 * A run of text named so the name lasts: its text object (`object`, an {@link ObjectRef}) and its
 * glyphs `start` to `end` (0-based, `end` exclusive) among the glyphs the object shows.
 *
 * @since 0.5.0
 */
export type TextRunRef = TextRunRefJs;

/**
 * One annotation on a page, in page space: its ID (which the `updateAnnot` and `deleteAnnot`
 * ops take), subtype, rectangle, text, author, dates, flags, colour, reply links and its own
 * geometry by `kind` (`"quads"`, `"ink"`, `"vertices"`, `"line"`, `"callout"`, `"inner"` or
 * `"none"`), points as flat `[x, y, …]` lists. A square's or circle's `"inner"` is the box it runs
 * along, `[x0, y0, x1, y1]`, inside its rectangle by the stroke's margin (`/RD`).
 *
 * @since 0.3.0
 */
export type PageAnnotation = PageAnnotationJs;

/**
 * What a stamp, a file attachment or a sound comment holds, by `kind`: a stamp's `name`; an
 * attachment's `icon`, `file` (the ID {@link Document.attachmentBytes} takes), `name`, `mime`,
 * `size` and `description`; a sound's `rate`, `channels`, `bits`, `encoding`, whether
 * {@link Document.soundAsWav} can play it, and its length in `seconds`.
 *
 * @since 0.6.0
 */
export type AnnotationDetails = DetailsJs;

/**
 * A stamp: its box `rect` in user space, the page's `rotation` it is drawn upright on, and what
 * it shows, a {@link StampKind}.
 *
 * @since 0.6.0
 */
export type Stamp = StampJs;

/**
 * What a stamp shows: `{ standard: StampName }`, `{ image: blobId }` (a JPEG or PNG added with
 * `addBlob`) or `{ dynamic: DynamicStamp }`.
 *
 * @since 0.6.0
 */
export type StampKind = StampKindJs;

/**
 * A standard stamp's name: ISO 32000-2's fourteen (`"approved"`, `"notApproved"`, `"draft"`, …)
 * and `"reviewed"` and `"void"`.
 *
 * @since 0.6.0
 */
export type StampName = StampNameJs;

/**
 * A stamp's colour: `"green"`, `"red"` or `"blue"`.
 *
 * @since 0.6.0
 */
export type StampStyle = StampStyleJs;

/**
 * A dynamic stamp: one to eight `lines`, already filled in, the first its title, and its `style`.
 *
 * @since 0.6.0
 */
export type DynamicStamp = DynamicStampJs;

/**
 * A file attached at a point: the icon's upper-left corner `at` in user space, its `icon` and
 * the `file`, a {@link FileToAttach}.
 *
 * @since 0.6.0
 */
export type FileAttachment = FileAttachmentJs;

/**
 * The file a file attachment holds: `file`, the ID of its bytes from `addBlob`, its `name`,
 * `mime`, and an optional `description`, `created` and `modified` date.
 *
 * @since 0.6.0
 */
export type FileToAttach = FileToAttachJs;

/**
 * A file attachment's icon: `"paperclip"`, `"pushPin"`, `"graph"` or `"tag"`.
 *
 * @since 0.6.0
 */
export type AttachmentIcon = AttachmentIconJs;

/**
 * A comment and what answers it, each annotation in its own page's space: the comment, its
 * replies in document order (each one's `inReplyTo` names what it answers), the annotations
 * grouped with it, the review states set in the thread (its history), the latest `review`
 * status and `marked` state, the `quoted` text under a highlight or other text markup, and
 * `broken` (`"missing-parent"` or `"cycle"`) when the thread doesn't start at a comment that
 * answers nothing.
 *
 * @since 0.4.1
 */
export type CommentThread = CommentJs;

/**
 * An annotation of a comment thread, with the page it is on (a reply can be on another page
 * than its comment).
 *
 * @since 0.4.1
 */
export type CommentPart = CommentPartJs;

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
 * A page's boxes in user space (`[x0, y0, x1, y1]`), as readers use them: the crop box cut to
 * the media box, and the bleed, trim and art boxes cut to the crop box, a box the page doesn't
 * set taken from the crop box. `own` says which of those three the page sets itself, and
 * `userUnit` is the size of a unit in points. {@link Document.setPageBoxes} takes boxes in the
 * same space.
 *
 * @since 0.5.0
 */
export type PageBoxesRead = PageBoxesReadJs;

/**
 * A form field's value, tagged with its kind: `{ type: "text", value: "…" }` for a text field,
 * `{ type: "button", value: "Yes" }` for a check box or radio group (`null` for off),
 * `{ type: "choice", value: ["…"] }` for a list or combo box, or, in a `setFieldValue` edit,
 * `{ type: "clear" }` to remove the value.
 *
 * @since 0.6.0
 */
export type FieldValue = FieldValueJs;

/**
 * The form's fields, in the order the form lists them, and what its field tree and the pages
 * disagree on: `orphanWidgets`, widgets a page shows that no field reaches, and `widgetless`,
 * fields none of whose widgets a page shows. `needAppearances` says the form asks viewers to
 * draw every widget again; `hasXfa` that it has an XFA part, which Lamina PDF doesn't lay out.
 *
 * @since 0.6.0
 */
export type FormFields = FormFieldsJs;

/**
 * A form field: its ID (`f12.0`, as `setFieldValue` takes it), its fully qualified name, its
 * `kind`, its value and default value, its options, the states its buttons turn on to and its
 * widgets. `readOnly` binds the interface, not the library: edits may still set the value.
 *
 * @since 0.6.0
 */
export type FormField = FormFieldJs;

/**
 * One widget of a form field: its ID, the 0-based index of the page that shows it, its
 * rectangle in that page's space (top-left and bottom-right, y down), and the state it shows.
 *
 * @since 0.6.0
 */
export type FormWidget = FormWidgetJs;

/**
 * One option of a list or combo box, or one export value of a check box or radio group: the value
 * exported and the text shown.
 *
 * @since 0.6.0
 */
export type ChoiceOption = ChoiceOptionJs;

/**
 * The order Tab visits a page's widgets in, and the order the page asks for (`tabs`: `"row"`,
 * `"column"`, `"structure"`, `"annotations"` or `"widgets"`; left out when it asks for none
 * this reader knows, and the widgets come in the order the page lists them).
 *
 * @since 0.6.0
 */
export type TabOrder = TabOrderJs;

/**
 * What a form field's keystroke, format, validate and calculate scripts do, as far as each is one
 * of Acrobat's helper calls (`keystroke`, `format`, `validate`, `calculate`), and the triggers
 * whose scripts aren't (`notHandled`): Lamina PDF fills those fields without them. It runs no
 * script.
 *
 * @since 0.6.0
 */
export type FieldBehaviour = BehaviourJs;

/**
 * One helper call as data: a `number`, `percent`, `date`, `time` or `special` format, a `range`
 * a value must keep, or a `calculate` over other fields.
 *
 * @since 0.6.0
 */
export type FieldHelper = FieldBehaviourJs;

/**
 * A value checked as its field would commit it: what it would store (`value`), why it is
 * `refused` (`format` or `range`), and what the field would show.
 *
 * @since 0.6.0
 */
export type FieldCheck = FieldCheckJs;

/**
 * One script of the document: what it belongs to, what fires it, its size, and whether Lamina
 * PDF handles it (`handled`, `not handled` or `xfa`). Its text is never given.
 *
 * @since 0.6.0
 */
export type FormScript = FormScriptJs;

/**
 * A widget in the tab order: its ID and its field's ID.
 *
 * @since 0.6.0
 */
export type TabStop = TabStopJs;

/**
 * One image a document draws, as a file: `format` `"jpeg"` or `"jpeg2000"` as stored, byte for
 * byte, or `"png"` decoded with its `/Decode`, colour space and mask; `data` holds the file's
 * bytes and `extension` its usual extension. `kind` is `"image"`, `"stencil"` (an image mask:
 * painted black, the rest white), or `"softMask"` / `"mask"`: the mask of a JPEG or JPEG 2000
 * image, which that file can't hold. `page` is the first page (0-based) that draws it, `object`
 * its object number (absent for an inline image), and `colorSpace` its family as `pdfimages
 * -list` names it.
 *
 * @since 0.4.0
 */
export type ExtractedImage = ExtractedImageJs & { data: Uint8Array<ArrayBuffer> };

/**
 * An image that couldn't be extracted: its `page`, its `object` when it has one, and why
 * (`reason`, in English; never document content).
 *
 * @since 0.4.0
 */
export type SkippedImage = SkippedImageJs;

/**
 * What {@link Document.extractImages} gives: the `images`, page by page, each once on the first
 * page that draws it, and the ones `skipped`.
 *
 * @since 0.4.0
 */
export interface ExtractedImages {
  /** The images, page by page in the order each page draws them. */
  readonly images: readonly ExtractedImage[];
  /** The images that couldn't be extracted, with why. */
  readonly skipped: readonly SkippedImage[];
}

/**
 * Which pages {@link Document.extractImages} reads: `pages` (0-based), or every page.
 *
 * @since 0.4.0
 */
export type ImageOptions = ImageOptionsInit;
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

/**
 * The version a file declares and the standards it claims ({@link Document.conformance}): the
 * `%PDF-x.y` header's `headerVersion`, the catalog's `catalogVersion`, and the `pdfa` (part and
 * conformance level), `pdfua` (part) and `pdfx` (`GTS_PDFXVersion`) claims, and `pdfxInfoConflict`, the Info
 * dictionary's PDF/X version when the XMP metadata claims another. Each is claimed, not verified.
 *
 * @since 0.6.0
 */
export type ConformanceClaims = ConformanceJs;

/**
 * One font the document uses ({@link Document.fonts}): its `name` without a subset tag, `kind`
 * (`Type0`, `Type1`, `TrueType`…), whether the file embeds it (`embedded`) or only the glyphs used
 * (`subset`), its `encoding`, its font dictionary's `object` number, whether it has a ToUnicode
 * CMap (`toUnicode`) and how many of its codes that maps (`toUnicodeCoverage`).
 *
 * @since 0.6.0
 */
export type DocumentFont = FontJs;

/**
 * The colour profiles a document carries ({@link Document.colorProfiles}): its `outputIntents`
 * and every ICC profile (`profiles`), once each.
 *
 * @since 0.6.0
 */
export type ColorProfiles = ColorProfilesJs;

/**
 * One output intent: the standard it is for (`subtype`, such as `GTS_PDFA1`), the output
 * condition's identifier, description and registry, its ICC `profile`'s object number, and the
 * `page` whose output intents hold it (absent for the catalog's).
 *
 * @since 0.6.0
 */
export type OutputIntent = OutputIntentJs;

/**
 * One ICC profile the document carries: its stream's `object` number, `components`, `alternate`
 * colour space, whether an output intent uses it, and what its header says (`version`,
 * `deviceClass`, `colorSpace`, `connectionSpace`, `description`).
 *
 * @since 0.6.0
 */
export type ColorProfile = ColorProfileJs;

/**
 * One object of the document ({@link Document.objects}): its `num` and `generation`, `kind`,
 * `/Type` (`typeName`) and `/Subtype`, where it is stored (`location`: an `offset` in the file, an
 * object stream and index, or `created` in this session) and a stream's stored length.
 *
 * @since 0.6.0
 */
export type ObjectEntry = ObjectEntryJs;

/**
 * One object with its value ({@link Document.object}): its `entry`, its `value` (a stream's
 * dictionary for a stream), a stream's `filters` and stored `length`, and the 0-based `revision`
 * that last wrote it.
 *
 * @since 0.6.0
 */
export type InspectedObject = InspectedObjectJs;

/**
 * A PDF value by its `type`: `null`, `boolean`, `integer`, `real`, `string` (its `text`, and
 * whether the file wrote it in hexadecimal), `name`, `array` (`items`), `dictionary` (`entries`,
 * keys without their `/`) or `reference` (`num` and `generation`).
 *
 * @since 0.6.0
 */
export type CosValue = CosValueJs;

/**
 * One place that refers to an object ({@link Document.referencesTo}): the object holding the
 * reference (`from`; absent for the trailer) and where in it (`path`, such as `/Kids[2]`).
 *
 * @since 0.6.0
 */
export type Referrer = ReferrerJs;

/**
 * One revision of the file ({@link Document.revisions}), oldest first: its `index`, the offset of
 * the cross-reference `section` that starts it, its length in `bytes`, the objects it `added`,
 * `changed` and `freed`, and whether a signature covers exactly it (`signed`, not verified).
 *
 * @since 0.6.0
 */
export type Revision = RevisionJs;

/**
 * The first bytes of a stream ({@link Document.streamBytes}) and its whole length in that view.
 *
 * @since 0.6.0
 */
export interface StreamBytes {
  /** At most the bytes asked for, from the start. */
  readonly bytes: Uint8Array<ArrayBuffer>;
  /**
   * The stream's whole length in this view, in bytes; absent for a decoded stream longer than
   * the bytes asked for, which is decoded no further.
   */
  readonly total?: number;
}

/**
 * An object to add to a page as page content: `{ image: { blob, crop? } }` (a JPEG or PNG),
 * `{ shape: { path, fill?, stroke? } }` or `{ form: { blob, page } }` (a page of a PDF,
 * counted from 0). An image's `crop` is the part of its unit square that shows,
 * `[x0, y0, x1, y1]`.
 *
 * @since 0.7.0
 */
export type ContentObjectSpec = ContentObjectSpecJs;

/**
 * A shape's path in page space: `{ line: { from, to } }`, `{ rect: { rect, radius? } }`,
 * `{ ellipse: { rect } }` or `{ polygon: { points, closed? } }`.
 *
 * @since 0.7.0
 */
export type PathSpec = PathSpecJs;

/**
 * A shape's outline: its colour, its width in points and its `dash`, `"solid"` when absent.
 *
 * @since 0.7.0
 */
export type Stroke = StrokeJs;

/**
 * A stroke's pattern: `"solid"`, `"dashed"` or `"dotted"`.
 *
 * @since 0.7.0
 */
export type StrokeDash = StrokeDashJs;

/**
 * What a screen reader gets for an added object in a tagged file: `{ text }`, a figure with that
 * alternate text, or `"decorative"`, an artifact it skips.
 *
 * @since 0.7.0
 */
export type AltText = AltJs;

/**
 * Where an added object goes in the page's stacking order: `"front"`, over the page's content,
 * or `"behind"` it.
 *
 * @since 0.7.0
 */
export type ContentLayer = ContentLayerJs;

/**
 * An object Lamina PDF added to a page, as {@link DocumentContent.added} lists it.
 *
 * @since 0.7.0
 */
export type AddedObject = AddedObjectJs;

/**
 * What {@link DocumentContent.add} puts on a page: its ID (1 to 64 ASCII letters, digits, `-`,
 * `_` or `.`, unique on the page), the object, the matrix that places it, and a PDF date for the
 * form's `/LastModified`.
 *
 * @since 0.7.0
 */
export interface NewContentObject {
  readonly id: string;
  readonly object: ContentObjectSpec;
  readonly matrix: readonly [number, number, number, number, number, number];
  /** From 0 to 1; opaque when absent. */
  readonly opacity?: number;
  /** In front of the page's content when absent. */
  readonly layer?: ContentLayer;
  readonly alt?: AltText;
  /** A PDF date, such as `"D:20261118120000Z"`. */
  readonly modified: string;
}

/**
 * What {@link DocumentContent.update} changes on an added object; a PDF date for the form's new
 * `/LastModified`, and any of the rest. `order` is the object's place among those on its layer:
 * 0 the backmost, -1 the frontmost.
 *
 * @since 0.7.0
 */
export interface ContentChanges {
  readonly modified: string;
  readonly matrix?: readonly [number, number, number, number, number, number];
  readonly opacity?: number;
  readonly crop?: readonly [number, number, number, number] | null;
  readonly fill?: AnnotColor | null;
  readonly stroke?: Stroke | null;
  readonly blob?: string;
  readonly alt?: AltText;
  readonly order?: number;
}
