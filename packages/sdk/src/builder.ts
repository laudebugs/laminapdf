// Builder, Page and Surface: new documents, page by page, in page space (points from the top-left
// of the displayed page, y down; ADR 0027). Over WasmBuilder / WasmPageBuilder (ADR 0016 §7):
// paths cross as flat Float64Arrays, colours as component arrays.
import type { WasmBuilder as BaseBuilder, WasmDocument } from "@laminapdf/wasm/base";
import type { WasmBuilder as TextBuilder, WasmPageBuilder as TextPageBuilder, WasmFont } from "@laminapdf/wasm/text";
import { defined } from "./defined.js";
import { Document } from "./document.js";
import { didYouMean, guard, packageError } from "./errors.js";
import type { Font, FontInfo, Image } from "./font.js";
import {
  ARTIFACT_KINDS,
  type ArtifactKind,
  type BlendMode,
  type FillRule,
  type LineCap,
  type LineJoin,
  TABLE_SCOPES,
  TAG_NAMES,
  type TableScope,
  type TagName,
} from "./generated/builder-enums.js";
import { type BaseGlue, readingModule, type TextGlue, textModule } from "./init.js";
import type { Transform } from "./matrix.js";
import { type PathBuilder, PathEncoder } from "./path.js";
import type {
  Align,
  AttachmentOptions,
  BuildOptions,
  Color,
  Direction,
  Fit,
  LinkTarget,
  Metadata,
  OutlineEntry,
  Overflow,
  PageLabel,
  Point,
  Rect,
  RenderMode,
  TextLayout,
  TextRunInfo,
  ViewerPreferences,
} from "./types.js";

/** Named page sizes (ISO 216 sizes from millimetres, as the Rust `PageSize` computes them). */
export type PageSizeName = "A0" | "A1" | "A2" | "A3" | "A4" | "A5" | "A6" | "Letter" | "Legal" | "Tabloid";

const mm = (w: number, h: number): readonly [number, number] => [(w * 72) / 25.4, (h * 72) / 25.4];

/** Each named size in points, width × height. */
export const PAGE_SIZES: Readonly<Record<PageSizeName, readonly [width: number, height: number]>> = {
  A0: mm(841, 1189),
  A1: mm(594, 841),
  A2: mm(420, 594),
  A3: mm(297, 420),
  A4: mm(210, 297),
  A5: mm(148, 210),
  A6: mm(105, 148),
  Letter: [612, 792],
  Legal: [612, 1008],
  Tabloid: [792, 1224],
};

/** Options for a new {@link Builder}. */
export interface BuilderOptions extends Omit<BuildOptions, "id"> {
  /** `/ID[0]`: 16 bytes, as 32 hex digits or a `Uint8Array` (default: derived from the content). */
  readonly id?: string | Uint8Array;
  /**
   * Build in the text module (fonts and text). Default: when it is loaded. `true` without the
   * text module loaded throws `text-module-not-loaded`.
   */
  readonly text?: boolean;
}

/** How strokes are drawn. */
export interface StrokeStyle {
  /** Line width in points (default 1). */
  readonly width?: number;
  /** Line ends (default `butt`). */
  readonly cap?: LineCap;
  /** Corners (default `miter`). */
  readonly join?: LineJoin;
  /** Miter limit (default 10). */
  readonly miterLimit?: number;
  /** Dash pattern in points (default: solid). */
  readonly dash?: readonly number[];
  /** Where the dash pattern starts (default 0). */
  readonly dashPhase?: number;
}

export type {
  ArtifactKind,
  BlendMode,
  FillRule,
  LineCap,
  LineJoin,
  TableScope,
  TagName,
} from "./generated/builder-enums.js";
export { TAG_NAMES } from "./generated/builder-enums.js";

/** `name` when `names` has it, else `invalid-argument` with the closest names. */
function checked(names: readonly string[], name: string, argument: string, detail: string): string {
  if (names.includes(name)) return name;
  throw packageError("invalid-argument", detail, { didYouMean: didYouMean(name, names), data: { argument, detail } });
}

