---
title: "Test the PDFs you generate"
description: "Compare the PDFs your code makes with reference PDFs you approved, byte for byte, and draw both files' pages to see what changed."
version: "0.3.0"
url: "https://docs.laminapdf.com/guides/test-generated-pdfs/"
---

# Test the PDFs you generate

Compare the PDFs your code makes with reference PDFs you approved, byte for byte, and draw both files' pages to see what changed.

You'll write a test that fails when a PDF your code makes changes. The test compares the PDF's bytes with a reference PDF you opened and approved. When the bytes differ, a second example draws both files' pages to show where they look different.

## Why the bytes repeat

Lamina PDF never reads a clock or picks a random value while it writes. With the same code, data, options and Lamina PDF version, Rust and every JavaScript runtime write the same bytes. Three values usually come from your app rather than from the content (the file ID, the producer and the creation date), so a test fixes them.

Your own code can vary the bytes too, and a failing test should point there first:

- a date taken from the clock, such as an invoice date or `new Date()` in a footer;
- records read in a different order, such as rows from a database without `ORDER BY`;
- a different font file: an update to the font changes the embedded subset.

## Compare with a reference PDF

The example below holds the options a test passes and a function that compares a PDF with its reference. The function answers `same`, `created` or `changed`, and your test runner checks the answer, so it works in Vitest, Jest, `node:test` or `cargo test`:

**TypeScript**

`packages/sdk/examples/snapshot-test.ts`:

```ts
// Checks that a PDF your code makes is byte for byte the reference PDF you approved.
//
// Build the document with `TEST_OPTIONS`, then let your test runner check the answer:
//
//   expect(compareWithReference(invoice(order, TEST_OPTIONS), "references/invoice.pdf")).toBe("same");
//
// The first run fails with `created` and writes the reference: open it, and commit it once it
// looks right. When the bytes change later, the new PDF is written beside the reference for
// review; pass `{ update: true }` to accept it.
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { basename, dirname, extname, join } from "node:path";
import type { BuilderOptions } from "@laminapdf/sdk";

/**
 * What a test passes in place of the clock and a random file ID. With these fixed, the same code
 * and data give the same bytes on every run, machine and runtime.
 */
export const TEST_OPTIONS: BuilderOptions = {
  // The file ID: any 32 hex digits, as long as they stay the same.
  id: "00112233445566778899aabbccddeeff",
  // The Info dictionary's /Producer: your app's name, without a version that changes.
  producer: "my-app tests",
  // The Info dictionary's /CreationDate, in PDF date form.
  creationDate: "D:20260101000000Z",
};

/** `same`: the bytes match. `created`: there was no reference, so one was written. `changed`: they differ. */
export type Outcome = "same" | "created" | "changed";

export interface CompareOptions {
  /** Replace the reference with the new bytes instead of writing them beside it. */
  readonly update?: boolean;
}

/**
 * Compares `bytes` with the reference PDF at `reference`. A changed PDF is written to
 * `<name>.new.pdf` next to the reference, or replaces it when `update` is set.
 */
export function compareWithReference(bytes: Uint8Array, reference: string, options: CompareOptions = {}): Outcome {
  const pending = join(dirname(reference), `${basename(reference, extname(reference))}.new.pdf`);
  if (!existsSync(reference)) {
    mkdirSync(dirname(reference), { recursive: true });
    writeFileSync(reference, bytes);
    return "created";
  }
  if (sameBytes(readFileSync(reference), bytes)) {
    rmSync(pending, { force: true });
    return "same";
  }
  if (options.update) {
    writeFileSync(reference, bytes);
    rmSync(pending, { force: true });
  } else {
    writeFileSync(pending, bytes);
  }
  return "changed";
}

function sameBytes(a: Uint8Array, b: Uint8Array): boolean {
  return a.length === b.length && a.every((byte, i) => byte === b[i]);
}
```

**Rust**

`crates/laminapdf/examples/snapshot_test.rs`:

````rust
//! Checks that a PDF your code makes is byte for byte the reference PDF you approved.
//!
//! Build the document with `test_options()`, then assert on the answer in a test:
//!
//! ```ignore
//! let bytes = invoice(&order, test_options()?)?;
//! assert_eq!(compare_with_reference(&bytes, Path::new("references/invoice.pdf"), false)?, Outcome::Same);
//! ```
//!
//! The first run fails with `Created` and writes the reference: open it, and commit it once it
//! looks right. When the bytes change later, the new PDF is written beside the reference for
//! review; pass `update: true` to accept it. Run on its own, the example compares two files:
//!
//! ```sh
//! cargo run -p laminapdf --example snapshot_test -- new.pdf reference.pdf
//! ```

