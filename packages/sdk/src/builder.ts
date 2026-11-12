// Builder, Page and Surface: new documents, page by page, in page space (points from the top-left
// of the displayed page, y down; ADR 0027). Over WasmBuilder / WasmPageBuilder (ADR 0016 §7):
// paths cross as flat Float64Arrays, colours as component arrays. Builders and pages are handles
// (internal/handle.ts): a page is freed with its builder.
import type { WasmBuilder as BaseBuilder, WasmDocument, WasmImage } from "@laminapdf/wasm/base";
import type { WasmBuilder as TextBuilder, WasmPageBuilder as TextPageBuilder, WasmFont } from "@laminapdf/wasm/text";
import { defined } from "./defined.js";
import { Document } from "./document.js";
import { didYouMean, packageError } from "./errors.js";
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
import type { FlowDocument } from "./generated/document.js";
import { type BaseGlue, readingModule, type TextGlue, textModule } from "./init.js";
import { disposable, forget, Handle, watch } from "./internal/handle.js";
import type { Instance } from "./internal/instance.js";
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

/**
 * Named page sizes (ISO 216 sizes from millimetres, as the Rust `PageSize` computes them).
 *
 * @since 0.2.0
 */
export type PageSizeName = "A0" | "A1" | "A2" | "A3" | "A4" | "A5" | "A6" | "Letter" | "Legal" | "Tabloid";

const mm = (w: number, h: number): readonly [number, number] => [(w * 72) / 25.4, (h * 72) / 25.4];

/**
 * Each named size in points, width × height.
 *
 * @since 0.2.0
 */
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

/**
 * Options for a new {@link Builder}.
 *
 * @since 0.2.0
 */
export interface BuilderOptions extends Omit<BuildOptions, "id"> {
  /** `/ID[0]`: 16 bytes, as 32 hex digits or a `Uint8Array` (default: derived from the content). */
  readonly id?: string | Uint8Array;
  /**
   * Build in the text module (fonts and text). Default: when it is loaded. `true` without the
   * text module loaded throws `text-module-not-loaded`.
   */
  readonly text?: boolean;
}

/**
 * How strokes are drawn.
 *
 * @since 0.2.0
 */
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

/**
 * A structure element with attributes; `type` may be a custom name when `role` is given.
 *
 * @since 0.2.0
 */
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

/**
 * A structure element: a type, or a type with attributes.
 *
 * @since 0.2.0
 */
export type Tag = TagName | TagSpec;

/**
 * Options of {@link Surface.text}.
 *
 * @since 0.2.0
 */