/** A structure element with attributes; `type` may be a custom name when `role` is given. */
export interface TagSpec {
  /** A standard type, or a custom name written with a `RoleMap` entry to `role`. */
  readonly type: TagName | (string & {});
  /** The standard type a custom `type` stands for. */
  readonly role?: TagName;
  /** `/Alt`: alternate text (a `Figure` needs it). */
  readonly alt?: string;
  /** `/ActualText`: the exact text the content stands for. */
  readonly actualText?: string;
  /** `/Lang`: the element's language (BCP 47). */
  readonly lang?: string;
  /** `/E`: the expansion of an abbreviation. */
  readonly expansion?: string;
  /** `/Scope` of a `TH`. */
  readonly scope?: TableScope;
}

/** A structure element: a type, or a type with attributes. */
export type Tag = TagName | TagSpec;

/** Options of {@link Surface.text}. */
export interface TextOptions {
  /** The font (needs the text module). */
  readonly font: Font;
  /** Size in points. */
  readonly size: number;
  /** Where the first line starts: `x`, and `y` is the baseline (ADR 0027 §5). */
  readonly at: Point;
  /** Fill colour. */
  readonly fill?: Color;
  /** Rendering mode (`invisible` for an OCR layer). */
  readonly renderMode?: RenderMode;
  /** A link over the run. */
  readonly link?: { readonly target: LinkTarget; readonly contents?: string };
  /** OpenType features (`liga=0`, `smcp`). */
  readonly features?: readonly string[];
  /** Language (BCP 47). */
  readonly language?: string;
  /** Paragraph direction (default: from the first strong character). */
  readonly direction?: Direction;
  /** The structure element the run is (as drawing it inside {@link Surface.tag}). */
  readonly tag?: Tag;
}

/** Options of {@link Surface.textBox}. */
export interface TextBoxOptions {
  /** The font (needs the text module). */
  readonly font: Font;
  /** Size in points. */
  readonly size: number;
  /** The box the lines are broken into. */
  readonly box: Rect;
  /** Line alignment (default `start`). */
  readonly align?: Align;
  /** Line height as a factor of the size (default 1.2). */
  readonly lineHeight?: number;
  /** What happens to text that doesn't fit (default `clip`; `error` throws `text-overflow`). */
  readonly overflow?: Overflow;
  /** Paragraph direction. */
  readonly direction?: Direction;
  /** Language (BCP 47). */
  readonly language?: string;
  /** OpenType features. */
  readonly features?: readonly string[];
  /** The structure element the box is (as drawing it inside {@link Surface.tag}). */
  readonly tag?: Tag;
}

/** The size of a named page; `invalid-argument` (with suggestions) for an unknown name. */
function namedSize(name: string): readonly [number, number] {
  const size = (PAGE_SIZES as Readonly<Record<string, readonly [number, number]>>)[name];
  if (size !== undefined && Object.hasOwn(PAGE_SIZES, name)) return size;
  throw packageError("invalid-argument", "not a page size name", {
    didYouMean: didYouMean(name, Object.keys(PAGE_SIZES)),
    data: { argument: "size", detail: "a page size is a name such as A4 or Letter, or [width, height] in points" },
  });
}

/** Whether `a` and `b` hold the same bytes (identity first: no copy, no hash). */
function sameBytes(a: Uint8Array, b: Uint8Array): boolean {
  if (a === b) return true;
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return false;
  return true;
}

const HEX = "0123456789abcdef";

function hexId(id: string | Uint8Array): string {
  if (typeof id === "string") return id;
  let out = "";
  for (const byte of id) out += (HEX[byte >> 4] ?? "") + (HEX[byte & 15] ?? "");
  return out;
}

