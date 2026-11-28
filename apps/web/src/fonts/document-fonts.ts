// The fonts of one open document: each font's bytes are added to the document once, as a blob the
// core names, and an added text names its fonts by the blob IDs (standard fonts by their names).
// A new document, which the viewer opens after a save or a reload, starts with none.

import { fallbackChain } from "./fallback.ts";
import type { FontRegistry } from "./registry.ts";

/** The fonts for one added text, as `AddText`'s `fonts` names them. */
export interface TextFonts {
  /** The font, then its fallbacks: `helvetica`, `times`, `courier` or a blob ID. */
  readonly fonts: readonly string[];
  /** The characters no font in the registry draws: the op would be refused for them. */
  readonly missing: readonly string[];
}

const STANDARD: readonly string[] = ["helvetica", "times", "courier"];

/** The fonts one document holds. */
export class DocumentFonts {
  readonly #registry: FontRegistry;
  readonly #addBlob: (bytes: Uint8Array) => Promise<string>;
  readonly #blobs = new Map<string, Promise<string>>();
  readonly #ids = new Map<string, string>();

  /** @param addBlob adds bytes to the document and returns the ID the core gives them. */
  constructor(registry: FontRegistry, addBlob: (bytes: Uint8Array) => Promise<string>) {
    this.#registry = registry;
    this.#addBlob = addBlob;
  }

  /** The name font `id` goes by in an op: its own for a standard font, else its blob's ID. */
  async name(id: string): Promise<string> {
    const bytes = await this.#registry.bytes(id);
    if (bytes === undefined) return id;
    let blob = this.#blobs.get(id);
    if (!blob) {
      blob = this.#addBlob(bytes);
      this.#blobs.set(id, blob);
      blob.then(
        (name) => this.#ids.set(name, id),
        () => this.#blobs.delete(id),
      );
    }
    return blob;
  }

  /** The registry font a name in an op stands for: a standard font's own, or a font this session added as a blob. */
  idOf(name: string): string | undefined {
    return this.#ids.has(name) ? this.#ids.get(name) : STANDARD.includes(name) ? name : undefined;
  }

  /** The fonts for `text` set in `primary`: the chain, fetched and named. */
  async forText(primary: string, text: string): Promise<TextFonts> {
    const plan = fallbackChain(this.#registry.list(), primary, text);
    return { fonts: await Promise.all(plan.fonts.map((id) => this.name(id))), missing: plan.missing };
  }
}
