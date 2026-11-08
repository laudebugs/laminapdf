// Heron's outputs as regions: its 17 labels mapped onto the extraction's, low scores dropped,
// near-duplicate boxes merged, boxes moved from pixels into page points. A port of
// crates/laminapdf-layout-onnx/src/post.rs.
import type { LayoutRegion, RegionLabel } from "@laminapdf/sdk";

/** Heron's labels in its `config.json`'s `id2label` order, as the extraction names them. */
const LABELS: readonly RegionLabel[] = [
  "caption",
  "footnote",
  "formula",
  "list-item",
  "page-footer",
  "page-header",
  "picture",
  "section-header",
  "table",
  "text",
  "title",
];

/** Heron's label `id` as the extraction names it; document_index, code, checkboxes, forms and key–value regions are `other`. */
export function label(id: number): RegionLabel {
  return LABELS[id] ?? "other";
}

/** A box overlapping a kept one by this much (intersection over union) is the same region read twice. */
const SAME_BOX = 0.9;

type Box = readonly [number, number, number, number];

function iou(a: Box, b: Box): number {
  const w = Math.max(Math.min(a[2], b[2]) - Math.max(a[0], b[0]), 0);
  const h = Math.max(Math.min(a[3], b[3]) - Math.max(a[1], b[1]), 0);
  const area = (r: Box) => Math.max(r[2] - r[0], 0) * Math.max(r[3] - r[1], 0);
  const union = area(a) + area(b) - w * h;
  return union > 0 ? (w * h) / union : 0;
}

/** The model's outputs for one image: 300 candidates, boxes `x0, y0, x1, y1` in the image's pixels. */
export interface Detections {
  readonly labels: ArrayLike<number | bigint>;
  readonly boxes: ArrayLike<number>;
  readonly scores: ArrayLike<number>;
}

/** The page the image shows: its size in pixels and in points. */
export interface PageSize {
  readonly widthPx: number;
  readonly heightPx: number;
  readonly widthPt: number;
  readonly heightPt: number;
}

/** The regions scoring at least `threshold`, best first, each box kept once, in page points. */
export function regions(d: Detections, page: PageSize, threshold: number): LayoutRegion[] {
  const found: { score: number; id: number; box: Box }[] = [];
  for (let i = 0; i < d.scores.length; i++) {
    const score = d.scores[i] ?? 0;
    if (!(score >= threshold)) continue;
    const box: Box = [d.boxes[i * 4] ?? 0, d.boxes[i * 4 + 1] ?? 0, d.boxes[i * 4 + 2] ?? 0, d.boxes[i * 4 + 3] ?? 0];
    found.push({ score, id: Number(d.labels[i] ?? -1), box });
  }
  found.sort((a, b) => b.score - a.score);
  const kept: typeof found = [];
  for (const candidate of found) {
    if (kept.every((k) => iou(k.box, candidate.box) < SAME_BOX)) kept.push(candidate);
  }
  const sx = page.widthPt / Math.max(page.widthPx, 1);
  const sy = page.heightPt / Math.max(page.heightPx, 1);
  return kept.map(({ score, id, box: [x0, y0, x1, y1] }) => ({
    label: label(id),
    bbox: [x0 * sx, y0 * sy, (x1 - x0) * sx, (y1 - y0) * sy],
    score,
  }));
}