use std::error::Error;
use std::path::{Path, PathBuf};

use laminapdf::BuildOptions;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let (Some(new), Some(reference)) = (args.next(), args.next()) else {
        return Err("usage: snapshot_test <new.pdf> <reference.pdf>".into());
    };
    let outcome = compare_with_reference(&std::fs::read(new)?, Path::new(&reference), false)?;
    println!("{outcome:?}");
    Ok(())
}

/// What a test passes in place of the clock and a random file ID. With these fixed, the same code
/// and data give the same bytes on every run, machine and runtime.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "your tests call it; `main` only compares two files")
)]
pub(crate) fn test_options() -> Result<BuildOptions, Box<dyn Error>> {
    /// The file ID: any 16 bytes, as long as they stay the same.
    const FILE_ID: [u8; 16] = [
        0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
    ];
    /// The Info dictionary's `/Producer`: your app's name, without a version that changes.
    const PRODUCER: &str = "my-app tests";
    /// The Info dictionary's `/CreationDate`, in PDF date form.
    const CREATED: &str = "D:20260101000000Z";
    Ok(BuildOptions::new()
        .with_id(FILE_ID)
        .with_producer(PRODUCER)
        .with_creation_date(CREATED.parse()?))
}

/// `Same`: the bytes match. `Created`: there was no reference, so one was written. `Changed`:
/// they differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome {
    Same,
    Created,
    Changed,
}

/// Compares `bytes` with the reference PDF at `reference`. A changed PDF is written to
/// `<name>.new.pdf` next to the reference, or replaces it when `update` is set.
pub(crate) fn compare_with_reference(bytes: &[u8], reference: &Path, update: bool) -> std::io::Result<Outcome> {
    let pending = pending_path(reference);
    if !reference.exists() {
        if let Some(folder) = reference.parent() {
            std::fs::create_dir_all(folder)?;
        }
        std::fs::write(reference, bytes)?;
        return Ok(Outcome::Created);
    }
    if std::fs::read(reference)? == bytes {
        remove_if_present(&pending)?;
        return Ok(Outcome::Same);
    }
    if update {
        std::fs::write(reference, bytes)?;
        remove_if_present(&pending)?;
    } else {
        std::fs::write(&pending, bytes)?;
    }
    Ok(Outcome::Changed)
}

/// `invoice.pdf` → `invoice.new.pdf`, in the same folder.
fn pending_path(reference: &Path) -> PathBuf {
    let stem = reference.file_stem().unwrap_or_default().to_string_lossy();
    reference.with_file_name(format!("{stem}.new.pdf"))
}

fn remove_if_present(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        result => result,
    }
}
````

On the first run there is no reference yet, so the function writes one, answers `created`, and the test fails. Open that PDF before you commit it: the test only proves that later runs match it, so it has to be right. After that, every run answers `same` until the output changes. A reference you forget to commit fails the test on every other machine too, since a fresh checkout has none.

When the bytes change, the reference stays as it was and the new PDF goes beside it as `<name>.new.pdf`. Open both, and if the change is the one you meant, run the tests again with `update` set, for example from an environment variable such as `UPDATE_REFERENCES=1`. Commit the new reference with the change that caused it.

## See where a changed PDF looks different

Different bytes don't always mean different pages: a reordered dictionary or a new producer changes the file but not how it looks. This example draws each page of both files and counts the pixels that differ:

`packages/sdk/examples/compare-pages.ts`:

