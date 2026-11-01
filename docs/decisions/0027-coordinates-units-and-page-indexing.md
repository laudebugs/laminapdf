# 0027 — Coordinates, units and page indexing

- Status: Accepted (owner, 2026-10-29)
- Date: 2026-10-29

## Context

PDF positions content in **user space**. Its origin is wherever the page's boxes put it (usually the bottom-left of the MediaBox), y points up, and one unit is 1/72 inch times the page's `/UserUnit` (ISO 32000-2 §8.3.2.3 *User space*; `/UserUnit` in Table 31, §7.7.3.3 *Page objects*). What a viewer shows is something else again:

- **The visible region is the CropBox.** It defaults to the MediaBox and is clipped to it (§14.11.2 *Page boundaries*; §14.11.2.2 *Display of page boundaries*).
- **`/Rotate`** turns the page clockwise "when displayed or printed", by a multiple of 90 (Table 31).
- **`/UserUnit`** (PDF 1.6) scales the unit, so a value of 10 makes a 1000-unit page 10,000/72 inch wide (§8.3.2.3, Table 31).

pdf-lib exposes raw user space, so its users write `y: height - 150` to place text near the top. It is the library's best-known footgun. Extraction vendors disagree with each other: Textract uses ratios from the top-left, Azure uses inches, Docling top-left points, OpenDataLoader bottom-left points ([report: coordinates must be an explicit, typed field](../research/libraries/shareable-libraries.md#coordinates-must-be-an-explicit-typed-field)). An LLM handed a bbox without its frame guesses.

What exists today: `laminapdf_model::Page` reads `media_box()`, `crop_box()` (intersected with the MediaBox, defaulting to it), the other boxes, `rotate() -> Rotation` (0/90/180/270; invalid values read as 0 with `invalid-rotate`) and `user_unit() -> f64` (1 when absent or invalid, with `invalid-entry`). Boxes come back as `laminapdf_model::Rect { x0, y0, x1, y1 }` in user space with normalised corners (§7.9.5 *Rectangles*). Pages are addressed by a 0-based `index` and a stable `PageId`. No public API takes or returns positions yet: B5a's builder and L3a's extraction will be the first.

## Decision

### 1. Two spaces, named in the types

| Space | Origin and axes | Unit | Used by |
| --- | --- | --- | --- |
| **Page space** (the default) | Top-left corner of the page's **effective CropBox as displayed** (after `/Rotate`); x right, **y down** | **Points**: 1 pt = 1/72 inch, physical (`/UserUnit` applied) | High-level drawing (B5a builder, B5c façade), annotations and forms at the façade, all extraction and render output (L3, C), CLI `--json`, MCP |
| **User space** (raw) | As the content stream sees it: §8.3.2.3, y up, the CTM at page level | User-space units (not scaled by `/UserUnit`) | Low-level APIs only: `laminapdf-content` (ADR 0023), COS/model views (`/MediaBox`, annotation `/Rect` as stored), content-stream editing |

Every type and function in user space says so in its name (`UserSpaceRect`, `UserSpacePoint`, `…_user_space`). Unprefixed geometry is page space. A conversion between the two is always explicit, through the page's frame (§3).

### 2. Units in the types

```rust
// Illustrative (B5a adds these to the crate that owns geometry; B5c re-exports them).
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Default, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Pt(f64);                       // 1/72 inch, physical
impl Pt {
    pub const fn new(pt: f64) -> Self;
    pub fn from_mm(mm: f64) -> Self;      // mm * 72 / 25.4
    pub fn from_in(inches: f64) -> Self;  // inches * 72
    pub const fn get(self) -> f64;
}
pub struct PointPt { pub x: Pt, pub y: Pt }                        // page space
pub struct RectPt  { pub x0: Pt, pub y0: Pt, pub x1: Pt, pub y1: Pt } // page space: x0 ≤ x1, y0 ≤ y1, (x0, y0) is top-left
impl RectPt { pub fn from_xywh(x: Pt, y: Pt, width: Pt, height: Pt) -> Self; pub fn width(&self) -> Pt; pub fn height(&self) -> Pt; }
pub struct UserSpacePoint { pub x: f64, pub y: f64 }
pub struct UserSpaceRect  { pub x0: f64, pub y0: f64, pub x1: f64, pub y1: f64 } // today's laminapdf_model::Rect
```

