// Fonts and images: descriptions that any builder can use. A document is built in one WASM
// instance (base or text, ADR 0016 §4), so the instance-side objects are made per builder (fonts)
// or per module (images) when first used.
import type { WasmImage } from "@laminapdf/wasm/base";
import { packageError } from "./errors.js";
import { type BaseGlue, baseModule } from "./init.js";
import { disposable, forget, watch } from "./internal/handle.js";
import type { Instance } from "./internal/instance.js";

/** The standard 14 fonts, by PostScript name: no bytes needed, not embedded, Latin only, unshaped. */
export type Standard14 =
  | "Times-Roman"
  | "Times-Bold"
  | "Times-Italic"
  | "Times-BoldItalic"
  | "Helvetica"
  | "Helvetica-Bold"
  | "Helvetica-Oblique"
  | "Helvetica-BoldOblique"
  | "Courier"
  | "Courier-Bold"
  | "Courier-Oblique"
  | "Courier-BoldOblique"
  | "Symbol"
  | "ZapfDingbats";

/** What a builder knows of a font once registered. */
export interface FontInfo {
  /** The family name. */
  readonly family: string;
  /** The PostScript name. */
  readonly postscriptName: string;
  /** Whether it is one of the standard 14 (not embedded). */
  readonly isStandard: boolean;
}

/**
 * A font to draw text with: TrueType or OpenType bytes (subset to the glyphs drawn and embedded)
 * or one of the standard 14. Needs the text module (`init({ text: true })`).
 *
 * @example
 * ```ts
 * const inter = Font.fromBytes(await (await fetch("/Inter.ttf")).arrayBuffer());
 * surface.text("Hello", { font: inter, size: 12, at: [72, 72] });
 * ```
 */
export class Font {
  /** The font program, for a font loaded from bytes. */
  readonly bytes: Uint8Array | undefined;
  /** The PostScript name, for a standard 14 font. */
  readonly standard: Standard14 | undefined;
  /** The face in a collection (`.ttc`), for a font loaded with {@link Font.fromCollection}. */
  readonly index: number | undefined;

  private constructor(bytes: Uint8Array | undefined, standard: Standard14 | undefined, index?: number) {
    this.bytes = bytes;
    this.standard = standard;
    this.index = index;
  }

  /**
   * A font from its bytes (TrueType or OpenType/CFF). The bytes are checked when a builder first
   * uses the font (`invalid-font`, `font-not-embeddable`).
   */
  static fromBytes(bytes: Uint8Array | ArrayBuffer): Font {
    return new Font(bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes), undefined);
  }

  /**
   * Face `index` (from 0) of a font collection (`.ttc` or `.otc`). The bytes are checked when a
   * builder first uses the font (`invalid-font` for a file that isn't a collection or has no such face).
   *
   * @throws {@link LaminaPdfError} `invalid-argument` for an index that isn't a whole number from 0.
   */
  static fromCollection(bytes: Uint8Array | ArrayBuffer, index: number): Font {
    if (!Number.isInteger(index) || index < 0 || index > 0xffff_ffff) {
      throw packageError("invalid-argument", "a collection's face number is a whole number from 0", {
        data: { argument: "index", detail: String(index) },
      });
    }
    return new Font(bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes), undefined, index);
  }

  /** One of the standard 14 fonts. */
  static standard(name: Standard14): Font {
    return new Font(undefined, name);
  }
}

type ImageSource =
  | { readonly kind: "jpeg"; readonly bytes: Uint8Array }
  | { readonly kind: "png"; readonly bytes: Uint8Array }
  | { readonly kind: "rgba"; readonly width: number; readonly height: number; readonly bytes: Uint8Array };

function decode(instance: Instance<BaseGlue>, source: ImageSource): WasmImage {
  return instance.call((glue) => {
    switch (source.kind) {
      case "jpeg":
        return glue.WasmImage.jpeg(source.bytes);
      case "png":
        return glue.WasmImage.png(source.bytes);
      case "rgba":
        return glue.WasmImage.rgba(source.width, source.height, source.bytes);
    }
  });
}

/**
 * An image to draw: a JPEG (passed through), a PNG (alpha as a soft mask) or raw RGBA pixels.
 * The bytes are copied when the image is created. It is decoded once per WASM module, the first
 * time a builder of that module draws it (or `width`/`height` is read), so problems with the
 * data (`invalid-image`, `unsupported-image`, …) surface there. Written once per document however
 * often it is drawn. It keeps its bytes, so it outlives `reset()` and decodes again.
 */
export class Image {
  readonly #source: ImageSource;
  /** The decoded copy in each module, with the generation of the instance it lives in. */
  readonly #decoded = new Map<Instance<BaseGlue>, { readonly generation: number; readonly image: WasmImage }>();
  #freed = false;

  private constructor(source: ImageSource) {
    this.#source = source;
  }

  /** A JPEG. */
  static jpeg(bytes: Uint8Array): Image {
    return new Image({ kind: "jpeg", bytes: bytes.slice() });
  }

  /** A PNG. */
  static png(bytes: Uint8Array): Image {
    return new Image({ kind: "png", bytes: bytes.slice() });
  }

  /** 8-bit RGBA pixels, row by row (`image-data-length` at first use for a wrong length). */
  static rgba(width: number, height: number, pixels: Uint8Array): Image {
    return new Image({ kind: "rgba", width, height, bytes: pixels.slice() });
  }

  /** Width in pixels (decodes the image if no module has yet). @throws {@link LaminaPdfError} as `in`. */
  get width(): number {
    const [instance, image] = this.#any();
    return instance.call(() => image.width);
  }

  /** Height in pixels (decodes the image if no module has yet). @throws {@link LaminaPdfError} as `in`. */
  get height(): number {
    const [instance, image] = this.#any();
    return instance.call(() => image.height);
  }

  /** A decoded copy, in any module (the base module when none has one). */
  #any(): readonly [Instance<BaseGlue>, WasmImage] {
    for (const [instance, held] of this.#decoded) {
      instance.live();
      if (held.generation === instance.generation) return [instance, held.image];
    }
    const base = baseModule();
    return [base, this.in(base)];
  }

  /**
   * Frees the image's WASM memory now; without it, the garbage collector frees it later. A second
   * call does nothing, and any other call afterwards throws `disposed`. Prefer `using`.
   */
  free(): void {
    if (this.#freed) return;
    this.#freed = true;
    forget(this);
    for (const [instance, held] of this.#decoded) {
      if (instance.owns(held.generation)) held.image.free();
    }
    this.#decoded.clear();
  }

  /** Frees the image at the end of a `using` block (an alias of `free()`). */
  [Symbol.dispose](): void {
    this.free();
  }

  /**
   * @internal The image in `instance`, decoded there at first use (and again after a reset).
   * @throws {@link LaminaPdfError} `disposed`; `invalid-image`, `unsupported-image`,
   *   `image-too-large`, `image-data-length`.
   */
  in(instance: Instance<BaseGlue>): WasmImage {
    if (this.#freed) throw packageError("disposed", "the Image was used after free() or its using block");
    instance.live();
    const held = this.#decoded.get(instance);
    if (held !== undefined && held.generation === instance.generation) return held.image;
    const image = decode(instance, this.#source);
    if (this.#decoded.size === 0) watch(this, "Image");
    this.#decoded.set(instance, { generation: instance.generation, image });
    return image;
  }
}

disposable(Image);
