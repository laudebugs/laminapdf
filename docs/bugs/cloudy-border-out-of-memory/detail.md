# A cloudy border on a huge square ran out of memory

[Resolution](resolution.md).

## What we saw

The nightly lane's `Fuzz ops_json` job failed on `main` on 2026-11-17 ([run 44077876849](https://github.com/laudebugs/laminapdf/actions/runs/44077876849/job/134020498538)). AddressSanitizer stopped the target when it tried to allocate 0x1b98410ce0 bytes, about 118 GB:

```
==5774==ERROR: AddressSanitizer: out of memory: allocator is trying to allocate 0x1b98410ce0 bytes
```

The input was a single op that adds a square annotation with a cloudy border, 44 billion points wide:

```json
{ "op": "addAnnot", "args": { "page": "p3.0", "spec": { "type": "shape", "kind": "square",
  "rect": { "x0": 44444444300, "y0": 400, "x1": 499, "y1": 460 }, "interior": [1, 1, 0.6],
  "border": { "style": { "type": "cloudy", "intensity": 1 } } } } }
```

Any caller of the op could ask for the same appearance, since the web and desktop readers and the SDK all draw annotation appearances with `laminapdf-build`.

## How to reproduce

Apply that op to `fixtures/corpus/hello.pdf`, whose page `p3.0` it names, through `laminapdf-save`'s `ops_json` fuzz entry, as the fuzz target does. The input is kept as `fixtures/regressions/ops_json/71c665b8750d7d6c36607fe24d5385d8cd73e0f7`.

## Cause

The op is valid: its corners are finite and the square has an area, which is all `Shape::is_valid` asks (`crates/laminapdf-ops/src/annot/drawing.rs`). PDF puts no limit on an annotation's size.

The appearance is drawn by `cloud()` in `crates/laminapdf-build/src/annot/appearance/drawing/cloud.rs`. It spaced one bump every 4 + 8 × intensity points around the outline, so the number of bumps grew with the outline's length, with nothing to stop it. The 88-billion-point perimeter asked for about 7.4 billion bumps of 12 points, and `spaced()` reserved a vector of that many points before drawing any. Each bump also writes two Bézier curves into the content stream, so even a smaller outline that fitted in memory could write an appearance stream of gigabytes.

The rule was the bug, not the input: no ordinary annotation needs more than a few thousand bumps. A0, the largest of ISO 216's A sizes, takes about 2,900 at the smallest bump Lamina PDF draws, 4 points across.

The same code failed differently on an outline whose length overflows `f64`, such as a square from −f64::MAX/2 to f64::MAX/2: casting the infinite bump count to `usize` saturated it to `usize::MAX`, and reserving that many points panicked with a capacity overflow.
