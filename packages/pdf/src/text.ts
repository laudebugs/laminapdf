// page.text() (B9c): a page's text as runs with glyph quads. The WASM hands over a struct of
// arrays (runs as records, every glyph's quad in one Float32Array, every glyph's text in one
// string); the objects here are built from it on demand, and `buffers` gives the arrays as they
// are, to post to a worker or the main thread without copying.
import type { PageTextJs, TextRunJs } from "@lamina/wasm/extract";

/** Options for {@link PageRef.text}. */
export interface TextOptions {
  /** Leave out text drawn invisibly (rendering modes 3 and 7: OCR layers, hidden text). */
  readonly skipInvisible?: boolean;
}

/** A point in page space (ADR 0027: top-left of the displayed CropBox, y down, points). */
export interface QuadPoint {
  readonly x: number;
  readonly y: number;
}

/** A glyph's quad: its four corners, in drawing order (bottom-left, bottom-right, top-right, top-left of the glyph box). */
export type GlyphQuad = readonly [QuadPoint, QuadPoint, QuadPoint, QuadPoint];

/** One glyph of a run. */
export interface TextGlyph {
  /** The glyph's text (document text; U+FFFD when its Unicode is unknown). */
  readonly text: string;
  /** Its quad in page space. */
  readonly quad: GlyphQuad;
}

/** A run: glyphs of one font, size, direction and marked content on one baseline, in logical order. */
export class TextRun {
  readonly #page: PageText;
  readonly #run: TextRunJs;

  readonly #range: readonly [number, number];

  /** @internal */
  constructor(page: PageText, run: TextRunJs) {
    this.#page = page;
    this.#run = run;
    this.#range = Object.freeze([run.glyphs[0], run.glyphs[1]] as const);
  }

  /** The run's text in logical order (document text). */
  get text(): string {
    return this.#run.text;
  }

  /** The font's `/BaseFont` (document text). */
  get font(): string {
    return this.#run.font;
  }

  /** Font size, the text matrix applied. */
  get size(): number {
    return this.#run.size;
  }

  /** Vertical writing. */
  get vertical(): boolean {
    return this.#run.vertical;
  }

  /** The marked-content ID the run is in, if any (structure tree links). */
  get mcid(): number | undefined {
    return this.#run.mcid;
  }

  /** The text comes from an `/ActualText` span. */
  get actualText(): boolean {
    return this.#run.actualText;
  }

  /** Drawn invisibly (`Tr` 3 or 7: an OCR layer, hidden text). */
  get invisible(): boolean {
    return this.#run.invisible;
  }

  /** The bounds of its glyph quads in page space: `{ x, y, width, height }`. */
  get bounds(): { readonly x: number; readonly y: number; readonly width: number; readonly height: number } {
    const [x, y, width, height] = this.#run.bounds;
    return { x, y, width, height };
  }

  /** The `[start, end)` range of its glyphs in the page's glyph arrays ({@link PageText.buffers}). */
  get glyphRange(): readonly [number, number] {
    return this.#range;
  }

  /** Its glyphs, with their quads (built on each call). */
  glyphs(): TextGlyph[] {
    const [start, end] = this.#run.glyphs;
    const out: TextGlyph[] = [];
    for (let i = start; i < end; i++) out.push(this.#page.glyph(i));
    return out;
  }
}

/** @internal The raw arrays of a PageText (the worker client sends them, L2a). */
export let rawOf: (text: PageText) => PageTextJs;

/** A page's extracted text: lines in reading order, and runs with their glyphs' quads. */
export class PageText {
  readonly #raw: PageTextJs;
  #runs: readonly TextRun[] | undefined;

  static {
    rawOf = (text) => text.#raw;
  }

  readonly #lines: readonly string[];

  /** @internal */
  constructor(raw: PageTextJs) {
    this.#raw = raw;
    this.#lines = Object.freeze([...raw.lines]);
    // Kept from the start: transferring the buffers detaches glyphOffsets.
    this.glyphCount = Math.max(0, raw.glyphOffsets.length - 1);
  }

  /** The text by line (runs on one baseline joined, trimmed), in content order. */
  get lines(): readonly string[] {
    return this.#lines;
  }

  /** The page's text: its lines joined with `\n`. */
  text(): string {
    return this.#lines.join("\n");
  }

  /** The runs, in content order (made on first use). */
  get runs(): readonly TextRun[] {
    this.#runs ??= this.#raw.runs.map((r) => new TextRun(this, r));
    return this.#runs;
  }

  /** Glyphs whose Unicode is unknown (shown as U+FFFD). */
  get unmapped(): number {
    return this.#raw.unmapped;
  }

  /** The number of glyphs. */
  readonly glyphCount: number;

  /**
   * Glyph `i`'s text and quad.
   *
   * @throws `RangeError` for an index outside `0 … glyphCount - 1`; `TypeError` once
   *   {@link PageText.buffers}' arrays were transferred to another thread.
   */
  glyph(i: number): TextGlyph {
    if (!Number.isInteger(i) || i < 0 || i >= this.glyphCount) {
      throw new RangeError(`glyph index ${i} is outside 0..${this.glyphCount - 1}`);
    }
    const q = this.#raw.quads;
    if (q.length < (i + 1) * 8 || this.#raw.glyphOffsets.length <= i + 1) {
      throw new TypeError("the glyph arrays were transferred (buffers): this PageText no longer holds them");
    }
    const o = i * 8;
    const point = (k: number): QuadPoint => ({ x: q[o + 2 * k] ?? 0, y: q[o + 2 * k + 1] ?? 0 });
    return {
      text: this.#raw.glyphText.slice(this.#raw.glyphOffsets[i] ?? 0, this.#raw.glyphOffsets[i + 1] ?? 0),
      quad: [point(0), point(1), point(2), point(3)],
    };
  }

  /**
   * The arrays as the WASM returned them: `quads` (8 numbers per glyph), `glyphText` and
   * `glyphOffsets` (glyph i is `glyphText.slice(glyphOffsets[i], glyphOffsets[i + 1])`). Post
   * `quads.buffer` and `glyphOffsets.buffer` as transferables to move them to another thread;
   * that detaches them here, and this PageText's `glyph()` and `glyphs()` then throw (`lines`
   * and `runs` keep working). They are this PageText's own arrays, not copies: writing to them
   * changes what `glyph()` returns.
   */
  get buffers(): { readonly quads: Float32Array; readonly glyphText: string; readonly glyphOffsets: Uint32Array } {
    return { quads: this.#raw.quads, glyphText: this.#raw.glyphText, glyphOffsets: this.#raw.glyphOffsets };
  }
}
