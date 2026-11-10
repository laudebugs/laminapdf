// Link previews' pictures (R07): one render of the destination region through the document worker,
// at visible priority, and the text of the lines it shows. The last few pictures are kept, so
// hovering a citation again draws at once; they are closed when pushed out, on a zoom or rotation
// change, when the appearance changes and when the document closes. A render aborted before its
// answer has its late bitmap closed by the worker client, so nothing paints after the pointer left.

import type { PageAppearance } from "@laminapdf/render";
import type { Target } from "@laminapdf/sdk";
import type { ViewerClient } from "../client.js";
import {
  type PreviewLine,
  type PreviewRegion,
  previewLines,
  previewRegion,
  previewRender,
  previewText,
} from "../layers/link-preview.js";
import type { PageSize, Rotation } from "../layout/page-layout.js";
import { appearanceKey } from "../protocol.js";

/** How many pictures are kept. */
export const PREVIEW_CACHE = 8;

/** A link's destination as a picture, for a preview card. */
export interface LinkPreviewPicture {
  /** The crop, drawn already turned as the view is. Owned by the viewer: draw it, don't close it. */
  readonly bitmap: ImageBitmap;
  /** The crop's size on screen, in CSS px. */
  readonly width: number;
  readonly height: number;
  /** `region`: around the destination point; `page`: the top of the page (R07). */
  readonly kind: PreviewRegion["kind"];
  /** The destination page (0-based) and the top of the crop in page space, for the card's title. */
  readonly page: number;
  readonly y: number;
  /** The text of the lines inside the crop (document text), cut at 300 characters; empty without text. */
  readonly text: string;
}

/** What the pictures need of their viewer. */
export interface PreviewHost {
  readonly client: Pick<ViewerClient, "render" | "pageText">;
  /** Page `page`'s displayed size in points. */
  size(page: number): PageSize | undefined;
  rotation(): Rotation;
  /** The reader's zoom, in percent. */
  percent(): number;
  appearance(): PageAppearance;
  /** Device pixels per CSS px. */
  dpr(): number;
}

interface Kept {
  readonly picture: LinkPreviewPicture;
}

/** The pictures of one viewer. */
export class LinkPreviews {
  readonly #host: PreviewHost;
  /** Insertion order is recency: the first key is the oldest. */
  readonly #kept = new Map<string, Kept>();
  /** Bumped by `clear`: a render that started before it drew at an old zoom or appearance. */
  #generation = 0;

  constructor(host: PreviewHost) {
    this.#host = host;
  }

  /** How many pictures are kept (never more than {@link PREVIEW_CACHE}). */
  get size(): number {
    return this.#kept.size;
  }

  /**
   * `target` as a picture. Rejects with `AbortError` when `signal` aborts (no bitmap is left
   * behind), and with a `RangeError` for a page the document doesn't have.
   */
  async picture(target: Target, signal: AbortSignal): Promise<LinkPreviewPicture> {
    const host = this.#host;
    const size = host.size(target.page);
    if (size === undefined) throw new RangeError(`no page ${target.page}`);
    const lines = await this.#lines(target.page, signal);
    const region = previewRegion(target, size, lines, host.percent());
    const rotation = host.rotation();
    const appearance = host.appearance();
    const render = previewRender(region, size, rotation, host.dpr());
    const { x, y, width, height } = render.clip;
    const key = [target.page, render.scale, x, y, width, height, rotation, appearanceKey(appearance)].join(":");
    const kept = this.#kept.get(key);
    if (kept !== undefined) {
      // Most recent last.
      this.#kept.delete(key);
      this.#kept.set(key, kept);
      return kept.picture;
    }
    const generation = this.#generation;
    const drawn = await host.client.render(
      { page: target.page, scale: render.scale, clip: render.clip, rotation, appearance },
      { priority: "visible", signal },
    );
    if (signal.aborted || generation !== this.#generation) {
      drawn.bitmap.close();
      throw signal.reason instanceof Error ? signal.reason : new DOMException("aborted", "AbortError");
    }
    const raced = this.#kept.get(key);
    if (raced !== undefined) {
      // The same picture drawn by a call that started meanwhile: keep one.
      drawn.bitmap.close();
      return raced.picture;
    }
    const picture: LinkPreviewPicture = {
      bitmap: drawn.bitmap,
      width: render.css.width,
      height: render.css.height,
      kind: region.kind,
      page: target.page,
      y: region.clip.y,
      text: lines === undefined ? "" : previewText(lines, region.clip),
    };
    this.#kept.set(key, { picture });
    for (const [old, { picture: gone }] of this.#kept) {
      if (this.#kept.size <= PREVIEW_CACHE) break;
      this.#kept.delete(old);
      gone.bitmap.close();
    }
    return picture;
  }

  /** Closes every kept picture; renders under way are dropped when they finish. */
  clear(): void {
    this.#generation++;
    for (const { picture } of this.#kept.values()) picture.bitmap.close();
    this.#kept.clear();
  }

  /** Page `page`'s text lines, or `undefined` without text (no module, an image-only page). */
  async #lines(page: number, signal: AbortSignal): Promise<PreviewLine[] | undefined> {
    try {
      const text = await this.#host.client.pageText(page, { signal, priority: "visible" });
      return previewLines(text.runs, text.quads);
    } catch (error) {
      if (signal.aborted) throw error;
      return undefined;
    }
  }
}