export interface TextOptions {
  /** The font (needs the text module). */
  readonly font: Font;
  /** Size in points. */
  readonly size: number;
  /** Where the first line starts, in page space: `x`, and `y` is the baseline, not the top of the text. */
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

/**
 * Options of {@link Surface.textBox}.
 *
 * @since 0.2.0
 */
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

/**
 * The fonts {@link Builder.markdown} sets each kind of text in.
 *
 * @since 0.2.0
 */
export interface MarkdownFonts {
  /** Body text; the other roles fall back to it. */
  readonly regular: Font;
  /** Bold text and headings. */
  readonly bold?: Font;
  /** Italic text. */
  readonly italic?: Font;
  /** Bold italic text. */
  readonly boldItalic?: Font;
  /** Code. */
  readonly mono?: Font;
  /** Fonts for characters the others lack, tried in order (Arabic, Devanagari, …). */
  readonly fallbacks?: readonly Font[];
}

/**
 * A header or footer: text at the start, centre and end, with `{page}`, `{pages}`, `{section}` and `{title}` filled in.
 *
 * @since 0.2.0
 */
export interface PageRegion {
  readonly start?: string;
  readonly center?: string;
  readonly end?: string;
}

/**
 * The pages a {@link Builder.markdown} document flows through.
 *
 * @since 0.2.0
 */
export interface PageTemplate {
  /** A page size name or `[width, height]` in points. */
  readonly size: PageSizeName | readonly [width: number, height: number];
  /** Top, right, bottom and left margins in points (72 each by default). */
  readonly margins?: readonly [top: number, right: number, bottom: number, left: number];
  readonly header?: PageRegion;
  readonly footer?: PageRegion;
  /** The header and footer text's size in points, in the regular font (default 8). */
  readonly furnitureSize?: number;
}

/**
 * Options of {@link Builder.markdown}.
 *
 * @since 0.2.0
 */
export interface MarkdownOptions {
  /** The fonts by role; without them, Helvetica and Courier: Latin only, which PDF/UA-1 refuses (`font-not-embedded`). */
  readonly fonts?: MarkdownFonts;
  /** The images the Markdown shows, by the target it writes (`![…](photo.jpg)` → `"photo.jpg"`). Nothing is fetched. */
  readonly images?: Readonly<Record<string, Image>>;
  /** The body text's size in points (default 10.5); headings scale from it. */
  readonly size?: number;
  /** A contents page listing headings of levels 1 to `contents`, at the start. */
  readonly contents?: number;
  /** The pages (default A4, 56-point margins, "Page {page} of {pages}" in the footer). */
  readonly page?: PageTemplate;
}

/** A pointer into the data (`"/items"`) or, inside an `$each`, into the current item. */
type DataPointer = string | { readonly $item: string };

/** A value a template fills in: `{ $data: "/invoice/number" }`, or `{ $item: "/name" }` inside an `$each`. */
type ValuePlaceholder = { readonly $data: string } | { readonly $item: string };

/** What a list placeholder's `do` adds to its list: an item, another list placeholder, or several of either. */
type ListBody<T> = Templated<T> | ListPlaceholder<T> | readonly (Templated<T> | ListPlaceholder<T>)[];

/** In a list, `do` once per item of a list in the data (`$each`), or only when a value is there and isn't `false` or `null` (`$if`). They nest, as `fill` reads them. */
type ListPlaceholder<T> =
  | { readonly $each: DataPointer; readonly do: ListBody<T> }
  | { readonly $if: DataPointer; readonly do: ListBody<T> };

/**
 * Keys kept as written in a template. `type` and `schema` say what an object is, so a misspelt key
 * elsewhere in a block is reported as that, not as a type mismatch; `profiles` is read before
 * filling, to give the builder the profiles the template asks for.
 */
type AsWritten = "type" | "schema" | "profiles";

/**
 * `T` with a template's placeholders allowed where `fill` takes them: a value for text, numbers and `true`/`false`, a list placeholder among a list's items. `type`, `schema` and `profiles` are written as they are, with no placeholder.
 *
 * @since 0.2.0
 */
export type Templated<T> = T extends readonly (infer U)[]
  ? readonly (Templated<U> | ListPlaceholder<U>)[]
  : T extends object
    ? { readonly [K in keyof T]: K extends AsWritten ? T[K] : Templated<T[K]> }
    : T | ValuePlaceholder;

/**
 * A `laminapdf.document/1` template: a {@link FlowDocument}, with the same keys, in which a value
 * can be a placeholder that {@link Builder.flow} fills from `options.data`.
 *
 * - `{ $data: "/invoice/number" }` takes a value from the data, by JSON Pointer.
 * - `{ $each: "/items", do: … }`, in a list, adds `do` once per item of a list in the data;
 *   inside it, `{ $item: "/name" }` takes a value from the current item.
 * - `{ $if: "/paid", do: … }`, in a list, adds `do` only when the value is there and isn't
 *   `false` or `null`.
 *
 * Placeholders nest. `type`, `schema` and `profiles` take none: `profiles` is read before filling,
 * for `new Builder({ profiles })`.
 *
 * @example
 * ```ts
 * const template: FlowTemplate = {
 *   schema: "laminapdf.document/1",
 *   styles: { body: { font: "Helvetica", size: 11 } },
 *   blocks: [
 *     { type: "heading", level: 1, text: { $data: "/title" } },
 *     { $each: "/lines", do: { type: "paragraph", text: { $item: "/text" } } },
 *   ],
 * };
 * const data = { title: "Invoice 42", lines: [{ text: "Paper, 2 reams" }] };
 * builder.flow(template, { data });
 * ```
 *
 * @since 0.2.0
 */
// An interface, not an alias of `Templated<FlowDocument>`, so that TypeDoc names it where it is
// used rather than spelling out every key of every block.
export interface FlowTemplate extends Templated<FlowDocument> {}

/**
 * Options of {@link Builder.flow}.
 *
 * @since 0.2.0
 */
export interface FlowOptions {
  /** The data a template's placeholders read; without it, the document is read as it is. */
  readonly data?: unknown;
  /** The fonts the document's styles name, by name; without one, a style that names it fails with `unknown-font`. */
  readonly fonts?: Readonly<Record<string, Font>>;
  /** The images the document's figures name, by name. Nothing is fetched. */
  readonly images?: Readonly<Record<string, Image>>;
}

/**
 * What {@link Builder.flow} added.
 *
 * @since 0.2.0
 */
export interface FlowReport {
  /** The pages it added, in order, by ID. */
  readonly pages: readonly string[];
}

/**
 * What {@link Builder.markdown} added.
 *
 * @since 0.2.0
 */
export interface MarkdownReport {
  /** The pages it added, in order, by ID. */
  readonly pages: readonly string[];
}

/** `page` as the WebAssembly record takes it: the size in points, the tuples copied. */
function pageRecord(page: PageTemplate) {
  const [width, height] = typeof page.size === "string" ? namedSize(page.size) : page.size;
  const { margins, size: _size, ...rest } = page;
  const sized = { ...rest, size: [width, height] as [number, number] };
  return margins === undefined ? sized : { ...sized, margins: [...margins] as [number, number, number, number] };
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

/**
 * The raw page of a {@link Page}, a handle freed with its builder: `builder-finished` once the page
 * or its builder finishes, `disposed` once either is freed.
 */
class LivePage extends Handle<BaseGlue> {
  #raw: TextPageBuilder | undefined;
  readonly #onClose: (page: LivePage) => void;

  constructor(raw: TextPageBuilder, builder: Builder, instance: Instance<BaseGlue>, onClose: (page: LivePage) => void) {
    super("Page", instance, builder);
    this.#raw = raw;
    this.#onClose = onClose;
  }

  /**
   * Runs `f`, calls into the glue, on this page's raw side.
   *
   * @throws {@link LaminaPdfError} `disposed`, `instance-reset`, `builder-finished`; what `f` throws.
   */
  run<T>(f: (page: TextPageBuilder) => T): T {
    return this.call(() => {
      if (this.#raw === undefined) throw packageError("builder-finished", "the page has already finished");
      return f(this.#raw);
    });
  }

  /** Frees the raw page and keeps the handle: later calls throw `builder-finished`. */
  close(): void {
    if (this.owned) this.#raw?.free();
    this.#raw = undefined;
    this.#onClose(this);
  }

  protected release(): void {
    this.close();
  }
}

/**
 * A path waiting to be painted: fill it, stroke it, both, or clip to it.
 *
 * @since 0.2.0
 */
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

  /**
   * Fills the path with the fill colour. @returns the surface.
   *
   * @since 0.2.0
   */
  fill(rule: FillRule = "nonZero"): Surface {
    this.#page.run((page) => page.fillPath(this.#path, rule));
    return this.#surface;
  }

  /**
   * Strokes the path with the stroke colour and `style`. @returns the surface.
   *
   * @since 0.2.0
   */
  stroke(style: StrokeStyle = {}): Surface {
    this.#surface.strokeStyle(style);
    this.#page.run((page) => page.strokePath(this.#path));
    return this.#surface;
  }

  /**
   * Fills, then strokes the path. @returns the surface.
   *
   * @since 0.2.0
   */
  fillStroke(rule: FillRule = "nonZero", style: StrokeStyle = {}): Surface {
    this.#surface.strokeStyle(style);
    this.#page.run((page) => page.fillStrokePath(this.#path, rule));
    return this.#surface;
  }

  /**
   * Clips what is drawn next to the path, until the next `restore`. @returns the surface.
   *
   * @since 0.2.0
   */
  clip(rule: FillRule = "nonZero"): Surface {
    this.#page.run((page) => page.clipPath(this.#path, rule));
    return this.#surface;
  }
}

/**
 * What a page is drawn with. Methods that set state return the surface, so calls chain; invalid
 * geometry is reported by {@link Page.finish} (`invalid-geometry`).
 *
 * @since 0.2.0
 */
export class Surface {
  readonly #builder: Builder;
  readonly #page: LivePage;

  /** @internal */
  constructor(builder: Builder, page: LivePage) {
    this.#builder = builder;
    this.#page = page;
  }

  /**
   * Saves the graphics state (`q`).
   *
   * @since 0.2.0
   */
  save(): this {
    this.#page.run((page) => page.save());
    return this;
  }

  /**
   * Restores the graphics state (`Q`).
   *
   * @since 0.2.0
   */
  restore(): this {
    this.#page.run((page) => page.restore());
    return this;
  }

  /**
   * Applies `matrix` (page space) to what is drawn next. See {@link Matrix}.
   *
   * @since 0.2.0
   */
  transform(matrix: Transform): this {
    this.#page.run((page) => page.transform(new Float64Array(matrix)));
    return this;
  }

  /**
   * The colour fills (and text) use.
   *
   * @since 0.2.0
   */
  fillColor(color: Color): this {
    this.#page.run((page) =>
      typeof color === "string" ? page.fillColorHex(color) : page.fillColor(new Float64Array(color)),
    );
    return this;
  }

  /**
   * The colour strokes use.
   *
   * @since 0.2.0
   */
  strokeColor(color: Color): this {
    this.#page.run((page) =>
      typeof color === "string" ? page.strokeColorHex(color) : page.strokeColor(new Float64Array(color)),
    );
    return this;
  }

  /**
   * Fill and stroke opacity, 0 to 1.
   *
   * @since 0.2.0
   */
  opacity(fill: number, stroke: number): this {
    this.#page.run((page) => page.opacity(fill, stroke));
    return this;
  }

  /**
   * The blend mode.
   *
   * @since 0.2.0
   */
  blendMode(mode: BlendMode): this {
    this.#page.run((page) => page.blendMode(mode));
    return this;
  }

  /** @internal The stroke style the next stroke uses. */
  strokeStyle(style: StrokeStyle): void {
    this.#page.run((page) =>
      page.strokeStyle(
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
   *
   * @since 0.2.0
   */
  path(build: (p: PathBuilder) => void): PendingPath {
    const encoder = new PathEncoder();
    build(encoder);
    return new PendingPath(this, this.#page, encoder.finish());
  }

  /**
   * Draws `image` scaled into `rect`; with `alt`, as a `Figure` structure element with that
   * alternate text.
   *
   * @since 0.2.0
   */
  image(image: Image, rect: Rect, alt?: string): this {
    const raw = this.#builder.wasmImage(image);
    const draw = () => this.#page.run((page) => page.image(raw, rect[0], rect[1], rect[2], rect[3]));
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
   *
   * @since 0.2.0
   */
  tag(tag: Tag, draw: (s: this) => void): this {
    this.#begin(tag);
    this.#within(draw);
    return this;
  }

  /**
   * Draws what `draw` draws as an artifact: decoration, running heads and footers, backgrounds;
   * content outside the structure tree. `"header"`, `"footer"` and `"watermark"` are pagination
   * artifacts that also say which (`/Subtype`), a header attached to the page's top edge and a
   * footer to its bottom (`/Attached`).
   *
   * @since 0.2.0
   */
  artifact(kind: ArtifactKind, draw: (s: this) => void): this {
    const name = checked(ARTIFACT_KINDS, kind, "kind", "not an artifact kind");
    this.#page.run((page) => page.beginArtifact(name));
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
    this.#page.run((page) =>
      page.beginTag(kind, custom ? t.type : undefined, t.alt, t.actualText, t.lang, t.expansion, scope),
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
        this.#page.run((page) => page.endTag());
      } catch {
        // `draw`'s error is the one to report.
      }
      throw e;
    }
    this.#page.run((page) => page.endTag());
    return result;
  }

  /** Runs `draw` inside `tag`, when there is one. */
  #tagged<T>(tag: Tag | undefined, draw: () => T): T {
    if (tag === undefined) return draw();
    this.#begin(tag);
    return this.#ended(draw);
  }

  /**
   * A link annotation over `rect`, with `contents` as its alternate text.
   *
   * @since 0.2.0
   */
  link(rect: Rect, target: LinkTarget, contents?: string): this {
    this.#page.run((page) => page.link(rect[0], rect[1], rect[2], rect[3], target, contents));
    return this;
  }

  /**
   * Draws `text`: shaped, bidi-resolved, one line per line break (needs the text module).
   *
   * @returns what the run measures, with `bottom`: where the next element can start (the next text box's top), the
   *   descent below the last line that draws something, before the surface's transform; left out when nothing is.
   * @throws {@link LaminaPdfError} `text-module-not-loaded`, `missing-glyph`, `font-not-shapable`, …
   *
   * @since 0.2.0
   */
  text(text: string, options: TextOptions): TextRunInfo {
    const font = this.#builder.wasmFont(options.font);
    const extra = rest(options, ["font", "size", "at", "tag"]);
    return this.#tagged(options.tag, () =>
      this.#page.run((page) => page.text(text, font, options.size, options.at[0], options.at[1], extra as never)),
    );
  }

  /**
   * Lays `text` out in `options.box`, breaking lines at Unicode line-break opportunities.
   *
   * @returns the lines drawn, where the overflow starts (UTF-16 offsets; test it with `!== undefined`, as `0` means no
   *   line fitted) and `bottom`: the last line that draws something, before the surface's transform; left out when none
   *   does.
   * @throws {@link LaminaPdfError} as {@link Surface.text}; `text-overflow` with `overflow: "error"`.
   *
   * @since 0.2.0
   */
  textBox(text: string, options: TextBoxOptions): TextLayout {
    const font = this.#builder.wasmFont(options.font);
    const [x, y, w, h] = options.box;
    const extra = rest(options, ["font", "size", "box", "tag"]);
    return this.#tagged(options.tag, () =>
      this.#page.run((page) => page.textBox(text, font, options.size, x, y, w, h, extra as never)),
    );
  }
}