- **Concrete types, not `Rect<Space>`.** A space parameter would be a generic at the binding boundary, which [ADR 0028](0028-facade-rules-jobs-and-cancellation.md) §1 rules out. Two concrete types are as safe and bind directly. `laminapdf_model::Rect` becomes `UserSpaceRect` (B5a renames it; the name `Rect` alone is not kept).
- **f64** throughout, like the model. Bindings receive plain numbers.
- **TypeScript:** `type Pt = number & { readonly [ptBrand]: "pt" }`, with constructors `pt(n)`, `mm(n)` and `inch(n)` (`in` is a reserved word). Inputs take `Pt`. Outputs are typed `Pt`, so passing millimetres by mistake is a type error, and `mm(210)` is the documented way to write A4.
- **JSON:** numbers in points. A rect is the array `[x0, y0, x1, y1]`, i.e. `[left, top, right, bottom]` in page space. Drawing inputs in TS also accept `{ x, y, width, height }`, which the binding converts.

### 3. Frame: box, rotation and `/UserUnit`

- **Reference box: the effective CropBox**, exactly as `Page::crop_box()` computes it (`/CropBox` ∩ `/MediaBox`, else the MediaBox). It is what viewers show, so a bbox from extraction lands on what the user sees.
- **Rotation: output in the displayed (rotated) frame.** Page `width`/`height` are the displayed ones (swapped for 90 and 270). This matches viewers, renderers and every extraction vendor.
- **`/UserUnit` is applied:** page-space values are physical points, so a page with `/UserUnit 10` reports ten times its user-space size.
- **No usable box: US Letter.** If the effective CropBox is absent (no usable `/MediaBox` and no `/CropBox`) or has zero width or height, the frame uses the MediaBox if that has an area, else `[0 0 612 792]`. That is what pdf.js (`LETTER_SIZE_MEDIABOX`, [`src/core/document.js`](https://github.com/mozilla/pdf.js/blob/master/src/core/document.js)) and PDFium (`CPDF_Page::UpdateDimensions`, [`cpdf_page.cpp`](https://pdfium.googlesource.com/pdfium/+/refs/heads/main/core/fpdfapi/page/cpdf_page.cpp)) do. `/MediaBox` is required (Table 31), so the fallback raises `invalid-box` (key `MediaBox`) once per page. Strict mode makes it an error, as for any warning. `PageFrame` is therefore infallible, apart from the strict and `DataPending`/`Source` errors every page read has. In JSON, `coordinates.box` is `"default"` instead of `"CropBox"` when the Letter fallback was used.
- **Coordinates outside the box** are allowed (negative, or beyond `width`/`height`): content can sit outside the visible region. Extraction flags it (`offPage`, L3a). A rect is never clipped silently.

With CropBox `[llx lly urx ury]` and `u` = `/UserUnit`, a user-space point (x, y) maps to page space (X, Y) as follows:

| `/Rotate` | Displayed top-left is user-space corner | X | Y |
| --- | --- | --- | --- |
| 0 | (llx, ury) | (x − llx)·u | (ury − y)·u |
| 90 | (llx, lly) | (y − lly)·u | (x − llx)·u |
| 180 | (urx, lly) | (urx − x)·u | (y − lly)·u |
| 270 | (urx, ury) | (ury − y)·u | (urx − x)·u |

Worked example: a US Letter page, CropBox = MediaBox = `[0 0 612 792]`, `u` = 1, with a rect at user-space `[72 692 272 720]` (a heading box 1 inch from the left and top):

| Frame | Rect | Page size |
| --- | --- | --- |
| User space (raw) | `UserSpaceRect [72 692 272 720]` (y up) | 612 × 792 |
| Page space, `/Rotate 0` | `RectPt [72 72 272 100]` | 612 × 792 |
| Page space, `/Rotate 90` | `RectPt [692 72 720 272]` | 792 × 612 |

The frame is a value: `PageFrame { crop_box: UserSpaceRect, rotate: Rotation, user_unit: f64 }`, with `to_page(UserSpacePoint) -> PointPt` and `to_user_space(PointPt) -> UserSpacePoint`. Every API that converts names the frame it used. **Which API sees which frame:** high-level reads and writes use page space in the displayed frame, including drawing onto an existing rotated page (the builder maps through the frame, so "top-left" means what the user sees). Content-stream reading and editing (`laminapdf-content`, E-round content editing) use user space, unrotated and unscaled, because operators and the CTM live there. Both are always available, so the two needs don't conflict.

New pages made by the builder default to `/Rotate 0`, `/UserUnit` absent and a MediaBox at the origin, so page space is a y-flip of user space and nothing else.

### 4. The `coordinates` block and `bboxNorm`

Every JSON output that carries positions (extraction, `laminapdf info --json`, render metadata, MCP results) has a `coordinates` block on each page object, next to the displayed `width` and `height` in points:

```json
{ "index": 0, "number": 1, "width": 792, "height": 612,
  "coordinates": { "origin": "top-left", "unit": "pt", "box": "CropBox", "rotate": 90, "userUnit": 1 } }
```

`origin` and `unit` have one value each today. `box` is `"CropBox"`, or `"default"` when §3's Letter fallback was used. All three are written out anyway so a reader never has to know this ADR. `rotate` and `userUnit` are the values applied. An item may also carry **`bboxNorm`**: its bbox divided by the page's `width`/`height`, in [0, 1], for highlighting independent of the renderer's scale (`bbox_norm` in Rust). For an item partly outside the box, `bboxNorm` is the visible part, clamped to [0, 1]. An item entirely outside has none.

### 5. Text placement

- **Drawing: `y` is the baseline.** `draw_text` at (x, y) puts the glyph origin of the first glyph there. That is where PDF positions glyphs (glyph origins sit on the baseline, ISO 32000-2 §9.2.4 *Glyph positioning and metrics*), and it is what pdf-lib and Canvas's default `alphabetic` baseline do. With page space y-down, a larger `y` is lower on the page. A future text-box layout API (top-aligned frames) names its anchor explicitly (`anchor: "top"`), so the default never changes.
- **Extraction:** a text span carries both its `bbox` (top of the ascent to the bottom of the descent) and its `baseline` (y in page space). Readers never have to derive one from the other.

### 6. Page indexing

- **APIs take a 0-based `index`** (`u32` across bindings), or a `PageId` where the page's identity matters across edits ([ADR 0005](0005-document-model.md) §5).
- **Output page objects carry both** `index` (0-based) and `number` (= `index` + 1), plus `label` (the page label, §12.4.2) when the document defines one.
- **Human-facing text shows 1-based numbers:** messages, hints, CLI text output ("page 3"). JSON fields that refer to a page are named `pageIndex` (or `pageNumber`). A field named bare `page` holding a number is not used, so a reader is never left guessing the base. [ADR 0026](0026-error-and-diagnostic-contract.md)'s `location.pageIndex` follows this.
- **CLI page ranges are 1-based, as in qpdf** (`1-5`, `z` for the last page, `r1` counting from the end; ADR 0031 fixes the grammar), and are converted to indices at the edge.

## Consequences

- An agent or user can place, find and highlight content without knowing PDF's coordinate model, and every JSON output states its frame.
- `Pt` makes unit mistakes type errors in Rust and TS. JSON stays plain numbers.
- Every position API does one frame transform per page. Today `Pages` caches only the media and crop boxes. `Page::rotate()` and `Page::user_unit()` re-read the dictionary on every call, and every invalid value goes through the warning log again (deduplicated there by code and object, but read, checked and, in strict mode, failed again each time). So the `PageFrame` (box, rotation, unit, fallback) is computed **once per page and cached next to the boxes**, and its warnings are raised once. [B5a](../todos/done/B5a-lamina-build-core.md) owns this.
- Code working on content streams must convert explicitly. That is the point, but content-editing ops (E-rounds) will carry user-space geometry and need `PageFrame` to show it in the UI.
- A malformed or missing box, `/Rotate` or `/UserUnit` falls back to a default with a warning (`invalid-box`, `invalid-rotate`, `invalid-entry`; §3 for the boxes), so the frame is always defined.
- B5a renames `laminapdf_model::Rect` to `UserSpaceRect`. Nothing is published, so nothing breaks for users.

## Alternatives considered

- **Bottom-left user space everywhere** (pdf-lib, raw PDF). Spec-faithful, but it is the documented footgun, it shifts with the CropBox origin, and it ignores `/Rotate`, so extracted boxes don't match what is displayed. Kept for low-level APIs only.
- **Ratios of the page** (Textract). Independent of the renderer, but it loses physical size, and drawing in ratios is awkward. Offered as the extra `bboxNorm` instead.
- **Inches** (Azure) or **millimetres**. Fine units, but PDF tools, fonts and design tools all speak points. Constructors (`mm()`, `inch()`) cover input.
- **MediaBox as the frame.** Simpler, but bboxes would be offset from what viewers show whenever a CropBox exists (common in print-ready files).
- **Unrotated output** (user space orientation, top-left). Right for content editing, wrong for everything a user sees. Content editing gets user space explicitly instead.
- **`y` as the top of the line box.** Natural for CSS-minded users, but it depends on font metrics the caller may not know, and it differs from PDF and pdf-lib. Reserved for an explicit text-box API.
- **A generic `Rect<Space>`.** Safest in pure Rust, but not bindable (ADR 0028 §1). Concrete types give the same safety.
- **1-based page indices in the API.** Friendly in text, but off-by-one against every array and every other API. The API stays 0-based, and output adds `number`.
