// Where a page's images and annotations land in a tile, read from PDFium's own page objects, for
// the page appearance pass (@laminapdf/render's recolourTile). As laminapdf-render-pdfium does:
// images through forms (whose children's bounds are in the form's space), sorted by whether the
// page shows through them; PDFium's HasTransparency covers the drawing state, so images up to
// 1 MP are also decoded to look for transparent samples (an /SMask or /Mask of their own).

import type { Quad, RenderPage, TilePlan, ViewRotation } from "@laminapdf/render";
import { pageToTile } from "@laminapdf/render";
import { heap, type Pdfium, withScratch } from "./engine.js";

/** An affine matrix `[a, b, c, d, e, f]`. */
type Matrix = readonly [number, number, number, number, number, number];

const FPDF_PAGEOBJ_IMAGE = 3;
const FPDF_PAGEOBJ_FORM = 5;
const FPDF_ANNOT_FLAG_HIDDEN = 2;
const FPDFBITMAP_BGRA = 4;
/** Images up to this many pixels are decoded to look for transparent samples. */
const ALPHA_PROBE_PIXELS = 1 << 20;

/** `first`, then `second`. */
function then(first: Matrix, second: Matrix): Matrix {
  const [a, b, c, d, e, f] = first;
  const [a2, b2, c2, d2, e2, f2] = second;
  return [
    a * a2 + b * c2,
    a * b2 + b * d2,
    c * a2 + d * c2,
    c * b2 + d * d2,
    e * a2 + f * c2 + e2,
    e * b2 + f * d2 + f2,
  ];
}

function apply(m: Matrix, x: number, y: number): [number, number] {
  return [m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]];
}

/** Four floats PDFium writes, read right after the call. */
function floats(p: Pdfium, at: number, count: number): number[] {
  return Array.from(new Float32Array(heap(p).buffer, at, count));
}

/**
 * From PDFium's user space (its crop box, y up, `/Rotate` not applied) to the tile's pixels:
 * into the displayed page space `pageToTile` starts from, then `pageToTile`.
 */
export function userToTile(p: Pdfium, page: number, info: RenderPage, plan: TilePlan, rotation: ViewRotation): Matrix {
  const box = withScratch(p, 16, (at) => {
    const read = (get: typeof p.FPDFPage_GetCropBox) =>
      get(page, at, at + 4, at + 8, at + 12) ? floats(p, at, 4) : undefined;
    // left, bottom, right, top
    return read(p.FPDFPage_GetCropBox) ?? read(p.FPDFPage_GetMediaBox);
  }) ?? [0, 0, 0, 0];
  const [x0 = 0, y0 = 0, x1 = 0, y1 = 0] = box;
  const toPage: Matrix = {
    0: [1, 0, 0, -1, -x0, y1] as const,
    90: [0, 1, 1, 0, -y0, -x0] as const,
    180: [-1, 0, 0, 1, x1, -y0] as const,
    270: [0, -1, -1, 0, y1, x1] as const,
  }[info.rotate as 0 | 90 | 180 | 270];
  const [a = 1, b = 0, c = 0, d = 1, e = 0, f = 0] = pageToTile(plan, info, rotation);
  return then(toPage, [a, b, c, d, e, f]);
}

/** Each shown annotation's `/Rect` in tile pixels. */
export function annotationRects(p: Pdfium, page: number, toTile: Matrix): Quad[] {
  const out: Quad[] = [];
  const count = p.FPDFPage_GetAnnotCount(page);
  withScratch(p, 16, (at) => {
    for (let i = 0; i < count; i++) {
      const annot = p.FPDFPage_GetAnnot(page, i);
      if (annot === 0) continue;
      try {
        if (p.FPDFAnnot_GetFlags(annot) & FPDF_ANNOT_FLAG_HIDDEN) continue;
        if (!p.FPDFAnnot_GetRect(annot, at)) continue;
        // FS_RECTF: left, top, right, bottom.
        const [l = 0, t = 0, r = 0, b = 0] = floats(p, at, 4);
        out.push([apply(toTile, l, b), apply(toTile, r, b), apply(toTile, r, t), apply(toTile, l, t)]);
      } finally {
        p.FPDFPage_CloseAnnot(annot);
      }
    }
  });
  return out;
}