/**
 * A page being drawn: its boxes and rotation, its {@link Surface}, then {@link Page.finish}. Freed
 * with its builder; `using` or {@link Page.free} drops it without writing it.
 *
 * @since 0.2.0
 */
export class Page {
  readonly #page: LivePage;
  readonly #surface: Surface;

  /** @internal */
  constructor(builder: Builder, page: LivePage) {
    this.#page = page;
    this.#surface = new Surface(builder, this.#page);
  }

  /**
   * What the page is drawn with (the same object on every call).
   *
   * @since 0.2.0
   */
  surface(): Surface {
    return this.#surface;
  }

  /**
   * `/Rotate`: 0, 90, 180 or 270 degrees.
   *
   * @since 0.2.0
   */
  rotate(degrees: 0 | 90 | 180 | 270): this {
    this.#page.run((page) => page.rotate(degrees));
    return this;
  }

  /**
   * `/UserUnit`, for pages larger than 14,400 units.
   *
   * @since 0.2.0
   */
  userUnit(unit: number): this {
    this.#page.run((page) => page.userUnit(unit));
    return this;
  }

  /**
   * `/CropBox` (the visible region), in the media box's page space.
   *
   * @since 0.2.0
   */
  cropBox(rect: Rect): this {
    return this.#box("crop", rect);
  }

