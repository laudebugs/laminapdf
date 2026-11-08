// The shapes @laminapdf/sdk's extractJob takes, declared here so the package needs neither the
// SDK nor its WebAssembly build to typecheck. tools/heron-web/ort-types.ts checks they still fit
// the SDK's own types (the binding's, from crates/laminapdf-wasm/src/extract.rs).

/** What a region is, as the extraction names it. */
export type RegionLabel =
  | "text"
  | "title"
  | "section-header"
  | "list-item"
  | "table"
  | "picture"
  | "caption"
  | "footnote"
  | "formula"
  | "page-header"
  | "page-footer"
  | "other";

/** A region Heron found: `bbox` is `[x, y, width, height]` in top-left points. */
export interface LayoutRegion {
  readonly label: RegionLabel;
  readonly bbox: [number, number, number, number];
  readonly score: number;
}

/** A model as the output's `generator.models` names it. */
export interface ModelInfo {
  readonly name: string;
  readonly version: string;
  readonly sha256: string;
  readonly licence: string;
}