```ts
// Draws each page of two PDFs and counts the pixels that differ, to show where a changed PDF looks different.
//
// Both files are drawn by the same renderer, so its own quirks cancel out. In Node, load it with
// the module's bytes:
//
//   const wasm = fileURLToPath(import.meta.resolve("@laminapdf/wasm/lamina_pdf_render_hayro.wasm"));
//   const renderer = await loadRenderer(readFileSync(wasm));
import {
  bytesSource,
  DEFAULT_LIMITS,
  type RenderBackend,
  type RenderDocument,
  type RenderedTile,
} from "@laminapdf/render";
import { initHayro } from "@laminapdf/render-hayro";

/** Pixels per point: 2 draws at 144 dpi, enough to see a glyph that moved. Pages too large for the renderer's limits at this scale are drawn smaller. */
const SCALE = 2;
/** How far a colour channel (0–255) may move before the pixel counts as changed: absorbs smoothing noise. */
const TOLERANCE = 8;

/** A rectangle in page points, from the page's top-left corner. */
export interface Box {
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
}

/** One page: how many pixels differ, and the box around them (`null` when none do). */
export interface PageDifference {
  readonly page: number;
  readonly pixels: number;
  readonly box: Box | null;
}

export interface Comparison {
  /** The page counts of the first and second file; only the pages both have are compared. */
  readonly pageCounts: readonly [number, number];
  readonly pages: readonly PageDifference[];
}

/** Loads the renderer from the module's bytes, a URL, or (in a browser) the file next to its code. */
export function loadRenderer(wasm?: Uint8Array | URL): Promise<RenderBackend> {
  return initHayro(wasm);
}

/** Compares every page the two PDFs share. */
export function comparePages(renderer: RenderBackend, before: Uint8Array, after: Uint8Array): Comparison {
  using a = renderer.open(bytesSource(before));
  using b = renderer.open(bytesSource(after));
  const shared = Math.min(a.pageCount, b.pageCount);
  const pages = Array.from({ length: shared }, (_, page) => comparePage(a, b, page));
  return { pageCounts: [a.pageCount, b.pageCount], pages };
}

function comparePage(a: RenderDocument, b: RenderDocument, page: number): PageDifference {
  const scale = Math.min(fittingScale(a, page), fittingScale(b, page));
  const first = a.render({ page, scale });
  const second = b.render({ page, scale });
  const width = Math.max(first.width, second.width);
  const height = Math.max(first.height, second.height);
  let pixels = 0;
  let [left, top, right, bottom] = [width, height, -1, -1];
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      if (!differs(first, second, x, y)) continue;
      pixels++;
      [left, top] = [Math.min(left, x), Math.min(top, y)];
      [right, bottom] = [Math.max(right, x), Math.max(bottom, y)];
    }
  }
  const box =
    pixels === 0
      ? null
      : { x: left / scale, y: top / scale, width: (right - left + 1) / scale, height: (bottom - top + 1) / scale };
  return { page, pixels, box };
}

/** `SCALE`, or less when the page would pass the renderer's pixel or side limit at it. */
function fittingScale(doc: RenderDocument, page: number): number {
  const { width, height } = doc.page(page);
  const byArea = Math.sqrt(DEFAULT_LIMITS.maxRenderPixels / (width * height));
  const bySide = DEFAULT_LIMITS.maxRenderSide / Math.max(width, height);
  return Math.min(SCALE, byArea, bySide);
}

/** Whether the pixel at (x, y) differs; a pixel only one page has always does. */
function differs(first: RenderedTile, second: RenderedTile, x: number, y: number): boolean {
  if (x >= first.width || y >= first.height || x >= second.width || y >= second.height) return true;
  const i = (y * first.width + x) * 4;
  const j = (y * second.width + x) * 4;
  for (let channel = 0; channel < 4; channel++) {
    const delta = Math.abs((first.pixels[i + channel] ?? 0) - (second.pixels[j + channel] ?? 0));
    if (delta > TOLERANCE) return true;
  }
  return false;
}
```

Each page gets one of two outcomes. With no differing pixels, the file's structure changed but its pages look the same. With some, the box shows where on the page, in points from the top-left corner. The same renderer draws both files, so its own small differences from other viewers cancel out.

To read the change itself, build with `compress: false` (`with_compress(false)` in Rust). The content streams then stay readable text, and a diff tool shows what moved. In Rust, the `laminapdf-render` crate's `hayro` feature draws pages the same way.

## Upgrade Lamina PDF

Pin the exact Lamina PDF version, so an upgrade happens only when you choose it. A release that changes the bytes written for the same input lists that change first in its changelog. Upgrade in a change of its own, regenerate the references with `update` set, and look at the page comparison before you commit them.

## Keep fonts and edits repeatable

Fonts need the same care. The embedded subset depends on the font file's exact bytes, so keep the font your test uses in your own repository rather than loading it from the system. Edited files repeat too: opening a PDF, applying the same edits and saving writes the same bytes each time.

The [stability policy](../../pages/reference/stability.md) states the promise these tests rely on: the same input gives the same bytes everywhere.