  /**
   * `/BleedBox`.
   *
   * @since 0.2.0
   */
  bleedBox(rect: Rect): this {
    return this.#box("bleed", rect);
  }

  /**
   * `/TrimBox`.
   *
   * @since 0.2.0
   */
  trimBox(rect: Rect): this {
    return this.#box("trim", rect);
  }

  /**
   * `/ArtBox`.
   *
   * @since 0.2.0
   */
  artBox(rect: Rect): this {
    return this.#box("art", rect);
  }

  #box(kind: "crop" | "bleed" | "trim" | "art", rect: Rect): this {
    this.#page.run((page) => page.setBox(kind, rect[0], rect[1], rect[2], rect[3]));
    return this;
  }

  /**
   * Writes the page into its builder and frees the page's WASM side; the page takes no more
   * drawing afterwards (`builder-finished`).
   *
   * @returns the page's ID.
   * @throws {@link LaminaPdfError} `invalid-geometry`, `builder-finished`.
   *
   * @since 0.2.0
   */
  finish(): string {
    try {
      return this.#page.run((page) => page.finish());
    } finally {
      // After a trap, `close` frees nothing: the instance is about to be replaced.
      this.#page.close();
    }
  }

  /**
   * Drops the page without writing it and frees its WASM side; a second call does nothing, and
   * any other call afterwards throws `disposed`.
   *
   * @since 0.2.0
   */
  free(): void {
    this.#page.free();
  }

  /**
   * Drops the page at the end of a `using` block (an alias of `free()`).
   *
   * @since 0.2.0
   */
  [Symbol.dispose](): void {
    this.free();
  }
}

