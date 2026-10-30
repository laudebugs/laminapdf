# 02 — The Imaging Model: Content Streams, Graphics & Color

> **Scope.** How PDF describes *what appears on a page*: the painter's model, content-stream syntax and operators, coordinate systems and transforms, the graphics state, paths, color, images, form XObjects, patterns/shadings/functions, transparency, optional content, and how real renderers rasterize all of it. File structure (objects, streams, filters, xref) is in **[chapter 01](01-file-structure.md)**; fonts, text state and glyph positioning are in **[chapter 03](03-text-and-fonts.md)**.
>
> **Primary reference:** ISO 32000-2:2020 (PDF 2.0), clauses 8 (Graphics), 10 (Rendering), 11 (Transparency) and 8.11 (Optional content). Clause numbers below refer to ISO 32000-2 unless noted. ISO 32000-1:2008 (PDF 1.7, freely available from Adobe) has almost the same structure and is a fine companion. This chapter has been checked against the ISO 32000-1 text (see the verification log at the end); where 1.7 table/clause numbers were confirmed they are given explicitly as "ISO 32000-1 …".

---

## 1. The model in one page

### 1.1 PostScript heritage

PDF's imaging model is the PostScript Level 2/3 imaging model **with the programming language removed**. A PostScript page description is a program: it has procedures, loops, variables, conditionals, and you only know what it draws by running it. In 1991 John Warnock's "Camelot" memo proposed taking the *output* of a PostScript interpreter — the sequence of painting operations after all the control flow is gone — and storing that. That is a PDF content stream: a flat list of drawing operators with literal operands.

What survived from PostScript:

- Postfix syntax (`operands operator`), the same names for most path operators (`m`/`moveto`, `l`/`lineto`, `c`/`curveto`), and the graphics-state stack (`q`/`Q` ≈ `gsave`/`grestore`).
- The **current transformation matrix** (CTM), user space vs device space, 1/72-inch default units.
- Cubic Bézier paths, nonzero and even-odd fill rules, stroke parameters, clipping by paths.
- Color spaces (Device, CIE-based, Indexed, Separation, DeviceN, Pattern), shading types 1–7, functions types 0/2/3 and a PostScript-subset calculator (type 4).
- Images with `Decode` arrays, stencil masks, halftones, transfer functions, black generation/UCR.

What PDF added: transparency (1.4), optional content (1.5), marked content and structure, soft masks, JPEG 2000/JBIG2 filters, ICC profiles as first-class color spaces, and in PDF 2.0 black-point compensation, page-level output intents, and lots of clarifications.

What PDF removed: arbitrary computation. The only "code" left is the Type 4 function (a tiny, side-effect-free calculator, §9.4 below). PDF 2.0 deleted PostScript XObjects entirely (in PDF 1.7 they still exist — `/Subtype /PS`, ISO 32000-1 §8.8.2 — but are print-only, ignored on screen, and discouraged).

### 1.2 Device independence

A page is described in an abstract coordinate space (user space) with abstract colors (ideally colorimetrically specified). The renderer maps both onto the device: geometry via the CTM and device resolution, color via color management. The same file can be rendered on a 96 dpi screen, a 2400 dpi imagesetter, or a CMYK press, and *device-dependent* graphics-state parameters (halftones, transfer functions, black generation, flatness) are segregated so that a screen viewer can ignore them.

### 1.3 The painter's model

Every painting operator places paint on the page **in order**, and later paint covers earlier paint. In the opaque model (pre-1.4) covering is total: there is no blending, and the page is a sequence of *opaque* layers. Each painting operation applies:

```
  paint := (shape) ∩ (current clipping path), filled with (current color / pattern / image samples)
  page  := page with paint placed on top
```

Notable consequences:

- There is no "erase". To make something disappear you paint over it (usually with white), which is why redaction must remove content, not cover it.
- The page starts as an infinitely thin, white (in practice "paper") backdrop. PDF has no concept of a page background color.
- Order is semantics. A writer that reorders operators changes the picture.

### 1.4 How transparency extends it

PDF 1.4 generalized "place on top" to **composite on top**: every object has a *shape* and an *opacity* in [0,1] and a *blend mode*. With opacity 1, shape 1 and blend mode `Normal`, compositing reduces exactly to the opaque painter's model — that's the backward-compatibility guarantee. Objects can be grouped (transparency groups), and groups are composited as a unit. Details in §10.

---

## 2. Content streams