/** The record without the keys the TS layer handles, or `undefined` when nothing is left. */
function rest<T extends object>(record: T, keys: readonly string[]): object | undefined {
  const out = Object.fromEntries(Object.entries(defined(record)).filter(([k]) => !keys.includes(k)));
  return Object.keys(out).length > 0 ? out : undefined;
}

/** The raw page of a {@link Page}: freed when the page finishes, `builder-finished` afterwards. */
class LivePage {
  #raw: TextPageBuilder | undefined;
  readonly #onFree: (page: LivePage) => void;

  constructor(raw: TextPageBuilder, onFree: (page: LivePage) => void) {
    this.#raw = raw;
    this.#onFree = onFree;
  }

  get(): TextPageBuilder {
    if (this.#raw === undefined) throw packageError("builder-finished", "the page has already finished");
    return this.#raw;
  }

  free(): void {
    this.#raw?.free();
    this.#raw = undefined;
    this.#onFree(this);
  }
}

/** A path waiting to be painted: fill it, stroke it, both, or clip to it. */
export class PendingPath {
  readonly #surface: Surface;
  readonly #page: LivePage;
  readonly #path: Float64Array;

  /** @internal */
  constructor(surface: Surface, page: LivePage, path: Float64Array) {
    this.#surface = surface;
    this.#page = page;
    this.#path = path;
  }

