// The address of one tile on the `lamina://` scheme (ADR 0008 §4), the TypeScript side of
// src-tauri/src/engine/tiles.rs's parser: `tile/{doc}/{page}?s=…&r=…&a=…&x=…&y=…&w=…&h=…&p=…&t=…`,
// with `fmt=webp` for the WebP transport and without it for raw RGBA. The base comes from
// Tauri's `convertFileSrc("", "lamina")`, which gives `lamina://localhost/` or, on Windows,
// `http://lamina.localhost/`.

import type { PageAppearance, PixelRect, Rgba8, ViewRotation } from "@laminapdf/render";

/** How tiles travel: lossless WebP (the default), or raw RGBA (the fallback, for diagnosis). */
export type TileTransport = "webp" | "raw";

/** One tile as the engine draws it. */
export interface TileAddress {
  readonly doc: number;
  readonly page: number;
  /** Pixels per point, device pixel ratio applied. */
  readonly scale: number;
  readonly rotation: ViewRotation;
  readonly appearance: PageAppearance | undefined;
  readonly clip: PixelRect;
  /** Whether it is on screen (`visible`) or drawn ahead of a scroll. */
  readonly visible: boolean;
  /** This tile's ticket, which `cancel_tiles` names to drop it. */
  readonly ticket: number;
  readonly transport: TileTransport;
}

/** Eight hex digits: straight RGBA. */
function hex({ r, g, b, a }: Rgba8): string {
  return [r, g, b, a].map((channel) => channel.toString(16).padStart(2, "0")).join("");
}

/** The `a` parameter: `original`, `sepia`, `dark` or `hc-{ink}-{paper}`. */
export function appearanceParam(appearance: PageAppearance | undefined): string {
  if (appearance === undefined || typeof appearance === "string") return appearance ?? "original";
  return `hc-${hex(appearance.ink)}-${hex(appearance.paper)}`;
}

/**
 * The tile's address under `base` (`lamina://localhost/` or `http://lamina.localhost/`).
 * The scale goes as the single-precision value the shared planner sized the page with
 * (`Math.fround`), so the engine reads the same number and its page is the same size in pixels.
 */
export function tileUrl(base: string, tile: TileAddress): string {
  const { clip } = tile;
  const params = new URLSearchParams({
    s: String(Math.fround(tile.scale)),
    r: String(tile.rotation),
    a: appearanceParam(tile.appearance),
    x: String(clip.x),
    y: String(clip.y),
    w: String(clip.width),
    h: String(clip.height),
    p: tile.visible ? "v" : "p",
    t: String(tile.ticket),
  });
  if (tile.transport === "webp") params.set("fmt", "webp");
  const root = base.endsWith("/") ? base : `${base}/`;
  return `${root}tile/${tile.doc}/${tile.page}?${params}`;
}