A **content stream** is a stream object (chapter 01) whose decoded bytes are a sequence of tokens using the PDF lexical conventions: numbers, names, strings, arrays, dictionaries — followed by an operator keyword (a bare token that isn't one of the other types). Operands precede the operator: *postfix*. There is no stack beyond a single instruction; operands do not carry over from one operator to the next.

```
0.2 0.4 0.8 rg      % three operands, operator rg
72 600 200 120 re   % four operands, operator re
f                   % no operands
```

Key facts for implementers:

- A page's `/Contents` may be a single stream or an **array of streams**; they're concatenated as if one stream, with a token boundary between them (split points can fall *between* any two tokens, not inside one — but a writer should never split inside `BT…ET` or inline images for robustness).
- Operators reference named **resources** (`/F1 Tf`, `/Im1 Do`, `/GS1 gs`, `/Sh1 sh`, `/P1 scn`, `/CS0 cs`, `/MC0 BDC`) that are resolved through the `/Resources` dictionary of the page, form XObject, pattern, or Type 3 font that owns the stream. Resource categories: `ExtGState`, `ColorSpace`, `Pattern`, `Shading`, `XObject`, `Font`, `ProcSet` (obsolete), `Properties`.
- Comments (`%` to EOL) are permitted and ignored.
- **Real numbers cannot use exponent notation.** `1e-5` is not a PDF number. Writers must format reals in plain decimal. Five significant decimals is far beyond any visible precision at 1/72 inch.
- Unknown operators are an error *unless* inside `BX … EX` (compatibility section), where viewers must ignore them.
- Inline images (`BI … ID <binary> EI`) are the one place where raw binary sits in a content stream, and they're the main parsing hazard (§7.3).

### 2.1 The operator catalogue

PDF defines **73 operators** (count verified against ISO 32000-1 Table 51 "Operator Categories" and the alphabetical list in Annex A, Table A.1; PDF 2.0 adds no new content-stream operators as far as we know). Table (adapted from the operator-categories table — ISO 32000-1 Table 51; cited earlier as ISO 32000-2 Table 50, number unverified — with operand signatures):

| Category | Operator | Operands | Meaning |
|---|---|---|---|
| **General graphics state** | `w` | *lineWidth* | Set line width |
| | `J` | *lineCap* | 0 butt, 1 round, 2 projecting square |
| | `j` | *lineJoin* | 0 miter, 1 round, 2 bevel |
| | `M` | *miterLimit* | Set miter limit |
| | `d` | *dashArray dashPhase* | Set dash pattern, e.g. `[3 2] 0 d` |
| | `ri` | *intent* | Rendering intent name |
| | `i` | *flatness* | Flatness tolerance 0–100 (device pixels) |
| | `gs` | *dictName* | Apply an ExtGState resource |
| **Special graphics state** | `q` | — | Push copy of graphics state |
| | `Q` | — | Pop graphics state |
| | `cm` | *a b c d e f* | CTM := M × CTM |
| **Path construction** | `m` | *x y* | Begin new subpath |
| | `l` | *x y* | Straight segment |
| | `c` | *x1 y1 x2 y2 x3 y3* | Cubic Bézier |
| | `v` | *x2 y2 x3 y3* | Cubic, first control point = current point |
| | `y` | *x1 y1 x3 y3* | Cubic, second control point = end point |
| | `h` | — | Close subpath (straight line to start) |
| | `re` | *x y width height* | Rectangle as closed subpath |
| **Path painting** | `S` | — | Stroke |
| | `s` | — | Close and stroke (`h S`) |
| | `f` | — | Fill, nonzero winding |
| | `F` | — | Same as `f` (obsolete; don't write) |
| | `f*` | — | Fill, even-odd |
| | `B` | — | Fill (nonzero) then stroke |
| | `B*` | — | Fill (even-odd) then stroke |
| | `b` | — | Close, fill (nonzero), stroke |
| | `b*` | — | Close, fill (even-odd), stroke |
| | `n` | — | End path, no paint (used after `W`) |
| **Clipping** | `W` | — | Intersect clip with path (nonzero), effective after next painting op |
| | `W*` | — | Same, even-odd |
| **Text objects** | `BT` / `ET` | — | Begin/end text object (ch. 03) |
| **Text state** | `Tc Tw Tz TL Tf Tr Ts` | various | Char spacing, word spacing, horizontal scale, leading, font+size, render mode, rise (ch. 03) |
| **Text positioning** | `Td TD Tm T*` | various | Move/set text matrix (ch. 03) |
| **Text showing** | `Tj TJ ' "` | strings/arrays | Show glyphs (ch. 03) |
| **Type 3 fonts** | `d0` | *wx wy* | Glyph width; glyph may set color |
| | `d1` | *wx wy llx lly urx ury* | Width + bbox; glyph is a stencil (color ops forbidden) |
| **Color** | `CS` / `cs` | *name* | Set stroking / nonstroking color space |
| | `SC` / `sc` | *c1…cn* | Set color in Device, CalGray/CalRGB/Lab, or Indexed spaces only (not ICCBased, Separation, DeviceN or Pattern — ISO 32000-1 Table 74) |
| | `SCN` / `scn` | *c1…cn [name]* | Same as `SC`/`sc` plus ICCBased, Separation, DeviceN and Pattern (name last) |
| | `G` / `g` | *gray* | DeviceGray + color |
| | `RG` / `rg` | *r g b* | DeviceRGB + color |
| | `K` / `k` | *c m y k* | DeviceCMYK + color |
| **Shading** | `sh` | *name* | Paint shading over current clip |
| **Inline images** | `BI` `ID` `EI` | dict pairs / data | Inline image |
| **XObjects** | `Do` | *name* | Paint image/form XObject |
| **Marked content** | `MP` | *tag* | Marked-content point |
| | `DP` | *tag properties* | Point with properties |
| | `BMC` | *tag* | Begin marked-content sequence |
| | `BDC` | *tag properties* | Begin sequence with properties (e.g. `/OC /oc1 BDC`, `/P <</MCID 0>> BDC`) |
| | `EMC` | — | End sequence |
| **Compatibility** | `BX` / `EX` | — | Begin/end section where unknown operators are ignored |

Convention: uppercase = stroking, lowercase = nonstroking (`RG`/`rg`, `CS`/`cs`). Capital and lowercase *path painting* letters are a different convention (`S` stroke vs `s` close+stroke).

### 2.2 Where operators are legal

The spec defines a small state machine (Figure 9, "Graphics objects" — ISO 32000-1 §8.2, figure in Annex L). Writers must respect it; readers should be lenient.

```
                    ┌────────── general gstate / color / q Q cm / marked content ───────┐
                    ▼                                                                     │
           ┌──── PAGE LEVEL ────┐                                                         │
   m, re ──┤                    ├── BT ──► TEXT OBJECT ── ET ──► page level               │
           │                    │          (Tj TJ ' " Td Tm ... + color, gs, w...)        │
           ▼                    ├── BI ──► INLINE IMAGE ── ID data EI ──► page level      │
     PATH OBJECT                ├── sh, Do ──► (painted immediately) ─────────────────────┘
     (m l c v y h re)
           │ W / W* (optional)
           ▼
   CLIPPING PATH ── S s f f* B B* b b* n ──► page level
```

Rules that bite:

- `q`, `Q`, `cm` are **not allowed inside a path object** (between `m` and the painting operator), nor inside `BT…ET` (the text-object state in Figure 9 allows general graphics state, color, text and marked-content operators but not the special graphics state `q`/`Q`/`cm` — already so in ISO 32000-1 §8.2, not new in 2.0; real files do it anyway).
- `W`/`W*` must come immediately before a painting operator; the clip takes effect *after* the painting. Idiom: `… re W n`.
- `q`/`Q` must balance within each content stream (a page's array of streams counts as one). Unbalanced `Q` is common in the wild; readers clamp.
- Marked-content `BDC/EMC` must nest properly and must not straddle `BT/ET` boundaries asymmetrically.
- Historic limit: `q`/`Q` nesting depth 28 (ISO 32000-1 Annex C, Table C.1 — confirmed; the spec's own note says it "is not a limit as such" but stems from mapping `q`/`Q` to PostScript `gsave`/`grestore` when generating PostScript). Not a PDF 2.0 rule, but stay well under it.

---

## 3. Coordinate systems and transformations

### 3.1 The spaces

```
 glyph space ──FontMatrix──► text space ──Trm(Tm, Tfs, Th, Trise)──┐
 image space ──(implicit: w×h → unit square)──────────────────────┤
 form space ──/Matrix──────────────────────────────────────────────┤
                                                                   ▼
                                                   USER SPACE ──CTM──► DEVICE SPACE
                                                       ▲
 pattern space ──Pattern /Matrix──► DEFAULT space of the parent (page or form)
```

| Space | Defined by | Notes |
|---|---|---|
| **Device space** | The output device | Pixels/dots; origin and orientation device-specific. Never appears in the file. |
| **Default user space** | Page | Origin at bottom-left of the MediaBox's coordinate system (not necessarily at the MediaBox corner — MediaBox can be `[-50 -50 562 742]`). x right, **y up**. 1 unit = 1/72 inch × `UserUnit`. |
| **User space** | Default user space × CTM changes | What operands of path/painting ops are in. |
| **Text space** | Text matrix `Tm` | ch. 03; text rendering matrix `Trm = [Tfs·Th 0 0 Tfs 0 Trise] × Tm × CTM`. |
| **Glyph space** | Font | 1000 units/em for most fonts; Type 3 uses its `FontMatrix`. |
| **Image space** | Image | Every image maps to the 1×1 unit square of user space. |
| **Form space** | Form XObject `/Matrix` | Concatenated onto CTM on `Do`. |
| **Pattern space** | Pattern `/Matrix` | Maps to the default coordinate space of the pattern's **parent stream** (page or form), *not* the current CTM. |

**UserUnit** (PDF 1.6, page dictionary): a positive number, default 1.0, scaling the default unit to `UserUnit/72` inch. It exists because the recommended maximum page size is 14,400 × 14,400 units (200 in at 1/72 in; ISO 32000-1 Annex C.2, a "should" constraint that Acrobat enforces); with `UserUnit 10` a page can be 2000 in. Support is spotty in non-Adobe tools — a writer should avoid it unless needed.

Other page-level geometry that a renderer must apply *before* content: `/MediaBox` and `/CropBox` (visible region), and `/Rotate` (multiple of 90°, clockwise when displayed). Those are covered in chapter 01; just remember that "default user space" is unrotated, and `/Rotate` happens in the display transform.

### 3.2 The matrix math

A transform is six numbers `[a b c d e f]` representing the 3×3 affine matrix

```
        ┌ a  b  0 ┐
   M =  │ c  d  0 │
        └ e  f  1 ┘
```

PDF uses **row vectors**: `[x' y' 1] = [x y 1] × M`, i.e.

```
   x' = a·x + c·y + e
   y' = b·x + d·y + f
```

Common forms:

| Transform | `[a b c d e f]` |
|---|---|
| Translate (tx, ty) | `1 0 0 1 tx ty` |
| Scale (sx, sy) | `sx 0 0 sy 0 0` |
| Rotate θ counter-clockwise | `cosθ sinθ −sinθ cosθ 0 0` |
| Skew x by α, y by β | `1 tanα tanβ 1 0 0` |
| Flip vertically about y=H/2 | `1 0 0 −1 0 H` |

**Concatenation order (the #1 bug source).** The `cm` operator does

```
   CTM_new = M × CTM_old
```

— *pre*-multiplication. With row vectors, a point is transformed by `M` first and then by the old CTM. So in a sequence

```
   M1 cm  M2 cm  M3 cm   … draw p …
```

device point = `p × M3 × M2 × M1 × CTM0`. The **last** `cm` written is the **first** applied to the geometry. This matches the "move the coordinate system" mental model: each `cm` redefines user space relative to the current one. Practical rule: *write transforms outermost-first*, exactly like nested `ctx.translate(); ctx.rotate()` calls in Canvas/Skia/Cairo, which use the same convention.

Matrix product with this layout (useful for collapsing chains into one `cm`):

```
 [a b c d e f] × [a' b' c' d' e' f'] =
   [ a·a' + b·c',   a·b' + b·d',
     c·a' + d·c',   c·b' + d·d',
     e·a' + f·c' + e',   e·b' + f·d' + f' ]
```

Inverse (needed for hit-testing, pattern phase, and "where is this device pixel in user space"): with `det = a·d − b·c`,

```
 M⁻¹ = [ d/det, −b/det, −c/det, a/det, (c·f − d·e)/det, (b·e − a·f)/det ]
```

A singular CTM (det = 0) is legal in a file but makes painting degenerate; renderers typically skip the operation. A writer should never emit one.

### 3.3 Worked example: rotation about a point

Rotate a shape by θ about (px, py): translate the pivot to the origin, rotate, translate back. In PDF writing order (outermost first):

```
1 0 0 1 px py cm            % 3) move origin to pivot
cosθ sinθ −sinθ cosθ 0 0 cm % 2) rotate
1 0 0 1 −px −py cm          % 1) shift so pivot is at origin
… draw shape in its normal coordinates …
```

Collapsed into one matrix:

```
 [ cosθ, sinθ, −sinθ, cosθ,  px − px·cosθ + py·sinθ,  py − px·sinθ − py·cosθ ]
```

Check: x' = cosθ(x−px) − sinθ(y−py) + px ✓. The worked example in §13 uses the simpler variant: translate the origin to the centre and draw the square centred on (0,0).

### 3.4 Worked example: top-left origin (y-down)

Most UI/graphics APIs and HTML/CSS use a top-left origin with y down. For a page of height H:

```
1 0 0 -1 0 H cm      % y' = H − y
```

Now `(0,0)` is top-left and `100 50 200 30 re f` draws a box 50 units below the top. Pitfalls:

1. **Text is mirrored.** Glyphs are defined y-up. Counter-flip in the text matrix: `BT 1 0 0 -1 x y Tm (Hello) Tj ET`, where (x, y) is the baseline in y-down coordinates.
2. **Images are upside down.** An image maps to the unit square with its first row at the top in *user* space y-up terms (§7.1). Under a flip, draw with `w 0 0 -h x y+h cm /Im Do` (negative height, baseline at the bottom edge).
3. **Arc and rotation direction invert.** A positive θ in `cosθ sinθ −sinθ cosθ` becomes clockwise visually.
4. **Fill-rule orientation flips.** `re` winds counter-clockwise in user space; in a flipped space it winds clockwise on the page. Irrelevant for a single shape, relevant when you rely on opposite windings to cut holes with the nonzero rule — generate holes by actually reversing direction, not by assuming `re`'s orientation.
5. **Annotations, links, form fields, and `/Rect` values are always in default (unflipped) user space.** Your API must convert.

Recommendation for lamina: expose a y-down API if you want, but keep one flip `cm` at the start of each page (or compute in y-up internally) and handle text/images explicitly; don't sprinkle flips per object.

---

## 4. The graphics state

The **graphics state** is the implicit context for every painting operation. `q` pushes a full copy, `Q` restores it. Everything — CTM, clip, colors, line style, font, alpha, soft mask — is saved; the *path* under construction is **not** part of the graphics state (and must be empty at `q`/`Q`).

### 4.1 Parameters (ISO 32000-2 Tables 51–52; ISO 32000-1 Tables 52–53)

**Device-independent:**

| Parameter | Set by | Initial value |
|---|---|---|
| CTM | `cm` | default user space → device |
| clipping path | `W`, `W*` | whole imageable area |
| color space (stroke/fill) | `CS`/`cs`, `G`/`RG`/`K` etc. | DeviceGray |
| color (stroke/fill) | `SC/SCN/sc/scn/G/g/RG/rg/K/k` | black |
| text state (Tc Tw Th Tl Tf Tfs Tmode Trise Tk) | text operators, `gs` | ch. 03 |
| line width | `w`, `LW` | 1.0 |
| line cap | `J`, `LC` | 0 (butt) |
| line join | `j`, `LJ` | 0 (miter) |
| miter limit | `M`, `ML` | 10.0 |
| dash pattern | `d`, `D` | solid `[] 0` |
| rendering intent | `ri`, `RI` | RelativeColorimetric |
| stroke adjustment | `SA` | false (device may default otherwise) |
| blend mode | `BM` | Normal |
| soft mask | `SMask` | None |
| alpha constant (stroke/fill) | `CA` / `ca` | 1.0 |
| alpha source | `AIS` | false (alpha is opacity, not shape) |
| black point compensation (PDF 2.0) | `UseBlackPtComp` | Default |

**Device-dependent:**

| Parameter | Set by | Notes |
|---|---|---|
| overprint (stroke/fill) | `OP` / `op` | false |
| overprint mode | `OPM` | 0; 1 = nonzero overprint for DeviceCMYK |
| black generation | `BG`, `BG2` | function of C,M,Y → K |
| undercolor removal | `UCR`, `UCR2` | |
| transfer function | `TR`, `TR2` | **deprecated in PDF 2.0** |
| halftone | `HT` | dictionary/stream |
| halftone origin (PDF 2.0) | `HTO` | |
| flatness | `i`, `FL` | curve-flattening tolerance in device pixels |
| smoothness | `SM` | shading accuracy tolerance |

### 4.2 ExtGState dictionaries

Most parameters have no operator; they're set by `/Name gs` referencing an **ExtGState** in the resources:

```
/GS1 << /Type /ExtGState
        /CA 0.5 /ca 0.5          % stroke / fill constant alpha
        /BM /Multiply            % blend mode
        /SMask /None             % or a soft-mask dictionary
        /LW 2 /LC 1 /LJ 1 /ML 4  % line params
        /D [[6 3] 0]             % dash
        /RI /Perceptual
        /OP true /op true /OPM 1 % overprint
        /SA true                 % stroke adjustment
        /Font [8 0 R 12]         % font + size
        /TK true                 % text knockout
        /UseBlackPtComp /ON      % PDF 2.0
     >>
```

Writer guidance: intern ExtGStates by value (hash the canonicalized dictionary) — a document with a million translucent shapes needs one or a handful of ExtGState objects, not a million. Prefer operators (`w`, `J`, …) for parameters that have them; use `gs` for alpha, blend, soft mask, overprint.

### 4.3 The stack as a scoping mechanism

`q…Q` is the *only* way to undo a `cm` or a clip (clips only shrink: `W` intersects). Therefore:

- Every "draw this object with this transform/clip" is `q … cm … W n … paint … Q`.
- A library should expose save/restore as a **scoped** construct (closure, RAII guard, `using`/`with` block) so balance is guaranteed by construction.
- Redundant state elimination (don't re-emit `rg` if unchanged) must be done against a *modeled* stack — after `Q` the writer's notion of current color must pop too.

---

## 5. Paths

### 5.1 Cubic Bézier curves

A segment `x1 y1 x2 y2 x3 y3 c` from the current point P0 with control points P1, P2 and end P3:

```
B(t) = (1−t)³·P0 + 3(1−t)²t·P1 + 3(1−t)t²·P2 + t³·P3,   t ∈ [0,1]
B'(0) = 3(P1 − P0),   B'(1) = 3(P3 − P2)
```

The curve starts tangent to P0→P1, ends tangent to P2→P3, and lies within the convex hull of the four points (which renderers use for cheap bounds and subdivision termination). `v` and `y` are shorthands where P1 = P0 or P2 = P3.

**Why cubic?** It is the lowest degree that (a) allows independent tangent directions *and* magnitudes at both ends, so segments join with G1/C1 continuity under free control; (b) supports an inflection point within one segment (S-curves); and (c) is closed under affine transforms (transform the control points, done). PostScript and Type 1 fonts used cubics; TrueType's quadratics convert exactly to cubics (degree elevation):

```
quadratic (Q0, Q1, Q2)  →  cubic (Q0,  Q0 + ⅔(Q1−Q0),  Q2 + ⅔(Q1−Q2),  Q2)
```

There is no arc operator. Arcs, circles and ellipses must be approximated.

**Circles: the kappa constant.** A quarter circle of radius r from (r,0) to (0,r) is approximated by control points (r, k·r) and (k·r, r), where

```
κ = 4/3 · (√2 − 1) ≈ 0.5522847498
```

This value makes the curve pass exactly through the circle at the 45° midpoint. Maximum radial error ≈ 0.027 % of r (a 1-inch-radius circle is off by ~0.02 pt). A slightly smaller κ ≈ 0.55191 minimizes the *maximum* error (~0.019 %) at the cost of not hitting the midpoint exactly; either is visually perfect. Ellipses: use κ·rx horizontally and κ·ry vertically, or draw a unit circle under a scaling `cm` (but then the stroke is scaled non-uniformly too — see §5.4). General arcs: split into segments ≤ 90°; for a segment of sweep φ, the control-point distance along the tangent is `(4/3)·tan(φ/4)·r`.

### 5.2 Subpaths and closing

A path is a list of **subpaths**; each starts with `m` (or `re`, which is `x y m, x+w y l, x+w y+h l, x y+h l, h`). `h` adds a straight closing segment *and* makes the subpath "closed", which matters for stroking: a closed subpath gets a **join** at its start point; an open one whose last point happens to coincide gets two **caps**. Fills implicitly close every subpath. A *degenerate* subpath — a single-point closed path or two or more points at the same coordinates (e.g. `x y m x y l`, or `x y m h`) — is stroked as a filled dot **only with round caps (1)**; with butt (0) or projecting square (2) caps `S` paints nothing, because cap orientation is indeterminate. A single-point *open* subpath (a trailing `x y m` alone) produces no output at all (ISO 32000-1 §8.5.3.2). The classic "dot" trick is therefore `1 J x y m x y l S`. (Zero-length *dashes* within a dash pattern always get caps, since the path direction is known.)

### 5.3 Fill rules

The interior of a self-intersecting or multi-subpath path is defined by one of two rules. For a point P, cast a ray from P to infinity in any direction and look at path segments crossing it.

**Nonzero winding (`f`, `B`, `b`, `W`):** start at 0; add +1 when a segment crosses the ray left-to-right (counter-clockwise around P), −1 right-to-left. P is inside if the sum ≠ 0.

**Even-odd (`f*`, `B*`, `b*`, `W*`):** count crossings regardless of direction. P is inside if the count is odd.

```
  Two concentric circles, both drawn counter-clockwise:      Outer CCW, inner CW:
     nonzero: ring AND hole filled (winding 2 in hole)          nonzero: hole empty (1 + −1 = 0)
     even-odd: hole empty (2 crossings)                         even-odd: hole empty

  Five-point star drawn as one self-crossing polyline:
     nonzero: solid star (centre winding = 2)
     even-odd: pentagon in the centre is empty
```

Use even-odd when you don't control directions (e.g. imported SVG with `fill-rule: evenodd`); use nonzero (and correct winding) for font-like shapes — TrueType/CFF outlines are designed for nonzero.

### 5.4 Stroking

Stroking paints a band of width `w` centred on the path, as if by sweeping a line segment perpendicular to the path. The stroke is computed **in user space** and then transformed — so a non-uniform CTM scale produces a calligraphic, variable-width stroke. To stroke an ellipse with uniform width, compute the ellipse's control points directly (κ·rx, κ·ry) rather than drawing a unit circle under a scaling `cm` — you cannot "undo" the scale between path construction and `S`, because `q`/`Q`/`cm` are illegal inside a path object.

Parameters:

- **Width** `w`: user-space units. `0` means "the thinnest line the device can render" (1 device pixel) — resolution-dependent, so avoid it in print-targeted output; it's also the source of "hairline" complaints.
- **Cap** `J`: 0 butt (ends at endpoint), 1 round (semicircle radius w/2), 2 projecting square (extends w/2).
- **Join** `j`: 0 miter, 1 round, 2 bevel.
- **Miter limit** `M`: at a join with interior angle φ, the miter length / line width = `1 / sin(φ/2)`. If that exceeds `M`, the join becomes a bevel. Default 10 → miters are cut for angles below ≈ 11.5°. `M 1.414` cuts at 90°.
- **Dash** `[on off on off …] phase d`: lengths in user space, cycling; `phase` is the distance into the pattern at which each subpath starts. `[] 0 d` = solid. `[3] 0 d` = 3 on, 3 off. Each subpath restarts the pattern; caps apply to each dash.

Strokes and fills are painted with separate colors (stroking vs nonstroking); `B` fills then strokes the same path, so the inner half of the stroke covers the fill edge.

### 5.5 Scan conversion rules (ISO 32000-1 §10.6.4; originally cited here as 2.0 §10.7, unverified)

The spec describes the *ideal* geometry and then gives rules for mapping to pixels:

- **Filling:** a pixel is painted if **any part** of it lies within the shape ("any part of pixel" rule). This is Adobe's PostScript rule: it guarantees nothing ever disappears, even zero-area shapes, at the price of fattening shapes by up to one pixel on each edge. Anti-aliased renderers instead compute *coverage* and treat it as alpha — which is also compliant in spirit and is what screen viewers do.
- **Zero-area fills** still paint pixels under the any-part rule (a zero-width or zero-height rectangle paints a 1-pixel line; a degenerate single-point fill paints the one pixel under it). **Zero-width strokes** (`0 w`) are explicitly implementation-defined and may paint fewer pixels than the rule implies (ISO 32000-1 §10.6.4, §8.5.3.3.1).
- **Stroke adjustment** (`SA true`): the renderer may nudge stroke coordinates so that line widths are uniform after rounding to the pixel grid (a 1.4-pixel line otherwise alternates between 1 and 2 pixels). Important for table rules and hairlines on low-resolution devices.
- **Images** use a different rule: a pixel is painted with an image sample if the pixel's *centre* lies within the transformed image sample.
- **Flatness** (`i`, `FL`): the maximum allowed device-pixel distance between the true curve and the polyline a renderer uses to approximate it. Most modern renderers ignore the value and choose their own tolerance (~0.25 px).

---

## 6. Color

### 6.1 Color science minimum

- **CIE XYZ (1931)** is a device-independent tristimulus space derived from the standard observer's color-matching functions; Y is luminance. **CIE L\*a\*b\*** is a perceptually more uniform transform of XYZ relative to a white point.
- An **ICC profile** describes how a device's native values (RGB, CMYK, …) map to a *profile connection space* (PCS: XYZ or Lab, D50). Converting device A → device B is A→PCS→B, with gamut mapping governed by a **rendering intent**.
- **Rendering intents:** *Perceptual* (compress the whole gamut, preserve relationships; photos), *RelativeColorimetric* (map in-gamut colors exactly relative to the media white, clip out-of-gamut; PDF default), *Saturation* (vividness; business graphics), *AbsoluteColorimetric* (no white-point adaptation; proofing paper simulation). Black-point compensation (PDF 2.0 `UseBlackPtComp`) additionally maps source black to destination black for relative colorimetric.
- **Why CMYK ≠ RGB.** RGB is *additive* light (emissive screens, gamut defined by phosphor/LED primaries). CMYK is *subtractive* inks on paper; its gamut depends on ink, paper, dot gain, and total ink limits; it's smaller in saturated blues/greens and different in shape. K exists because CMY overprinted makes muddy brown, wastes ink, and causes registration problems with small black text. There's no single formula between them: `DeviceCMYK 0 0 0 1` is a rich "registration-dependent black" on one press and something else on another. Any RGB↔CMYK conversion without profiles is a guess.

### 6.2 Color space families

| Family | Spaces | Components | Notes |
|---|---|---|---|
| **Device** | DeviceGray, DeviceRGB, DeviceCMYK | 1, 3, 4 (each 0–1) | Uncalibrated; meaning depends on device or output intent |
| **CIE-based** | CalGray, CalRGB, Lab, ICCBased | 1, 3, 3, N | Device-independent |
| **Special** | Indexed, Pattern, Separation, DeviceN | 1, 0/N, 1, N | Built on top of a base/alternate space |

**Syntax (as color space arrays in `/ColorSpace` resources):**

```
[/CalGray << /WhitePoint [0.9505 1 1.089] /Gamma 2.2 >>]
[/CalRGB  << /WhitePoint [...] /Gamma [2.2 2.2 2.2] /Matrix [9 numbers XYZ] >>]
[/Lab     << /WhitePoint [0.9642 1 0.8249] /Range [-128 127 -128 127] >>]
[/ICCBased 12 0 R]       % stream: << /N 3 /Alternate /DeviceRGB /Length … >> + ICC bytes
[/Indexed /DeviceRGB 255 <...768 bytes...>]          % base, hival (≤255), lookup
[/Pattern /DeviceRGB]                                 % uncolored pattern, underlying space
[/Separation /PANTONE#20185#20C /DeviceCMYK 15 0 R]   % name, alternate, tint transform fn
[/DeviceN [/Cyan /Magenta /Spot1] /DeviceCMYK 16 0 R << /Subtype /NChannel ... >>]
```

Notes:

- **ICCBased** is the workhorse for accurate color: embed sRGB IEC61966-2.1 (≈3 KB) once and reference it everywhere you'd use DeviceRGB. `N` must match the profile's channel count; `Alternate` is a fallback. PDF 1.7 references ICC.1:2004-10 (ISO 15076-1:2005; ISO 32000-1 Table 67) and says writers *should* use such profiles; PDF 2.0 references ICC.1:2010 (profile version 4.3) — the 2.0 figure is unverified here; v2 profiles are universally supported. ICC v5/iccMAX is not part of PDF 2.0.
- **Indexed** maps a single component 0…hival through a lookup table (hival+1 entries × base components bytes). Base can't be Pattern or Indexed. Natural for PNG palette images.
- **Separation** names a single colorant (spot ink: Pantone, varnish, die-cut line). On a device with that ink, the tint is painted on its plate; otherwise the **tint transform** function maps tint → alternate space. Special names: `/All` (every plate, e.g. crop marks) and `/None` (paints nothing).
- **DeviceN** generalizes to N colorants (duotones, hexachrome, CMYK + spots in one image). `/NChannel` subtype (1.6) adds `Colorants`, `Process`, `MixingHints` for better previews.
- **Pattern** color space: the "color" is a pattern (§9). For uncolored tiling patterns the space carries an underlying space and the `scn` supplies its components: `/CSp cs 1 0 0 /P1 scn`.

Setting a color space with `cs` also resets the color to its initial value (black in device spaces, 1.0 tint for Separation — which is *full ink*, not white, a classic surprise).

### 6.3 Default color spaces and device conversion

A resource dictionary may define `/DefaultGray`, `/DefaultRGB`, `/DefaultCMYK` in its `/ColorSpace` subdictionary; when present, every use of the corresponding device space in that scope is reinterpreted as the given space — in practice an ICCBased or Cal* space; the spec allows any space except Lab, Indexed or Pattern, with the same number of components, and values are passed through unchanged (ISO 32000-1 §8.6.5.6). This lets a producer write simple `rg` operators yet get calibrated color.

When a device lacks the requested space and no profile is available, the spec gives naive conversions (ISO 32000-1 §10.3; originally cited as 2.0 §10.4, unverified): gray = 0.30R + 0.59G + 0.11B; RGB→CMY as complements followed by black generation/UCR; CMYK→RGB as `R = 1 − min(1, C + K)` etc. Viewers use these for DeviceCMYK on screen unless an output intent says otherwise, which is why unmanaged CMYK looks dull and oversaturated in different viewers.

### 6.4 Output intents (PDF/A, PDF/X)

`/OutputIntents` in the catalog (and, in PDF 2.0, optionally on pages) declares the intended output condition:

```
/OutputIntents [ << /Type /OutputIntent /S /GTS_PDFA1
                   /OutputConditionIdentifier (sRGB IEC61966-2.1)
                   /DestOutputProfile 20 0 R >> ]
```

- **PDF/A** (`/S /GTS_PDFA1`, used by all PDF/A parts): device color spaces are only allowed if an output intent with a profile of matching type exists (or if DefaultXXX spaces are set); otherwise all color must be ICC-based. For lamina: to emit PDF/A, embed sRGB as an output intent and you can keep using `rg`.
- **PDF/X** (`/S /GTS_PDFX`): names the press characterization (e.g. FOGRA51, GRACoL2013). Device CMYK in the file is understood as that condition's CMYK. PDF/X-1a forbids RGB and transparency; X-4 allows both.
- Effect on rendering: a viewer with "simulate output intent" interprets DeviceCMYK through `DestOutputProfile`.

### 6.5 Overprint

In separation-based printing, painting an object normally **knocks out** (erases) the other plates beneath it. With overprint on (`OP` stroke / `op` fill), only the plates the object actually paints are changed; the others keep earlier ink. Uses: black text overprinting backgrounds (no white halo on misregistration), spot varnish. `OPM 1` ("nonzero overprint mode") additionally treats DeviceCMYK components equal to 0 as "don't touch" — the conventional setting. Screen viewers ignore overprint by default (Acrobat has "Overprint Preview"); MuPDF and Ghostscript can simulate it. A writer targeting print should expose overprint; one targeting screens can ignore it.

---

## 7. Images

### 7.1 Image XObjects

An image is a stream with `/Type /XObject /Subtype /Image` (chapter 01 covers streams/filters). Painted by `/Im1 Do`.

| Key | Meaning |
|---|---|
| `Width`, `Height` | Samples per row, rows (required) |
| `ColorSpace` | Any space except Pattern; optional for JPX; **must not be present** for image masks |
| `BitsPerComponent` | 1, 2, 4, 8, 16 (16 since 1.5); omitted for JPX; 1 for masks |
| `Decode` | Pairs `[Dmin Dmax]` per component mapping sample values to the color range |
| `Interpolate` | Hint to smooth when upsampling (viewers vary) |
| `ImageMask` | true → stencil mask; samples paint current fill color |
| `Mask` | Stencil-mask image stream, or color-key array `[min1 max1 … minN maxN]` |
| `SMask` | Soft-mask image (DeviceGray alpha) — PDF 1.4 |
| `SMaskInData` | JPX only: 0 ignore, 1 use embedded alpha, 2 premultiplied alpha |
| `Intent` | Rendering intent for this image |
| `Alternates`, `OC`, `Metadata`, `StructParent`, `AF` (2.0) | Alternates, optional content, XMP, structure, associated files |

**Decoding:** sample value `s` (integer 0…2^bpc−1) maps to `Dmin + s·(Dmax − Dmin)/(2^bpc − 1)`. Default Decode is `[0 1]` per component for most spaces, `[0 2^bpc−1]` for Indexed, and the space's `Range` for Lab (`[0 100 amin amax bmin bmax]`) and ICCBased (ISO 32000-1 §8.9.5.2, Table 90). `/Decode [1 0]` inverts — used for 1-bit masks and Adobe-inverted CMYK JPEGs.

**Geometry:** image space is `Width × Height` with the origin at the **top-left** of the image, first sample row at the top. It's mapped onto the **unit square** of user space, with image row 0 at y = 1. Therefore drawing an image at (x, y) of size (w, h) is always:

```
q  w 0 0 h x y cm  /Im1 Do  Q
```

The implicit image matrix is `[1/Width 0 0 −1/Height 0 1]`; you never write it.

### 7.2 Masking

- **Stencil mask** (`/ImageMask true`, bpc 1, no ColorSpace): 1-bit samples select where the *current nonstroking color* (or pattern) is painted. Default: sample 0 paints (`Decode [0 1]`); `[1 0]` inverts. Ideal for scanned text/line art and Type 3 glyph bitmaps.
- **Explicit mask** (`/Mask <stencil image stream>`): a separate 1-bit `ImageMask` image (may have a different resolution; both are mapped to the same unit square). Same convention as stencil masks: with the default Decode, sample 0 = paint the base image, 1 = masked out. Rarely used now; prefer SMask.
- **Color-key mask** (`/Mask [min max …]`): samples whose every component falls within the ranges are not painted (GIF/PNG `tRNS`-style transparency). Ranges are in *raw sample* values, before Decode.
- **Soft mask** (`/SMask <image>`): per-pixel alpha, 0–1, as an image whose `ColorSpace` is required to be DeviceGray, of independent resolution and bpc — except that with `/Matte` its Width/Height must equal the parent's. Optional `/Matte [c…]` means the base image color was *pre-blended* with the matte color (`c′ = m + α·(c − m)`); the renderer un-premultiplies. Soft-mask images participate in the transparency model (§10): an image's `SMask` (or `Mask`) **overrides** the current soft mask in the graphics state — at most one mask applies to any compositing operation — while the alpha constant `ca` and blend mode still apply; so the effective opacity is `SMask × ca` (ISO 32000-1 Table 89, §11.6.4.3, §11.6.5.3).

### 7.3 Inline images

```
BI /W 16 /H 16 /BPC 8 /CS /RGB /F /Fl /L 123 ID <binary data> EI
```

Abbreviated keys: `W H BPC CS D DP F IM I` (`Intent` has no abbreviation; PDF 1.7 says `Length` is unnecessary and other keys are ignored — the optional `L`/`Length` key is a PDF 2.0 addition, not verifiable from 1.7); abbreviated values `G RGB CMYK I` (Indexed) and filters `AHx A85 LZW Fl RL CCF DCT`. Without a resource name, `CS` must be a device space or an Indexed space with a device base and a byte-string table; named color spaces from resources may be any space usable by an image XObject. JPXDecode and JBIG2Decode are forbidden. Parsing hazard: the data is raw bytes and may contain `EI`; readers must use the computed/declared length or heuristics (whitespace + `EI` + whitespace + plausible following operator). **Writers should not emit inline images** except for tiny data — ISO 32000-1 §8.9.7 makes this normative: inline images "shall be used only for small images (4 KB or less)"; when writing PDF 2.0, always include `/L` if you do. Their only benefit — avoiding an indirect object — is irrelevant with object streams.

### 7.4 Compression choices

| Filter | Type | Best for | Notes |
|---|---|---|---|
| `DCTDecode` | lossy JPEG | Photos | Pass-through from .jpg; baseline + progressive; 8-bit only; `ColorTransform` in DecodeParms |
| `JPXDecode` (1.5) | JPEG 2000 | High-quality photos, 16-bit, alpha | Decoder-heavy; color space and alpha may be inside the codestream; PDF/A-2+ restrictions |
| `FlateDecode` | lossless zlib | Screenshots, line art, anything with alpha | With PNG predictors (`/Predictor 10–15`) ≈ PNG size |
| `JBIG2Decode` (1.4) | bilevel, lossless or lossy | Scanned text | `JBIG2Globals` shared dictionary; lossy symbol substitution has caused real errors (the "6 vs 8" Xerox incident) — prefer lossless/generic |
| `CCITTFaxDecode` | bilevel | Fax/scans | `K` (<0 G4, 0 G3 1D, >0 G3 2D), `Columns`, `Rows`, `BlackIs1` |
| `RunLengthDecode`, `LZWDecode` | lossless | legacy | Don't write LZW; Flate is better |

### 7.5 Embedding PNG from code

PNG's `IDAT` stream is **zlib data of filtered scanlines** — byte-compatible with `FlateDecode` + PNG predictors:

```
/Filter /FlateDecode
/DecodeParms << /Predictor 15 /Colors 3 /BitsPerComponent 8 /Columns <width> >>
```

`Predictor ≥ 10` means "PNG predictor, per-row filter type byte" (15 = "optimum", any per-row type). So:

1. **Non-interlaced Gray/RGB, no alpha:** concatenate IDAT chunk payloads, write them verbatim as the stream. Zero recompression.
2. **Palette (color type 3):** same pass-through with `/ColorSpace [/Indexed /DeviceRGB n-1 <PLTE bytes>]`, `Colors 1`. `tRNS` with a single fully-transparent index → color-key mask `/Mask [i i]`; partial alpha → expand to an SMask.
3. **Alpha (Gray+A, RGBA):** must decompress, undo filters, split color and alpha planes, recompress each (base image + `/SMask` DeviceGray). PDF has no interleaved-alpha image format (except JPX).
4. **Interlaced (Adam7):** must decode and re-encode.
5. **16-bit:** supported since PDF 1.5 with `BitsPerComponent 16`; or downconvert.
6. **Color:** `iCCP` → `/ICCBased` stream; `sRGB` chunk → embed sRGB profile; `gAMA`/`cHRM` → CalRGB or ignore.

### 7.6 Embedding JPEG from code

JPEG is a **pass-through**: write the .jpg bytes unchanged as the stream with `/Filter /DCTDecode`. Parse just enough of the file to fill the dictionary: walk markers to the `SOFn` segment for precision (bpc, must be 8), height, width, component count (1 → DeviceGray, 3 → DeviceRGB, 4 → DeviceCMYK). Gotchas:

- **Adobe CMYK JPEGs** (APP14 "Adobe" marker) store inverted CMYK; add `/Decode [1 0 1 0 1 0 1 0]`. Most libraries apply this heuristic when the APP14 marker is present.
- **EXIF orientation** is ignored by PDF; apply it via the placement `cm` (rotate/flip the unit square) or re-encode.
- **Embedded ICC profile** (APP2 `ICC_PROFILE`, possibly split across segments) → extract into an `/ICCBased` color space for accurate color.
- `SOF2` (progressive) is fine (PDF 1.3+); arithmetic-coded JPEG (`SOF9+`) is poorly supported — reject or transcode.

---

## 8. Form XObjects

A **form XObject** is a self-contained content stream that can be painted any number of times:

```
10 0 obj
<< /Type /XObject /Subtype /Form
   /BBox [0 0 200 50]            % clip, in form space (required)
   /Matrix [1 0 0 1 0 0]         % form space → user space (default identity)
   /Resources << /Font << /F1 5 0 R >> >>
   /Group << /S /Transparency /I true /K false /CS /DeviceRGB >>   % optional
   /Length … >>
stream
0.9 g 0 0 200 50 re f  BT /F1 18 Tf 0 g 10 18 Td (CONFIDENTIAL) Tj ET
endstream
```

Painting `/Fm1 Do` is semantically:

```
q
  <Matrix> cm
  <BBox rectangle> W n
  <form content, resolved against form's /Resources>
Q
```

(If `/Group` is present, the content is composited as a transparency group first — §10.4.)

Why they matter for a library:

- **Templating/reuse.** A letterhead, logo, page border, or repeated diagram symbol is stored once and referenced on every page; file size and parse time drop, and viewers cache the rendering.
- **Stamping/imposition.** Any existing PDF page can be turned into a form XObject: take its (concatenated, decoded) content stream, its `/Resources`, its CropBox as `/BBox`, and adjust `/Matrix` for `/Rotate`. Overlaying/underlaying pages, n-up, watermarks, and "import page as graphic" (LaTeX `pdfpages`, qpdf `--overlay`/`--underlay`, pypdf `merge_page`-style transforms) all do this. It avoids resource-name collisions because the form has its own resource scope.
- **Annotation appearances** (`/AP` streams) are form XObjects (chapter on interactivity).
- **Groups.** Wrapping content in a form with `/Group /Transparency` is how you get "layer opacity" (Skia `saveLayer`, CSS `opacity` on a container) — see §10.
- **Reference XObjects** (`/Ref`) point to a page in another PDF file; poorly supported, avoid.

---

## 9. Patterns and shadings

### 9.1 Tiling patterns (PatternType 1)

A tiling pattern is a small content stream replicated over the plane:

```
/P1 << /Type /Pattern /PatternType 1
       /PaintType 1          % 1 colored (cell sets its own colors), 2 uncolored (stencil, color from scn)
       /TilingType 1         % 1 constant spacing, 2 no distortion, 3 constant spacing + faster
       /BBox [0 0 10 10] /XStep 10 /YStep 10
       /Matrix [1 0 0 1 0 0]
       /Resources << >> /Length … >>
stream  0 0 5 5 re f  5 5 5 5 re f  endstream
```

Usage: `/Pattern cs /P1 scn  0 0 300 300 re f` (colored) or `[/Pattern /DeviceRGB]` in a resource `/CSp`, then `/CSp cs 0 0 1 /P1 scn` (uncolored, blue).

**Pattern space pitfall:** `/Matrix` maps pattern space to the **default coordinate space of the pattern's parent content stream** (the page's default user space, or the form's space for a pattern used inside a form), *not* to the CTM at the time of use. Moving or scaling the shape with `cm` does *not* move the pattern; it stays anchored to the page. For a pattern that follows an object you must bake the object's transform into the pattern's `/Matrix` (one pattern object per distinct transform). This differs from Canvas/SVG and trips up almost everyone. Also: tile seams under anti-aliasing (§12.2) — renderers mitigate by rendering the cell once into a bitmap and tiling it.

### 9.2 Shading patterns (PatternType 2) and `sh`

A shading can be used two ways:

1. **As a pattern** (`/PatternType 2 /Shading … /Matrix …`) set via `scn`, then any fill/stroke/text paints with it. Pattern-space rules apply.
2. **Directly via `/Sh1 sh`:** paints the shading across the *current clipping region*, in **current user space** (CTM applies). Not affected by the current color. Idiom: `q <shape> W n /Sh1 sh Q`. Simpler, and the transform behaves intuitively — prefer this for gradients on shapes.

Common shading dictionary keys: `ShadingType`, `ColorSpace`, `Background` (only for pattern use), `BBox`, `AntiAlias`.

### 9.3 Shading types

| Type | Name | Defined by | Use |
|---|---|---|---|
| 1 | Function-based | `Domain [x0 x1 y0 y1]`, `Matrix`, `Function` f(x,y) → color | Arbitrary 2-D fields (rare) |
| 2 | Axial | `Coords [x0 y0 x1 y1]`, `Domain [t0 t1]`, `Function` f(t), `Extend [b b]` | Linear gradients |
| 3 | Radial | `Coords [x0 y0 r0 x1 y1 r1]`, `Domain`, `Function`, `Extend` | Radial/conical (two-circle) gradients |
| 4 | Free-form Gouraud triangle mesh | stream of `(flag, x, y, color)` vertices | Arbitrary meshes; flag 0 = new triangle, 1/2 = reuse edge |
| 5 | Lattice-form Gouraud mesh | stream of vertices, `VerticesPerRow` | Regular grids (terrain, heatmaps) |
| 6 | Coons patch mesh | 12 control points per patch (4 Bézier edges) + 4 corner colors | Gradient meshes (Illustrator) |
| 7 | Tensor-product patch mesh | 16 control points per patch + 4 corner colors | Most general; Coons is a special case |

**Axial math:** for a point p, the parameter is the projection onto the axis:

```
s = ((p − P0) · (P1 − P0)) / |P1 − P0|²       t = t0 + s·(t1 − t0)
color = Function(t)   for s ∈ [0,1];  outside → extend endpoints if Extend, else unpainted
```

**Radial math:** the shading is the family of circles `C(s)` centred `P0 + s(P1−P0)` with radius `r0 + s(r1−r0)`; for each point, the *largest* s whose circle contains it determines the color (so later circles paint over earlier — painter's model again). This is the same "two-point conical" gradient as Canvas `createRadialGradient` and Skia, which is why pdf.js can map it directly.

**Mesh streams** (types 4–7) are packed binary: `BitsPerCoordinate` (1, 2, 4, 8, 12, 16, 24 or 32), `BitsPerComponent` (1, 2, 4, 8, 12 or 16), `BitsPerFlag` (2, 4, 8; only the low 2 bits used; not used by type 5), and `Decode` ranges mapping integers to coordinates/colors; or a single parametric component with `Function`. Coordinates are relative to the shading's space. Colors are interpolated (Gouraud: bilinear in barycentric coordinates for triangles; bilinear in (u,v) parameter space for patches). Patch edges are shared with flags 1–3 (reuse an edge of the previous patch).

Multi-stop gradients (CSS/SVG `<stop>`s): one type-2/3 shading with a **Type 3 stitching function** whose sub-functions are Type 2 linear segments. Per-stop opacity requires a *soft mask* carrying a gray gradient with the same geometry, because shading colors have no alpha.

### 9.4 Functions

PDF functions map m inputs to n outputs; every function has `Domain` (clip inputs) and, for types 0 and 4, `Range` (clip outputs).

| Type | Name | Definition |
|---|---|---|
| 0 | Sampled | m-dimensional table: `Size`, `BitsPerSample`, `Order` (1 linear, 3 cubic spline), `Encode`, `Decode`; stream of samples. Multilinear interpolation. |
| 2 | Exponential | 1 input: `y_j = C0_j + x^N · (C1_j − C0_j)`; `N=1` is linear interpolation |
| 3 | Stitching | 1 input: `Functions [f0…fk]`, `Bounds [b1…bk]` subdividing Domain, `Encode [e0 e1 …]` remapping each subdomain to its function's domain |
| 4 | PostScript calculator | Stream `{ … }` using only: arithmetic (`abs add atan ceiling cos cvi cvr div exp floor idiv ln log mod mul neg round sin sqrt sub truncate`), relational/boolean/bitwise (`and bitshift eq false ge gt le lt ne not or true xor`), conditional (`if ifelse`), stack (`copy dup exch index pop roll`). No loops, no variables. Trig in degrees. |

Example Type 4 tint transform for a spot color → CMYK: `{ dup 0.0 mul exch dup 0.81 mul exch dup 0.87 mul exch 0.0 mul }`. Renderers often compile type-4 programs (pdf.js compiles to JS; MuPDF and PDFium interpret with a fixed-size stack — 100 entries, which is the spec's own requirement: implementations shall provide room for at least 100 entries, need not provide more, and overflow is an error, ISO 32000-1 §7.10.5.1) and cache samples of 1-input functions.

---

## 10. Transparency (PDF 1.4+)

### 10.1 Basic compositing formula

Each painted element ("source") has color `Cs`, shape `fs`, opacity `qs`, combined into alpha `αs = fs · qs` (where `qs` is the product of the constant alpha `ca`/`CA` and *one* soft mask — the image's own `SMask` if present, which overrides the graphics-state soft mask, else the graphics-state soft mask; with `AIS true` these contribute to shape instead; ISO 32000-1 §11.6.4). It is composited over the **backdrop** (`Cb`, `αb`):

```
αr = αb + αs − αb·αs                                   (union)
Cr = (1 − αs/αr)·Cb + (αs/αr)·[ (1 − αb)·Cs + αb·B(Cb, Cs) ]
```

`B` is the blend function. With `αb = 1` (opaque page) this simplifies to `Cr = (1−αs)·Cb + αs·B(Cb,Cs)`, and with `B = Normal` (`B = Cs`) to familiar "over". The full model (ISO 32000-2 §11.3) tracks shape and opacity separately (needed for knockout groups and `AIS`), not just alpha.

**Subtractive spaces.** In DeviceCMYK (and other subtractive spaces), blend modes are applied to the *complements* (1 − c) so that e.g. Multiply darkens as expected; results are complemented back. Separable modes on spot colorants likewise. Non-separable modes are defined on RGB; in CMYK blending spaces C, M, Y are complemented to RGB, blended, and converted back, while K is taken from the backdrop (`Cb`) for Hue, Saturation and Color and from the source (`Cs`) for Luminosity (ISO 32000-1 §11.3.5). Only separable modes may be used for spot colorants.

### 10.2 Blend modes

Values in [0,1]; `Cb` backdrop, `Cs` source.

**Separable** (applied per component):

| Mode | B(Cb, Cs) |
|---|---|
| Normal | Cs |
| Multiply | Cb · Cs |
| Screen | Cb + Cs − Cb·Cs |
| Overlay | HardLight(Cs, Cb) |
| Darken | min(Cb, Cs) |
| Lighten | max(Cb, Cs) |
| ColorDodge | Cb = 0 → 0; Cs = 1 → 1; else min(1, Cb / (1 − Cs)) ¹ |
| ColorBurn | Cb = 1 → 1; Cs = 0 → 0; else 1 − min(1, (1 − Cb) / Cs) ¹ |
| HardLight | Cs ≤ 0.5 → Multiply(Cb, 2Cs); else Screen(Cb, 2Cs − 1) |
| SoftLight | Cs ≤ 0.5 → Cb − (1 − 2Cs)·Cb·(1 − Cb); else Cb + (2Cs − 1)·(D(Cb) − Cb), with D(x) = x ≤ 0.25 ? ((16x − 12)x + 4)x : √x |
| Difference | \|Cb − Cs\| |
| Exclusion | Cb + Cs − 2·Cb·Cs |

¹ This is the W3C form. ISO 32000-1 Table 136 omits the leading `Cb` test: ColorDodge = `Cs < 1 → min(1, Cb/(1−Cs)); Cs = 1 → 1`, ColorBurn = `Cs > 0 → 1 − min(1, (1−Cb)/Cs); Cs = 0 → 0`. The two differ only at the corners (Cb = 0, Cs = 1 gives 1 in PDF 1.7 vs 0 in W3C; Cb = 1, Cs = 0 likewise). Whether PDF 2.0 adopted the W3C edge cases is unverified here.

(`Compatible` is still listed in ISO 32000-1 Table 136 as "same as Normal", for compatibility only, and should not be written.)

**Non-separable** (operate on whole RGB color), with helpers:

```
Lum(C)      = 0.3·R + 0.59·G + 0.11·B
ClipColor(C): l = Lum(C); n = min(R,G,B); x = max(R,G,B)
              if n < 0: C = l + (C − l)·l / (l − n)
              if x > 1: C = l + (C − l)·(1 − l) / (x − l)
SetLum(C,l) = ClipColor(C + (l − Lum(C)))
Sat(C)      = max(R,G,B) − min(R,G,B)
SetSat(C,s) = rescale so that max−min = s, min component → 0, mid scaled proportionally

Hue        = SetLum(SetSat(Cs, Sat(Cb)), Lum(Cb))
Saturation = SetLum(SetSat(Cb, Sat(Cs)), Lum(Cb))
Color      = SetLum(Cs, Lum(Cb))
Luminosity = SetLum(Cb, Lum(Cs))
```

These match the W3C *Compositing and Blending Level 1* definitions (which were derived from PDF) apart from the ColorDodge/ColorBurn corner cases noted above, so a lamina renderer can share code and test vectors with browser implementations (pick and document one convention for the corners).

### 10.3 Constant alpha and soft masks

- `CA` (stroking) and `ca` (nonstroking, also images, shadings via `sh`, and form XObjects as wholes) — constant opacity.
- `AIS true` reinterprets these (and the soft mask) as *shape* rather than opacity — matters only in knockout groups.
- **Soft mask** in ExtGState: `/SMask << /Type /Mask /S /Alpha|/Luminosity /G <transparency group form> /BC [backdrop color] /TR <transfer fn> >>`.
  - `/Alpha`: the mask value is the group's computed alpha (shape × opacity) at each point.
  - `/Luminosity`: the group is composited over the backdrop color `BC` (default black) and the resulting color's luminosity becomes the mask; the group must specify `/CS` in its group dictionary. This is how "gradient fade-outs" are encoded: a luminosity mask containing a white→black shading.
  - The mask's coordinate space is fixed **at the time `gs` is executed** (CTM then), not when objects are painted. `/SMask /None` removes it.

### 10.4 Transparency groups

A form XObject with `/Group << /S /Transparency … >>` (or a page with a `/Group`) is a **transparency group**: its contents are composited together into a temporary result, and that result is composited onto the backdrop as one object, using the graphics state (alpha, blend mode, soft mask) that was current at `Do`.

| Entry | Meaning |
|---|---|
| `/CS` | **Blending color space** for the group (e.g. DeviceRGB, an ICC space); required for luminosity masks |
| `/I` | **Isolated**: true → group starts with a fully transparent backdrop; false → it starts with (and blends against) whatever is underneath |
| `/K` | **Knockout**: true → each element composites against the group's *initial* backdrop, so later elements replace earlier ones within the group instead of blending with them |

Picture it:

```
  Non-isolated, non-knockout (default):    elements blend with each other and with what's under the group
  Isolated:                                blend modes inside see only the group's own content
  Knockout:                                overlapping translucent circles don't darken each other (like one shape)
```

The **page group** (`/Group` on the page) defines the color space in which the page's transparency is blended — set `/CS /DeviceRGB` (or an ICC space) on pages with transparency to get consistent results across viewers; otherwise viewers pick (Acrobat: often CMYK-ish document blending space, causing color shifts).

### 10.5 Why flattening exists

Before PDF 1.4, and still on PostScript RIPs, PDF/X-1a/X-3 workflows, and some printers, there is no compositing. **Flattening** converts a page with transparency into an equivalent opaque page: compute the planar map of all overlapping regions ("atomic regions"), compute each region's composited color, emit them as opaque vectors where possible and rasterize where not (shadings under blend modes, soft masks, complex overlaps). Costs: geometry explosion, rasterized vector content, and the infamous **stitching hairlines** where adjacent flattened regions meet under anti-aliased viewing. Modern print workflows (PDF/X-4, Adobe PDF Print Engine) keep live transparency; lamina should not plan to flatten except as an optional export path.

---

## 11. Optional content (layers)

PDF 1.5 **optional content groups (OCGs)** are named visibility switches:

```
% catalog
/OCProperties << /OCGs [30 0 R 31 0 R]
                 /D << /Order [30 0 R 31 0 R] /ON [30 0 R] /OFF [31 0 R]
                       /RBGroups [] /Locked [] /AS [ … print/view auto-state … ] >> >>
30 0 obj << /Type /OCG /Name (Dimensions) /Usage << /Print << /PrintState /ON >> >> >>

% page resources
/Properties << /oc1 30 0 R >>

% content
/OC /oc1 BDC   … dimension lines …   EMC
```

- Content can be bound to OCGs by marked content (`/OC … BDC/EMC`), by `/OC` on an XObject (image/form) or on an annotation.
- **OCMDs** (membership dictionaries) combine groups: `/P /AllOn|/AnyOn|/AnyOff|/AllOff`, or `/VE` visibility expressions (`[/And …] [/Or …] [/Not …]`, PDF 1.6).
- `/D` is the default configuration; `/Configs` provides alternates. `/Usage` + `/AS` let content change with print/view/export context (e.g., watermark visible only when printing; language-specific layers).
- Hidden content still exists — layers are **not** a security mechanism.
- Renderer: maintain a stack of visibility; when hidden, skip painting but **still execute** state changes (`q`, `cm`, color) so the state stays consistent.

---

## 12. Rendering and rasterization in practice

### 12.1 Renderer survey

| Engine | Used by | Rasterizer | Notes |
|---|---|---|---|
| **pdf.js** | Firefox, many web apps | HTML Canvas 2D (browser's Skia/Direct2D/…) | Interprets content streams into canvas calls on the main thread or worker+OffscreenCanvas. Axial/radial → canvas gradients; meshes rasterized into small bitmaps; blend modes → `globalCompositeOperation`; groups/soft masks → offscreen canvases. Enforces a minimum ~1-device-pixel line width. |
| **PDFium** | Chrome, Edge, Android | AGG (Anti-Grain Geometry) historically; Skia backend under active adoption in Chromium | Own compositing (`CFX_DIBitmap`), own shading code. |
| **MuPDF** | SumatraPDF, many mobile apps, PyMuPDF | Own "fitz" draw device with edge-list scan converter | Configurable AA levels (0–8 bits), overprint simulation, spot-color rendering; very fast; mesh shadings subdivided to triangles. |
| **Poppler** | Evince, Okular, `pdftoppm`, `pdftocairo` | **Splash** (own rasterizer, AA by 4×4 supersampling) or **Cairo** backend | xpdf lineage. Splash is the reference for `pdftoppm`; Cairo output for vector conversion (SVG/PS). |
| **Ghostscript** | Print RIPs, conversions | Own; implements PostScript rules ("any part of pixel", "fill adjust") | Most complete for print features (halftones, overprint, separations). |
| **Skia (SkPDF)** | Chrome "Save as PDF", Android printing | — | Skia's PDF *backend* (a writer), useful as a reference for API→PDF mapping. |
| **Apple PDFKit / Quartz** | macOS, iOS | Core Graphics | Quartz's drawing model was designed around PDF. |

### 12.2 Anti-aliasing and its discontents

Screen renderers compute **coverage** per pixel (analytically, via signed-area accumulation, or by supersampling) and use it as alpha. Consequences:

- **Conflation artifacts / seams.** Two shapes that abut exactly each cover 50% of an edge pixel; compositing 50% over 50% gives 75% coverage, so a faint background-colored line shows through (stitched tiles, flattened transparency, mesh triangles, table cells drawn as separate rectangles). Mitigations: renderers render tiling-pattern cells to a bitmap and blit; mesh shadings are rasterized without AA; writers can overlap adjacent fills slightly or draw one combined path.
- **Thin lines.** A 0.1 pt line at 72 dpi is 0.1 px wide → 10% gray. Acrobat's "Enhance thin lines" and pdf.js's minimum width make these visible; MuPDF/PDFium apply minimum widths too, each differently. Stroke adjustment is the spec-blessed tool. A writer should not emit `0 w` for print and should be aware that hairline output differs between viewers.
- **Aliased "any part of pixel"** (Adobe printers, Ghostscript) visibly fattens small text and thin fills; centre-sampling (most AA rasterizers when AA is off) can make thin shapes disappear. Test at low DPI.
- **Text** is usually rendered with special hinting/grid-fitting and sometimes subpixel AA — it isn't just "fill the glyph path" (ch. 03).

### 12.3 Cost model

Rough order from cheap to expensive:

1. Opaque solid fills/strokes of simple paths — scan conversion, O(edges + covered pixels).
2. Images — dominated by decoding (JPEG2000 and JBIG2 are notoriously slow) and resampling; downscaling large images on every redraw is the commonest viewer bottleneck, so renderers cache decoded/downsampled images.
3. Axial/radial shadings — per-pixel function evaluation; renderers precompute a 256- or 1024-entry color LUT over t.
4. **Transparency groups and soft masks** — each non-trivial group needs an offscreen buffer the size of its bbox (clipped to the visible area), composited with the full formula; luminosity masks require rendering the mask group *plus* converting to luminance. Non-isolated groups need backdrop copies; knockout groups need per-pixel shape tracking. Nesting multiplies memory. Blending in a CMYK or ICC group space requires color conversion per group.
5. **Mesh shadings (types 4–7)** — patches are recursively subdivided until flat (to sub-pixel tolerance), then triangles Gouraud-shaded; a single complex Illustrator gradient mesh can produce millions of triangles at high zoom.
6. Pathological content: huge paths with tens of thousands of points, clip-path stacks, tiling patterns with tiny steps (e.g. 0.1 pt XStep) that force millions of cell renders.

Implication for lamina's renderer: build a **display list** from the content stream (with resources resolved, functions compiled, images lazily decoded), then rasterize tiles in parallel; bound group buffers to the intersection of bbox, clip and tile; cap subdivision depth.

---

## 13. Worked, verified example

The following content stream draws (1) a filled rectangle, (2) a dashed stroked Bézier circle, (3) an axial gradient clipped to a rectangle, and (4) a square rotated 30° about its centre at 50% opacity. The page is US Letter (612×792) with default y-up space. It was generated by a Python script, validated with `pdfinfo` (Poppler), and rendered with `pdftoppm`: the output shows a blue box top-left, a red dashed ring top-right, a yellow→purple gradient band across the middle, and a translucent green square tilted counter-clockwise below it.

```
% 1. filled rectangle (DeviceRGB)
q
0.20 0.40 0.80 rg            % nonstroking color: DeviceRGB (0.2, 0.4, 0.8)
72 600 200 120 re            % rectangle: origin (72,600), 200 wide, 120 tall (y-up!)
f                            % fill, nonzero rule
Q                            % restore → fill color back to black
% 2. stroked Bezier circle, round joins, dashed
q
0.85 0.10 0.10 RG            % stroking color
6 w                          % 6 pt line width
1 j                          % round joins
[18 6] 0 d                   % 18 on, 6 off, phase 0
500.000 660.000 m            % start at (cx+r, cy), r = 70, centre (430,660)
500.000 698.660 468.660 730.000 430.000 730.000 c   % quarter 1: κ·r = 38.66
391.340 730.000 360.000 698.660 360.000 660.000 c   % quarter 2
360.000 621.340 391.340 590.000 430.000 590.000 c   % quarter 3
468.660 590.000 500.000 621.340 500.000 660.000 c   % quarter 4
h                            % close → join at start instead of two caps
S                            % stroke
Q
% 3. axial gradient clipped to a rectangle
q
72 380 468 150 re            % clip shape
W n                          % intersect clip (nonzero), consume path without painting
/Sh1 sh                      % paint shading over the clip region
Q                            % restore → clip removed
% 4. rotated square, 30 deg about its centre (306,220), 50% opacity
q
/GS1 gs                      % ExtGState: /ca 0.5 /CA 0.5
1 0 0 1 306 220 cm           % move origin to the pivot
0.866025 0.500000 -0.500000 0.866025 0 0 cm   % rotate 30° CCW (cos, sin, −sin, cos)
0 0.6 0.3 rg                 % fill green
0 0 0 RG                     % stroke black
2 w
-60 -60 120 120 re           % square centred on the (rotated) origin
B                            % fill then stroke
Q
```

The supporting objects:

```
3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R
   /Resources << /Shading << /Sh1 5 0 R >> /ExtGState << /GS1 7 0 R >> >> >>
5 0 obj << /ShadingType 2 /ColorSpace /DeviceRGB /Coords [72 0 540 0]
           /Function 6 0 R /Extend [true true] >>
6 0 obj << /FunctionType 2 /Domain [0 1] /C0 [1 0.85 0.1] /C1 [0.5 0 0.6] /N 1 >>
7 0 obj << /Type /ExtGState /ca 0.5 /CA 0.5 >>
```

Points worth noticing:

- The gradient axis `[72 0 540 0]` is horizontal and its y is irrelevant: an axial shading is constant along lines perpendicular to the axis, and `Extend` paints beyond the endpoints. The clip determines the visible area.
- `sh` coordinates are in *current user space*; a `cm` before `sh` would transform the gradient with the clip — unlike a shading *pattern*.
- Operation 4 writes the pivot translate first and the rotation second: the rotation applies to the square's coordinates first (last `cm` = innermost).
- Because `/GS1` sets both `ca` and `CA`, the black outline is also 50% opaque; the stroke's inner half overlaps the fill and both are composited sequentially (a stroke over a translucent fill shows the overlap darker). To avoid that, wrap fill+stroke in a transparency group form with `ca 0.5` applied at `Do` time.

The generator (`make_imaging_demo.py`) is ~70 lines of plain Python: builds the stream, computes κ, writes objects, xref offsets and trailer. It is a good first unit-test fixture for lamina's writer and renderer.

---

## 14. Implications for lamina's API

What maps cleanly from a modern 2-D graphics API to PDF primitives, and what doesn't:

| API concept | PDF mapping | Notes |
|---|---|---|
| `Path` builder: moveTo, lineTo, cubicTo, quadTo, close, rect, arc, ellipse | `m l c h re` | quadTo → degree-elevated `c`; arcs → ≤90° Bézier segments with `(4/3)tan(φ/4)`; emit `v`/`y` only as an optimization |
| `fill(path, rule)`, `stroke(path, style)`, `fillStroke` | `f`/`f*`, `S`, `B`/`B*` | Fill+stroke with *different alpha* needs two paints or a group |
| `StrokeStyle {width, cap, join, miterLimit, dash}` | `w J j M d` | Emit only diffs vs modeled state |
| `save()/restore()` or scoped closure | `q … Q` | Make it scope-based to guarantee balance; track modeled state stack for diffing |
| `Transform` (affine) | `cm` | Provide explicit composition (`then`, `pre`) with documented row-vector order; reject singular |
| `clip(path, rule)` | `W n` / `W* n` | Clips only intersect; "unclip" = restore |
| `Color` | tagged by color space: Gray, RGB, CMYK, ICC(profile), Spot(name, alternate), Lab | Colors must carry their space; `cs`+`scn` emitted per change; resource-intern spaces |
| `Opacity`, `BlendMode` | ExtGState `ca/CA/BM` | Intern ExtGStates by value |
| `Layer` / `saveLayer(opacity, blend, mask)` | Form XObject with `/Group /Transparency /I true` + `gs` + `Do` | The faithful way to apply group opacity |
| `LinearGradient`, `RadialGradient` with stops | Shading type 2/3 + Type 3 stitching of Type 2 functions; `sh` inside clip | Stop alphas → luminosity soft mask; `spreadMethod: repeat/reflect` isn't native — emulate via extended stitching domain or tiling |
| `Mesh/conic gradient` | Type 4/6/7 meshes (conic: approximate via type 1 or mesh) | Expensive; rarely needed |
| `Pattern / image fill` | Tiling pattern with baked `/Matrix` | Pattern space anchored to page — re-create per transform |
| `Image` (JPEG/PNG/raw) | Image XObject, dedup by content hash; JPEG pass-through; PNG predictor pass-through; alpha → SMask | Never inline; place with `w 0 0 h x y cm` |
| `Symbol`/`Template`/`Stamp`/`importPage` | Form XObject | Own resources avoid name clashes |
| `Layer visibility` (OCG) | `/OC BDC…EMC`, `/OC` on XObjects | Catalog-level `/OCProperties` bookkeeping |
| `Text` | ch. 03 | Separate subsystem; still uses CTM, color, clip, alpha |
| `Tag/semantic marker` | `BDC/EMC` with MCID | Needed for tagged PDF/accessibility |

Writer hygiene checklist:

1. Plain decimal numbers, trimmed (`0.5`, not `0.500000`, never `5e-1`); 3–4 decimals for coordinates, 3–4 for colors.
2. Balanced `q/Q` per stream; no `q/Q/cm` inside path or text objects.
3. One resource name space per stream owner; generate names deterministically (`/Im0`, `/GS3`) for reproducible output.
4. Compress content streams with Flate; dedupe identical images, fonts, ExtGStates, color spaces, shadings.
5. Prefer ICCBased (or output intent + device spaces) when color fidelity or PDF/A matters.
6. Set a page `/Group << /S /Transparency /CS /DeviceRGB >>` when a page uses transparency.
7. Keep an internal display-list representation shared by writer and renderer so both can be tested against the same fixtures (render → compare against pdftoppm/MuPDF/pdf.js output with a tolerance).

Reader/renderer hygiene: tolerate unbalanced `Q`, operators in wrong contexts, missing resources, `F`, inline images without `/L`, operand count mismatches (pop what's needed, discard extras), and singular matrices. Real-world PDFs are messier than the spec; lenient parsing with strict writing is the rule.

---

## References

- ISO 32000-2:2020, *Document management — Portable document format — Part 2: PDF 2.0* (free via PDF Association sponsorship): https://pdfa.org/sponsored-standards/ and announcement https://pdfa.org/announcing-no-cost-access-to-iso-32000-2-pdf-2-0/
- ISO 32000-1:2008 (PDF 1.7), Adobe public copy: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf
- Adobe *PDF Reference, sixth edition (1.7)* and the *Transparency in PDF* technical note (archived): https://opensource.adobe.com/dc-acrobat-sdk-docs/
- *PostScript Language Reference Manual, 3rd ed.* (imaging model, scan conversion, shadings): https://www.adobe.com/jp/print/postscript/pdfs/PLRM.pdf
- John Warnock, "The Camelot Project" (1991): https://web.archive.org/web/20230905100828/http://www.planetpdf.com/planetpdf/pdfs/warnock_camelot.pdf
- PDF Association, *PDF 2.0 Application Note 001: Black Point Compensation*: https://pdfa.org/download-area/publications/PDF20_AN001-BPC.pdf
- PDF Association, "What's new in the updated PDF 2.0 dated revision": https://www.pdfa.org/presentation/whats-new-in-the-updated-pdf-2-0-dated-revision/
- QualityLogic, "Technical review of ISO 32000-2 (PDF 2.0)": https://www.qualitylogic.com/knowledge-center/technical-review-iso-32000-2-pdf-2-0/
- W3C, *Compositing and Blending Level 1* (blend-mode formulas shared with PDF): https://www.w3.org/TR/compositing-1/
- ICC specifications (ICC.1 / ISO 15076-1; v4 profiles): https://www.color.org/specification/ICC.1-2022-05.pdf and https://www.color.org/icc_specs2.xalter
- Spencer Mortensen, "Approximate a circle with cubic Bézier curves": https://spencermortensen.com/articles/bezier-circle/
- pdf-tools.com, "Inline images and Type 3 fonts" (PDF 2.0 inline image length): https://www.pdf-tools.com/pdf-knowledge/inline-images-type3-fonts/
- pdf.js source (canvas renderer, patterns, functions): https://github.com/mozilla/pdf.js/tree/master/src/display and https://github.com/mozilla/pdf.js/blob/master/src/core/function.js
- PDFium source: https://pdfium.googlesource.com/pdfium/
- MuPDF (fitz draw device): https://github.com/ArtifexSoftware/mupdf and https://mupdf.com/docs/
- Poppler (Splash/Cairo): https://gitlab.freedesktop.org/poppler/poppler
- Skia PDF backend (SkPDF): https://skia.org/docs/user/sample/pdf/
- PNG specification (filters/predictors, chunks): https://www.w3.org/TR/png-3/
- JPEG / JFIF and Adobe APP14 marker: https://www.w3.org/Graphics/JPEG/itu-t81.pdf

---

## Verification log

**Date:** 2026-10-28

**Sources used:** ISO 32000-1:2008 (PDF 1.7), full text (Adobe public copy, text extraction) — clauses 7.8, 7.10, 8.2–8.11, 10.3, 10.6, 11.3–11.6, Annex A (operator summary), Annex B (Type 4 operators), Annex C (implementation limits). Library of Congress format description fdd000474 (PDF 2.0) was available but has no technical detail relevant to this chapter. ISO 32000-2 (PDF 2.0) text was **not** available, so 2.0 clause/table numbers and 2.0-only features remain unverified.

**Confirmed against ISO 32000-1 (no change needed beyond citation):** 73 operators and their categories/operands (Table 51, Annex A); `cm` pre-multiplication `CTM′ = M × CTM` and the row-vector convention (§8.3.3–8.3.4), translate/scale/rotate/skew matrix forms; graphics-state parameters and initial values (Tables 52–53: line width 1.0, miter limit 10 ≈ 11.5°, RelativeColorimetric, SA false, flatness 1.0); ExtGState keys in §4.2 example (Table 58; `UseBlackPtComp` is 2.0); `i` flatness range 0–100 (Table 57); miter ratio `1/sin(φ/2)` and the 1.414 → 90° example (§8.4.3.5); dash semantics restarted per subpath (§8.4.3.6); `re` expansion, `v`/`y` control points (Table 59); path-painting operators incl. `F` obsolete (Table 60); nonzero/even-odd rules and the star/concentric-circle examples (§8.5.3.3); clip takes effect after painting, `W n` idiom (§8.5.4); `Contents` arrays split only at token boundaries (Table 30); no exponent notation in reals; BX/EX semantics (§7.8.2); resource categories (Table 33); color-space families (Table 62); initial colors set by `cs` incl. Separation/DeviceN tint 1.0 (Table 74); Indexed hival ≤ 255 and base restrictions (§8.6.6.3); Separation `/All` and `/None`; device conversion formulas (§10.3); image coordinate system and implicit matrix `[1/w 0 0 −1/h 0 1]` (§8.9.4); Decode formula `Dmin + x·(Dmax−Dmin)/(2^n−1)` (§8.9.5.2); stencil-mask Decode convention, explicit and color-key masking in raw sample values (§8.9.6); inline-image abbreviations (Tables 93–94); progressive JPEG since PDF 1.3 (§7.4.8); scan-conversion "any part of pixel" and image pixel-centre rules (§10.6.4); stroke adjustment (§10.6.5); tiling types and pattern-matrix anchoring to the parent stream's default space (§8.7.2–8.7.3); axial and radial shading math incl. "greatest s wins" (§8.7.4.5); Type 4 operator list (Annex B); basic compositing formula (§11.3.3); subtractive-space complementing (§11.3.4); separable and non-separable blend formulas (Tables 136–137, except the note on dodge/burn); soft-mask dictionary semantics, BC default black, mask coordinate space fixed at `gs` time (§11.6.5.2, Table 144); group `CS`/`I`/`K` and Luminosity-mask `CS` requirement (Table 147); optional-content rules incl. "state operators still execute when hidden" (§8.11.3); marked-content nesting with `BT/ET` (§14.6). The κ constant and circle-error figures are not in the spec; they were re-derived arithmetically and are correct.

**Corrections made (was → now):**
- §5.2 dot trick: "single-point open subpath `x y m` + `S` draws a dot with round caps; sanctioned for caps 1 and 2" → a trailing single-point `m` paints nothing; only *degenerate* subpaths (closed single point, or coincident points) paint a dot, and only with round caps (1), not projecting caps (2) (ISO 32000-1 §8.5.3.2).
- §7.2 soft-mask images: "effective alpha is SMask × ca, with the graphics-state soft mask also applying" → the image's SMask/Mask *overrides* the graphics-state soft mask (at most one mask per compositing operation); also noted DeviceGray is required and Matte requires matching dimensions (Table 89, §11.6.4.3, Table 145).
- §10.1: "qs includes ca, any image SMask and the graphics-state soft mask, all multiplied" → ca × one soft mask (image SMask overrides gstate mask) (§11.6.4.3).
- §10.1 non-separable blending in CMYK: "K treated with Normal-like rules" → K from backdrop for Hue/Saturation/Color, from source for Luminosity (§11.3.5).
- §10.2 ColorDodge/ColorBurn: added note that PDF 1.7's formulas lack the W3C `Cb = 0` / `Cb = 1` special cases; "identical to W3C" → "match apart from these corner cases" (Table 136).
- §2.1 table: `SC`/`sc` "ICC-ok spaces" → not ICCBased (ICCBased needs `SCN`/`scn`) (Table 74).
- §2.1: operator count 73 now cited as verified (Table 51, Annex A); 1.7 table number given (Table 51; the "ISO 32000-2 Table 50" citation is unverified).
- §2.2: "PDF 2.0 forbids q/Q/cm in text objects" → already disallowed by Figure 9 in PDF 1.7 (§8.2).
- §2.2: q nesting depth 28 → confirmed, cited as Annex C Table C.1 with the spec's note that it derives from PostScript gsave/grestore.
- §3.1 UserUnit: "Acrobat caps page size at 14,400 units" → spec-recommended maximum 14,400 × 14,400 (Annex C.2).
- §4.1: added 1.7 table numbers (Tables 52–53).
- §5.5: scan-conversion clause "§10.7" → ISO 32000-1 §10.6.4 (2.0 number unverified); zero-width strokes are implementation-defined and may paint fewer pixels (only zero-area *fills* are guaranteed pixels).
- §6.2: added PDF 1.7's ICC reference (ICC.1:2004-10, Table 67).
- §6.3: Default color spaces "the given CIE-based space" → any space except Lab, Indexed, Pattern with same component count (§8.6.5.6); device-conversion clause "§10.4" → ISO 32000-1 §10.3.
- §7.1: image `ColorSpace` "optional for image masks" → forbidden for image masks (Table 89); default Decode for Lab/ICCBased uses `Range` (Table 90).
- §7.3 inline images: "Adobe's guidance: a few KB" → normative "shall be used only for small images (4 KB or less)" (§8.9.7); `L`/`Length` labelled PDF 2.0 only (1.7 ignores Length); clarified unnamed `CS` restrictions.
- §9.3 mesh streams: bit depths "1–32 / 1–16" → the enumerated allowed values (Tables 82–84).
- §9.4: Type 4 "100 entries is the classic limit" → it is the spec's minimum (§7.10.5.1).
- §1.1: noted PostScript XObjects still exist (print-only) in PDF 1.7 (§8.8.2).

**Still unverified (PDF 2.0 text unavailable, or outside the spec):**
- All ISO 32000-2 clause/table/figure numbers (Table 50, Tables 51–52, Figure 9, §10.4, §10.7, §11.3).
- PDF 2.0-only claims: inline-image `/L` key; `UseBlackPtComp`; `HTO`; page-level output intents; TR/TR2 deprecation; removal of PostScript XObjects; `AF` on images; ICC.1:2010 / v4.3 reference; whether 2.0 changed ColorDodge/ColorBurn edge cases; "no new operators in 2.0".
- Renderer internals (§12: pdf.js, PDFium, MuPDF, Poppler/Splash, Ghostscript, Skia, Quartz), Acrobat behaviour, PDF/A and PDF/X details (ISO 19005 / 15930), PNG/JPEG embedding details, and history (Camelot memo).
- The §13 worked example's rendering results (not re-run).
