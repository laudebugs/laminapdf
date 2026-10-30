// Fonts and images: descriptions that any builder can use. A document is built in one WASM
// instance (base or text, ADR 0016 §4), so the instance-side objects are made per builder (fonts)
// or per module (images) when first used.
import type { WasmImage } from "@lamina/wasm/base";
import { guard } from "./errors.js";
import { type BaseGlue, baseModule, type TextGlue } from "./init.js";

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

  private constructor(bytes: Uint8Array | undefined, standard: Standard14 | undefined) {
    this.bytes = bytes;
    this.standard = standard;
  }

  /**
   * A font from its bytes (TrueType or OpenType/CFF). The bytes are checked when a builder first
   * uses the font (`invalid-font`, `font-not-embeddable`).
   */
  static fromBytes(bytes: Uint8Array | ArrayBuffer): Font {
    return new Font(bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes), undefined);
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

function decode(glue: BaseGlue | TextGlue, source: ImageSource): WasmImage {
  return guard(() => {
    switch (source.kind) {
      case "jpeg":
        return glue.WasmImage.jpeg(source.bytes);
      case "png":
        return glue.WasmImage.png(source.bytes);
      case "rgba":
        return glue.WasmImage.rgba(source.width, source.height, source.bytes);
    }
  }) as WasmImage;
}

/**
 * An image to draw: a JPEG (passed through), a PNG (alpha as a soft mask) or raw RGBA pixels.
 * Checked when created; written once per document however often it is drawn.
 */
export class Image {
  readonly #source: ImageSource;
  readonly #decoded = new Map<BaseGlue | TextGlue, WasmImage>();
  /** Width in pixels. */
  readonly width: number;
  /** Height in pixels. */
  readonly height: number;

  private constructor(source: ImageSource) {
    this.#source = source;
    const glue = baseModule();
    const image = decode(glue, source);
    this.#decoded.set(glue, image);
    this.width = image.width;
    this.height = image.height;
  }

  /** A JPEG. @throws {@link LaminaError} `invalid-image`, `unsupported-image`. */
  static jpeg(bytes: Uint8Array): Image {
    return new Image({ kind: "jpeg", bytes });
  }

  /** A PNG. @throws {@link LaminaError} `invalid-image`, `unsupported-image`, `image-too-large`. */
  static png(bytes: Uint8Array): Image {
    return new Image({ kind: "png", bytes });
  }

  /** 8-bit RGBA pixels, row by row. @throws {@link LaminaError} `invalid-image` for a wrong length. */
  static rgba(width: number, height: number, pixels: Uint8Array): Image {
    return new Image({ kind: "rgba", width, height, bytes: pixels });
  }

  /** Frees the image's WASM memory now (otherwise when it is garbage-collected). */
  free(): void {
    for (const image of this.#decoded.values()) image.free();
    this.#decoded.clear();
  }

  /** @internal The image in `glue`'s instance. */
  in(glue: BaseGlue | TextGlue): WasmImage {
    let image = this.#decoded.get(glue);
    if (image === undefined) {
      image = decode(glue, this.#source);
      this.#decoded.set(glue, image);
    }
    return image;
  }
}