disposable(Page);

/**
 * Builds a new document: pages, navigation and metadata; {@link Builder.finish} returns a
 * {@link Document} to save. Created in the text module when it is loaded (or `text: true`),
 * else in the reading module (text, extract, then base; vector and image pages only). `finish()`
 * frees its WebAssembly memory; to drop it unfinished, use `using` or {@link Builder.free}.
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
 *
 * @since 0.2.0
 */
export class Builder extends Handle<BaseGlue> {
  #raw: TextBuilder | undefined;
  readonly #text: boolean;
  /**
   * Registered fonts: standard 14 by name, others by their bytes and then the face (-1 for a font
   * that isn't in a collection): one PDF font per distinct font.
   */
  readonly #fonts = new Map<string | Uint8Array, Map<number, WasmFont>>();
  /** Pages not yet finished: freed with the builder, so they can't finish into nothing. */
  readonly #pages = new Set<LivePage>();

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
    const withText = options.text ?? text !== undefined;
    // Without text, the reading module (text, extract, then base): the built document can then
    // extract text whenever a module that extracts is loaded.
    super("Builder", (withText && text !== undefined ? text : readingModule()) as unknown as Instance<BaseGlue>);
    this.#text = withText;
    const init = rest(options, ["text", "id"]) ?? {};
    const record = options.id === undefined ? init : { ...init, id: hexId(options.id) };
    this.#raw = this.instance.call((glue) => new glue.WasmBuilder(record as BuildOptions)) as unknown as TextBuilder;
    watch(this, "Builder");
  }

  /** @throws {@link LaminaPdfError} `disposed`, `instance-reset`, `builder-finished`. */
  get #builder(): TextBuilder {
    this.assertLive();
    if (this.#raw === undefined) throw packageError("builder-finished", "the builder has already finished");
    return this.#raw;
  }

  /** @internal `image` in this builder's instance. */
  wasmImage(image: Image): WasmImage {
    return image.in(this.instance);
  }

  /**
   * As `new Builder(options)`.
   *
   * @since 0.2.0
   */
  static create(options?: BuilderOptions): Builder {
    return new Builder(options);
  }

  /**
   * Whether this builder draws in the text module (fonts and text).
   *
   * @since 0.2.0
   */
  get hasText(): boolean {
    return this.#text;
  }

  /**
   * A new page of a named size or `[width, height]` in points.
   *
   * @since 0.2.0
   */
  page(size: PageSizeName | readonly [width: number, height: number] = "A4"): Page {
    const [w, h] = typeof size === "string" ? namedSize(size) : size;
    const raw = this.call(() => this.#builder.page(w, h)) as TextPageBuilder;
    const live = new LivePage(raw, this, this.instance, (p) => this.#pages.delete(p));
    this.#pages.add(live);
    return new Page(this, live);
  }

  /**
   * Registers `font` with this builder (otherwise it is registered at first use).
   *
   * @throws {@link LaminaPdfError} `text-module-not-loaded`, `invalid-font`, `font-not-embeddable`.
   *
   * @since 0.2.0
   */
  font(font: Font): FontInfo {
    const raw = this.wasmFont(font);
    return this.call(() => ({ family: raw.family, postscriptName: raw.postscriptName, isStandard: raw.isStandard }));
  }

  /** @internal The font in this builder's instance. */
  wasmFont(font: Font): WasmFont {
    // A cached font lives in the builder's instance: check it is still the current one.
    this.assertLive();
    const key = font.standard ?? this.#bytesKey(font.bytes ?? new Uint8Array());
    const face = font.index ?? -1;
    const faces = this.#fonts.get(key) ?? new Map<number, WasmFont>();
    let raw = faces.get(face);
    if (raw !== undefined) return raw;
    if (!this.#text) {
      throw packageError("text-module-not-loaded", "this builder was created without the text module");
    }
    const builder = this.#builder;
    raw = this.call(() => {
      if (font.bytes === undefined) return builder.standardFont(font.standard ?? "");
      return font.index === undefined ? builder.fontBytes(font.bytes) : builder.fontCollection(font.bytes, font.index);
    });
    faces.set(face, raw);
    this.#fonts.set(key, faces);
    return raw;
  }

  /** The key of a font program already registered with equal bytes, else `bytes` itself. */
  #bytesKey(bytes: Uint8Array): Uint8Array {
    for (const key of this.#fonts.keys()) {
      if (typeof key !== "string" && sameBytes(key, bytes)) return key;
    }
    return bytes;
  }

  /**
   * Defines the named destination `name`: page `index`, shown as `fit` (default: the page).
   *
   * @since 0.2.0
   */
  destination(name: string, index: number, fit?: Fit): this {
    this.call(() => this.#builder.destination(name, index, fit));
    return this;
  }

  /**
   * Adds outline (bookmark) items with their children. @returns their IDs, in pre-order.
   *
   * @since 0.2.0
   */
  outline(entries: readonly OutlineEntry[]): string[] {
    return this.call(() => this.#builder.outline(entries as OutlineEntry[]));
  }

  /**
   * The page labels (`i, ii, 1, 2…`), ranges by first page; checked at finish.
   *
   * @since 0.2.0
   */
  pageLabels(ranges: readonly PageLabel[]): this {
    this.call(() => this.#builder.pageLabels(ranges as PageLabel[]));
    return this;
  }

  /**
   * Lays `markdown` out as tagged pages that flow, after any pages already added: CommonMark with
   * tables, footnotes, strikethrough and task lists. The front matter (`title`, `lang`, `author`,
   * `subject` as `key: value` lines) becomes the document's metadata and language. Raw HTML and
   * images not given in `options.images` are left out with the warnings `markdown-html-ignored`
   * and `markdown-image-missing`, which the finished document's `warnings()` lists. Needs the text module.
   *
   * @throws {@link LaminaPdfError} `text-module-not-loaded`; `unknown-option`, `invalid-argument`;
   *   `missing-glyph` for a character the fonts lack; `flow-content-too-large`; `builder-finished`.
   *
   * @since 0.2.0
   */
  markdown(markdown: string, options: MarkdownOptions = {}): MarkdownReport {
    if (!this.#text) {
      throw packageError("text-module-not-loaded", "this builder was created without the text module");
    }
    const { fonts, images, page, ...layout } = options;
    const roles: [string, Font][] = [];
    if (fonts !== undefined) {
      roles.push(["regular", fonts.regular]);
      for (const role of ["bold", "italic", "boldItalic", "mono"] as const) {
        const font = fonts[role];
        if (font !== undefined) roles.push([role, font]);
      }
      for (const font of fonts.fallbacks ?? []) roles.push(["fallback", font]);
    }
    const raw = roles.map(([role, font]) => [role, this.wasmFont(font)] as const);
    const pictures = Object.entries(images ?? {}).map(([target, image]) => [target, this.wasmImage(image)] as const);
    const template = page === undefined ? undefined : pageRecord(page);
    const builder = this.#builder;
    return this.call((glue) => {
      const resources = new (glue as unknown as TextGlue).WasmMarkdownOptions();
      try {
        for (const [role, font] of raw) resources.font(role, font);
        for (const [target, image] of pictures) resources.image(target, image);
        const record = template === undefined ? layout : { ...layout, page: template };
        return builder.markdown(markdown, resources, record) as MarkdownReport;
      } finally {
        resources.free();
      }
    });
  }

  /**
   * Lays out a `laminapdf.document/1` document as tagged pages that flow, after any pages already
   * added. Given `options.data`, the document is a template: its `$data`, `$item`, `$each` and
   * `$if` placeholders are filled from the data first. The fonts and images it names come by
   * name in `options`. Its `profiles` are the builder's to check: pass the same ones to
   * `new Builder({ profiles })`. Its warnings are the finished document's.
   * Needs the text module.
   *
   * @example
   * ```ts
   * const builder = new Builder({ profiles: ["PDF/UA-1"] });
   * builder.flow(template, { data, fonts: { regular, bold } });
   * ```
   * @throws {@link LaminaPdfError} `text-module-not-loaded`; `unknown-option` for an unknown
   *   option; `invalid-document` with the JSON Pointer at fault in its `data` and the closest
   *   keys in `didYouMean`; `template-data-missing`, `template-data-type` and
   *   `template-too-large` with the pointer in `data`; `unknown-font` and `unknown-image` for a
   *   name `options` lacks; `missing-glyph`; `flow-content-too-large`; `builder-finished`.
   *
   * @since 0.2.0
   */
  flow(document: FlowTemplate, options: FlowOptions = {}): FlowReport {
    if (!this.#text) {
      throw packageError("text-module-not-loaded", "this builder was created without the text module");
    }
    const { fonts, images, ...rest } = options;
    const named = Object.entries(fonts ?? {}).map(([name, font]) => [name, this.wasmFont(font)] as const);
    const pictures = Object.entries(images ?? {}).map(([name, image]) => [name, this.wasmImage(image)] as const);
    const builder = this.#builder;
    return this.call((glue) => {
      const resources = new (glue as unknown as TextGlue).WasmFlowResources();
      try {
        for (const [name, font] of named) resources.font(name, font);
        for (const [name, image] of pictures) resources.image(name, image);
        return builder.flow(document, resources, rest) as FlowReport;
      } finally {
        resources.free();
      }
    });
  }

  /**
   * The metadata, written as the Info dictionary and an XMP packet.
   *
   * @since 0.2.0
   */
  metadata(metadata: Metadata): this {
    this.call(() => this.#builder.metadata(metadata));
    return this;
  }

  /**
   * The natural language (`/Lang`), a BCP 47 tag.
   *
   * @since 0.2.0
   */
  lang(tag: string): this {
    this.call(() => this.#builder.lang(tag));
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
   *
   * @since 0.2.0
   */
  attach(name: string, bytes: Uint8Array, mime: string, options?: AttachmentOptions): this {
    this.call(() => this.#builder.attach(name, bytes, mime, options));
    return this;
  }

  /**
   * How viewers present the document when it opens.
   *
   * @since 0.2.0
   */
  viewerPreferences(preferences: ViewerPreferences): this {
    this.call(() => this.#builder.viewerPreferences(preferences));
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
   *
   * @since 0.2.0
   */
  finish(): Document {
    const builder = this.#builder;
    try {
      return new Document(this.call(() => builder.finish()) as unknown as WasmDocument, this.instance);
    } finally {
      // After a trap, `#close` frees nothing: the instance is about to be replaced.
      this.#close();
    }
  }

  /**
   * Frees the raw builder, its fonts and unfinished pages, and keeps the handle: later calls throw
   * `builder-finished` (`free()` makes them `disposed`).
   */
  #close(): void {
    // Its WASM side is freed (or gone with its instance): not a leak.
    forget(this);
    for (const page of [...this.#pages]) page.close();
    if (this.owned) {
      for (const faces of this.#fonts.values()) for (const font of faces.values()) font.free();
      (this.#raw as unknown as BaseBuilder | undefined)?.free();
    }
    this.#fonts.clear();
    this.#raw = undefined;
  }

  protected release(): void {
    this.#close();
  }
}