/** The page's images in tile pixels, sorted by whether the page shows through them. */
export interface PageImages {
  readonly opaque: Quad[];
  readonly seeThrough: Quad[];
}

/** Every image on `page`, forms included. */
export function pageImages(p: Pdfium, doc: number, page: number, toTile: Matrix): PageImages {
  const images: PageImages = { opaque: [], seeThrough: [] };
  withScratch(p, 32, (at) => {
    const visit = (object: number, m: Matrix): void => {
      const type = p.FPDFPageObj_GetType(object);
      if (type === FPDF_PAGEOBJ_IMAGE) addImage(p, { doc, page, object, at }, m, images);
      if (type !== FPDF_PAGEOBJ_FORM) return;
      // A form's children are in its own space: its matrix comes first.
      const inner = p.FPDFPageObj_GetMatrix(object, at) ? then(floats(p, at, 6) as unknown as Matrix, m) : m;
      const count = p.FPDFFormObj_CountObjects(object);
      for (let i = 0; i < count; i++) visit(p.FPDFFormObj_GetObject(object, i), inner);
    };
    const count = p.FPDFPage_CountObjects(page);
    for (let i = 0; i < count; i++) visit(p.FPDFPage_GetObject(page, i), toTile);
  });
  return images;
}

/** Adds an image object's quad under `m`, sorted by whether the page shows through it. */
function addImage(
  p: Pdfium,
  { doc, page, object, at }: { doc: number; page: number; object: number; at: number },
  m: Matrix,
  images: PageImages,
): void {
  if (!p.FPDFPageObj_GetRotatedBounds(object, at)) return;
  const [x1 = 0, y1 = 0, x2 = 0, y2 = 0, x3 = 0, y3 = 0, x4 = 0, y4 = 0] = floats(p, at, 8);
  const quad: Quad = [apply(m, x1, y1), apply(m, x2, y2), apply(m, x3, y3), apply(m, x4, y4)];
  const through = p.FPDFPageObj_HasTransparency(object) || hasAlpha(p, doc, page, object);
  (through ? images.seeThrough : images.opaque).push(quad);
}

/**
 * Whether an image's own mask leaves some samples transparent. Larger images are taken as
 * opaque: decoding them costs more than a tile, and they are nearly always photos.
 */
function hasAlpha(p: Pdfium, doc: number, page: number, object: number): boolean {
  const size = withScratch(p, 8, (at) => {
    if (!p.FPDFImageObj_GetImagePixelSize(object, at, at + 4)) return 0;
    const [w = 0, h = 0] = new Uint32Array(heap(p).buffer, at, 2);
    return w * h;
  });
  if (size === undefined || size === 0 || size > ALPHA_PROBE_PIXELS) return false;
  const bitmap = p.FPDFImageObj_GetRenderedBitmap(doc, page, object);
  if (bitmap === 0) return false;
  try {
    if (p.FPDFBitmap_GetFormat(bitmap) !== FPDFBITMAP_BGRA) return false;
    const [width, height, stride] = [
      p.FPDFBitmap_GetWidth(bitmap),
      p.FPDFBitmap_GetHeight(bitmap),
      p.FPDFBitmap_GetStride(bitmap),
    ];
    const bytes = new Uint8Array(heap(p).buffer, p.FPDFBitmap_GetBuffer(bitmap), stride * height);
    for (let y = 0; y < height; y++) {
      for (let x = 0, o = y * stride + 3; x < width; x++, o += 4) if ((bytes[o] ?? 255) < 255) return true;
    }
    return false;
  } finally {
    p.FPDFBitmap_Destroy(bitmap);
  }
}