  /** Fills the path with the fill colour. @returns the surface. */
  fill(rule: FillRule = "nonZero"): Surface {
    guard(() => this.#page.get().fillPath(this.#path, rule));
    return this.#surface;
  }

  /** Strokes the path with the stroke colour and `style`. @returns the surface. */
  stroke(style: StrokeStyle = {}): Surface {
    this.#surface.strokeStyle(style);
    guard(() => this.#page.get().strokePath(this.#path));
    return this.#surface;
  }

  /** Fills, then strokes the path. @returns the surface. */
  fillStroke(rule: FillRule = "nonZero", style: StrokeStyle = {}): Surface {
    this.#surface.strokeStyle(style);
    guard(() => this.#page.get().fillStrokePath(this.#path, rule));
    return this.#surface;
  }

  /** Clips what is drawn next to the path, until the next `restore`. @returns the surface. */
  clip(rule: FillRule = "nonZero"): Surface {
    guard(() => this.#page.get().clipPath(this.#path, rule));
    return this.#surface;
  }
}

/**
 * What a page is drawn with. Methods that set state return the surface, so calls chain; invalid
 * geometry is reported by {@link Page.finish} (`invalid-geometry`).
 */
export class Surface {
  readonly #builder: Builder;
  readonly #page: LivePage;

  /** @internal */
  constructor(builder: Builder, page: LivePage) {
    this.#builder = builder;
    this.#page = page;
  }

  /** Saves the graphics state (`q`). */
  save(): this {
    guard(() => this.#page.get().save());
    return this;
  }

  /** Restores the graphics state (`Q`). */
  restore(): this {
    guard(() => this.#page.get().restore());
    return this;
  }

  /** Applies `matrix` (page space) to what is drawn next. See {@link Matrix}. */
  transform(matrix: Transform): this {
    guard(() => this.#page.get().transform(new Float64Array(matrix)));
    return this;
  }

  /** The colour fills (and text) use. */
  fillColor(color: Color): this {
    guard(() =>
      typeof color === "string"
        ? this.#page.get().fillColorHex(color)
        : this.#page.get().fillColor(new Float64Array(color)),
    );
    return this;
  }

  /** The colour strokes use. */
  strokeColor(color: Color): this {
    guard(() =>
      typeof color === "string"
        ? this.#page.get().strokeColorHex(color)
        : this.#page.get().strokeColor(new Float64Array(color)),
    );
    return this;
  }

  /** Fill and stroke opacity, 0 to 1. */
  opacity(fill: number, stroke: number): this {
    guard(() => this.#page.get().opacity(fill, stroke));
    return this;
  }

  /** The blend mode. */
  blendMode(mode: BlendMode): this {
    guard(() => this.#page.get().blendMode(mode));
    return this;
  }

  /** @internal The stroke style the next stroke uses. */
  strokeStyle(style: StrokeStyle): void {
    guard(() =>
      this.#page
        .get()
        .strokeStyle(
          style.width ?? 1,
          style.cap ?? "butt",
          style.join ?? "miter",
          style.miterLimit ?? 10,
          new Float64Array(style.dash ?? []),
          style.dashPhase ?? 0,
        ),
    );
  }

  /**
   * A path, built by `build`, to paint.
   *
   * @example
   * ```ts
   * surface.path((p) => p.moveTo(56, 590).lineTo(539, 590)).stroke({ width: 0.5 });
   * ```
   */
  path(build: (p: PathBuilder) => void): PendingPath {
    const encoder = new PathEncoder();
    build(encoder);
    return new PendingPath(this, this.#page, encoder.finish());
  }

  /**
   * Draws `image` scaled into `rect`; with `alt`, as a `Figure` structure element with that
   * alternate text.
   */
  image(image: Image, rect: Rect, alt?: string): this {
    const raw = image.in(this.#builder.glue);
    const draw = () => guard(() => this.#page.get().image(raw, rect[0], rect[1], rect[2], rect[3]));
    if (alt !== undefined) return this.tag({ type: "Figure", alt }, draw);
    draw();
    return this;
  }

  /**
   * Draws what `draw` draws inside the structure element `tag`: its text, paths and images
   * become the element's content, elements opened inside it its children, links its `Link`
   * children, in reading order. Elements and artifacts `draw` leaves open end with it, also when
   * it throws. A page using an element makes the document tagged.
   *
   * @example
   * ```ts
   * s.tag("Sect", (s) => {
   *   s.text("Results", { font, size: 18, at: [56, 96], tag: "H1" });
   *   s.tag({ type: "P", lang: "en-GB" }, (s) => s.textBox(summary, { font, size: 11, box }));
   * });
   * ```
   * @throws {@link LaminaPdfError} `invalid-argument` for an unknown type without a `role`.
   */
  tag(tag: Tag, draw: (s: this) => void): this {
    this.#begin(tag);
    this.#within(draw);
    return this;
  }

  /**
   * Draws what `draw` draws as an artifact: decoration, running heads and footers, backgrounds;
   * content outside the structure tree.
   */
  artifact(kind: ArtifactKind, draw: (s: this) => void): this {
    const name = checked(ARTIFACT_KINDS, kind, "kind", "not an artifact kind");
    guard(() => this.#page.get().beginArtifact(name));
    this.#within(draw);
    return this;
  }

  #begin(tag: Tag): void {
    const t: TagSpec = typeof tag === "string" ? { type: tag } : tag;
    const custom = !(TAG_NAMES as readonly string[]).includes(t.type) && t.role !== undefined;
    const kind = custom
      ? checked(TAG_NAMES, t.role ?? "", "role", "not a standard structure type")
      : checked(TAG_NAMES, t.type, "type", "not a standard structure type (give a custom type a role)");
    const scope = t.scope === undefined ? undefined : checked(TABLE_SCOPES, t.scope, "scope", "not a table scope");
    guard(() =>
      this.#page.get().beginTag(kind, custom ? t.type : undefined, t.alt, t.actualText, t.lang, t.expansion, scope),
    );
  }

  /** Runs `draw`, then ends the element or artifact just begun. */
  #within(draw: (s: this) => void): void {
    this.#ended(() => draw(this));
  }

  /**
   * Runs `draw`, then ends the innermost element or artifact. When `draw` throws, its error is
   * the one thrown (ending is still tried, and its own failure, a finished page, ignored).
   */
  #ended<T>(draw: () => T): T {
    let result: T;
    try {
      result = draw();
    } catch (e) {
      try {
        this.#page.get().endTag();
      } catch {
        // `draw`'s error is the one to report.
      }
      throw e;
    }
    guard(() => this.#page.get().endTag());
    return result;
  }

  /** Runs `draw` inside `tag`, when there is one. */
  #tagged<T>(tag: Tag | undefined, draw: () => T): T {
    if (tag === undefined) return draw();
    this.#begin(tag);
    return this.#ended(draw);
  }

  /** A link annotation over `rect`, with `contents` as its alternate text. */
  link(rect: Rect, target: LinkTarget, contents?: string): this {
    guard(() => this.#page.get().link(rect[0], rect[1], rect[2], rect[3], target, contents));
    return this;
  }

  /**
   * Draws `text`: shaped, bidi-resolved, one line per line break (needs the text module).
   *
   * @returns what the run measures.
   * @throws {@link LaminaPdfError} `text-module-not-loaded`, `missing-glyph`, `font-not-shapable`, …
   */
  text(text: string, options: TextOptions): TextRunInfo {
    const font = this.#builder.wasmFont(options.font);
    const extra = rest(options, ["font", "size", "at", "tag"]);
    return this.#tagged(options.tag, () =>
      guard(() => this.#page.get().text(text, font, options.size, options.at[0], options.at[1], extra as never)),
    );
  }

  /**
   * Lays `text` out in `options.box`, breaking lines at Unicode line-break opportunities.
   *
   * @returns the lines drawn and where the overflow starts (UTF-16 offsets).
   * @throws {@link LaminaPdfError} as {@link Surface.text}; `text-overflow` with `overflow: "error"`.
   */
  textBox(text: string, options: TextBoxOptions): TextLayout {
    const font = this.#builder.wasmFont(options.font);
    const [x, y, w, h] = options.box;
    const extra = rest(options, ["font", "size", "box", "tag"]);
    return this.#tagged(options.tag, () =>
      guard(() => this.#page.get().textBox(text, font, options.size, x, y, w, h, extra as never)),
    );
  }
}

/** A page being drawn: its boxes and rotation, its {@link Surface}, then {@link Page.finish}. */
export class Page {
  readonly #page: LivePage;
  readonly #surface: Surface;

  /** @internal */
  constructor(builder: Builder, page: LivePage) {
    this.#page = page;
    this.#surface = new Surface(builder, this.#page);
  }

  /** What the page is drawn with (the same object on every call). */
  surface(): Surface {
    return this.#surface;
  }

  /** `/Rotate`: 0, 90, 180 or 270 degrees. */
  rotate(degrees: 0 | 90 | 180 | 270): this {
    guard(() => this.#page.get().rotate(degrees));
    return this;
  }

  /** `/UserUnit`, for pages larger than 14,400 units. */
  userUnit(unit: number): this {
    guard(() => this.#page.get().userUnit(unit));
    return this;
  }

  /** `/CropBox` (the visible region), in the media box's page space. */
  cropBox(rect: Rect): this {
    return this.#box("crop", rect);
  }

  /** `/BleedBox`. */
  bleedBox(rect: Rect): this {
    return this.#box("bleed", rect);
  }

  /** `/TrimBox`. */
  trimBox(rect: Rect): this {
    return this.#box("trim", rect);
  }

  /** `/ArtBox`. */
  artBox(rect: Rect): this {
    return this.#box("art", rect);
  }

  #box(kind: "crop" | "bleed" | "trim" | "art", rect: Rect): this {
    guard(() => this.#page.get().setBox(kind, rect[0], rect[1], rect[2], rect[3]));
    return this;
  }

  /**
   * Writes the page into its builder and frees the page's WASM side; the page takes no more
   * drawing afterwards (`builder-finished`).
   *
   * @returns the page's ID.
   * @throws {@link LaminaPdfError} `invalid-geometry`, `builder-finished`.
   */
  finish(): string {
    const page = this.#page.get();
    try {
      return guard(() => page.finish());
    } finally {
      this.#page.free();
    }
  }

  /** Drops the page without writing it (frees its WASM side). */
  free(): void {
    this.#page.free();
  }
}

/**
 * Builds a new document: pages, navigation and metadata; {@link Builder.finish} returns a
 * {@link Document} to save. Created in the text module when it is loaded (or `text: true`),
 * else in the reading module (text, extract, then base; vector and image pages only).
 *
 * @example
 * ```ts
 * await init({ text: true });
 * const b = new Builder({ producer: "my app" });
 * const page = b.page("A4");
 * page.surface().text("Hello, world!", { font: Font.standard("Helvetica"), size: 24, at: [72, 96] });
 * page.finish();
 * const bytes = b.finish().save();
 * ```
 */
export class Builder {
  #raw: TextBuilder | undefined;
  readonly #text: boolean;
  /** Registered fonts: standard 14 by name, others by their bytes (one PDF font per distinct font). */
  readonly #fonts = new Map<string | Uint8Array, WasmFont>();
  /** Pages not yet finished: freed with the builder, so they can't finish into nothing. */
  readonly #pages = new Set<LivePage>();
  /** @internal The module the document is built in. */
  readonly glue: BaseGlue | TextGlue;

  /**
   * @param options - version, file ID, producer, creation date, compression, tagging,
   *   conformance `profiles`; `text`.
   * @throws {@link LaminaPdfError} `not-initialized`, `text-module-not-loaded`; `unknown-option`.
   */
  constructor(options: BuilderOptions = {}) {
    const text = textModule();
    if (options.text === true && text === undefined) {
      throw packageError("text-module-not-loaded", "the text module is not loaded");
    }
    this.#text = options.text ?? text !== undefined;
    // Without text, the reading module (text, extract, then base): the built document can then
    // extract text whenever a module that extracts is loaded.
    this.glue = this.#text && text !== undefined ? text : readingModule();
    const glue = this.glue;
    const init = rest(options, ["text", "id"]) ?? {};
    const record = options.id === undefined ? init : { ...init, id: hexId(options.id) };
    this.#raw = guard(() => new glue.WasmBuilder(record as BuildOptions)) as unknown as TextBuilder;
  }

  get #builder(): TextBuilder {
    if (this.#raw === undefined) throw packageError("builder-finished", "the builder has already finished");
    return this.#raw;
  }

  /** As `new Builder(options)`. */
  static create(options?: BuilderOptions): Builder {
    return new Builder(options);
  }

  /** Whether this builder draws in the text module (fonts and text). */
  get hasText(): boolean {
    return this.#text;
  }

  /** A new page of a named size or `[width, height]` in points. */
  page(size: PageSizeName | readonly [width: number, height: number] = "A4"): Page {
    const [w, h] = typeof size === "string" ? namedSize(size) : size;
    const raw = guard(() => this.#builder.page(w, h)) as TextPageBuilder;
    const live = new LivePage(raw, (p) => this.#pages.delete(p));
    this.#pages.add(live);
    return new Page(this, live);
  }

  /**
   * Registers `font` with this builder (otherwise it is registered at first use).
   *
   * @throws {@link LaminaPdfError} `text-module-not-loaded`, `invalid-font`, `font-not-embeddable`.
   */
  font(font: Font): FontInfo {
    const raw = this.wasmFont(font);
    return { family: raw.family, postscriptName: raw.postscriptName, isStandard: raw.isStandard };
  }

  /** @internal The font in this builder's instance. */
  wasmFont(font: Font): WasmFont {
    const key = font.standard ?? this.#bytesKey(font.bytes ?? new Uint8Array());
    let raw = this.#fonts.get(key);
    if (raw !== undefined) return raw;
    if (!this.#text) {
      throw packageError("text-module-not-loaded", "this builder was created without the text module");
    }
    const builder = this.#builder;
    raw = guard(() =>
      font.bytes !== undefined ? builder.fontBytes(font.bytes) : builder.standardFont(font.standard ?? ""),
    );
    this.#fonts.set(key, raw);
    return raw;
  }

  /** The key of a font program already registered with equal bytes, else `bytes` itself. */
  #bytesKey(bytes: Uint8Array): Uint8Array {
    for (const key of this.#fonts.keys()) {
      if (typeof key !== "string" && sameBytes(key, bytes)) return key;
    }
    return bytes;
  }

  /** Defines the named destination `name`: page `index`, shown as `fit` (default: the page). */
  destination(name: string, index: number, fit?: Fit): this {
    guard(() => this.#builder.destination(name, index, fit));
    return this;
  }

  /** Adds outline (bookmark) items with their children. @returns their IDs, in pre-order. */
  outline(entries: readonly OutlineEntry[]): string[] {
    return guard(() => this.#builder.outline(entries as OutlineEntry[]));
  }

  /** The page labels (`i, ii, 1, 2…`), ranges by first page; checked at finish. */
  pageLabels(ranges: readonly PageLabel[]): this {
    guard(() => this.#builder.pageLabels(ranges as PageLabel[]));
    return this;
  }

  /** The metadata, written as the Info dictionary and an XMP packet. */
  metadata(metadata: Metadata): this {
    guard(() => this.#builder.metadata(metadata));
    return this;
  }

  /** The natural language (`/Lang`), a BCP 47 tag. */
  lang(tag: string): this {
    guard(() => this.#builder.lang(tag));
    return this;
  }

  /**
   * Attaches a file: PDF/A-3b's associated files, such as an e-invoice's XML. The same name
   * again replaces the file. PDF/A-2b and PDF/A-2u refuse attachments at {@link Builder.finish},
   * and every PDF/A profile refuses one without a MIME type there.
   *
   * @param name - the file name viewers show.
   * @param bytes - the file's contents.
   * @param mime - its MIME type, such as `text/xml`.
   * @param options - how the file relates to the document (`relationship`, default
   *   `unspecified`), its `description` and `modDate`.
   * @throws {@link LaminaPdfError} `invalid-argument` for a `modDate` that isn't a PDF date,
   *   `unknown-option` for an unknown key, `builder-finished` after {@link Builder.finish}.
   */
  attach(name: string, bytes: Uint8Array, mime: string, options?: AttachmentOptions): this {
    guard(() => this.#builder.attach(name, bytes, mime, options));
    return this;
  }

  /** How viewers present the document when it opens. */
  viewerPreferences(preferences: ViewerPreferences): this {
    guard(() => this.#builder.viewerPreferences(preferences));
    return this;
  }

  /**
   * Writes fonts, links, the page tree, outline and metadata; the builder (and its unfinished
   * pages) take no more calls afterwards.
   *
   * @returns the new document (unsaved; its save is a full rewrite).
   * @throws {@link LaminaPdfError} `page-index-out-of-range`, `unknown-destination`,
   *   `invalid-page-labels`, `invalid-geometry`, `subset-failed`, `builder-finished`; under a
   *   profile (`profiles` in the options), the first rule the document breaks, with that rule's
   *   code (the error reference lists them, from `profile-version-conflict` on).
   */
  finish(): Document {
    const builder = this.#builder;
    try {
      return new Document(guard(() => builder.finish()) as unknown as WasmDocument);
    } finally {
      this.free();
    }
  }

  /**
   * Frees the builder's WASM memory now (`finish` does this itself); the builder takes no more
   * calls afterwards. Unfinished pages are freed with {@link Page.free}.
   */
  free(): void {
    for (const page of [...this.#pages]) page.free();
    for (const font of this.#fonts.values()) font.free();
    this.#fonts.clear();
    (this.#raw as unknown as BaseBuilder | undefined)?.free();
    this.#raw = undefined;
  }

  /** Frees the builder at the end of a `using` block. */
  [Symbol.dispose](): void {
    this.free();
  }
}
