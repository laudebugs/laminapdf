// The RenderBackend contract (todo C2c): one suite both web adapters run, in Node and in
// Chromium, Firefox and WebKit. It checks what ADR 0004 promises every backend: the page sizes
// lamina reports, bounded tiles that stitch into the whole page, view rotations, typed errors,
// AbortSignal handling, bounded memory and, where the runner can decode PNGs (browsers), C2b's
// render goldens of the same backend.
import { describe, expect, it } from "vitest";
import {
  bytesSource,
  isRenderError,
  type RenderBackend,
  type RenderDocument,
  type RenderedTile,
  type RenderSource,
} from "../src/index.js";

export interface Decoded {
  readonly width: number;
  readonly height: number;
  readonly pixels: Uint8ClampedArray;
}

export interface ContractOptions {
  /** A fresh backend (its module initialised). */
  readonly backend: () => Promise<RenderBackend>;
  /** Reads a repository file, `fixtures/…`. */
  readonly read: (path: string) => Promise<Uint8Array>;
  /** Where PNGs can be decoded (browsers): C2b's goldens are compared at `minSsim`. */
  readonly goldens?: { readonly decode: (png: Uint8Array) => Promise<Decoded>; readonly minSsim: number };
  /** The engine's WebAssembly memory size in bytes, for the memory-bound test. */
  readonly memory?: () => number;
}

/** Files whose page 1 is compared with the goldens. */
const GOLDEN_FILES = ["hello.pdf", "imaging-demo.pdf"];

function thrown(f: () => unknown): unknown {
  try {
    f();
  } catch (e) {
    return e;
  }
  throw new Error("expected the call to throw");
}

/** Pixels whose channels differ by more than 8. */
function differing(a: Uint8ClampedArray, b: Uint8ClampedArray): number {
  let n = 0;
  for (let i = 0; i < a.length; i += 4) {
    for (let c = 0; c < 4; c++) {
      if (Math.abs((a[i + c] ?? 0) - (b[i + c] ?? 0)) > 8) {
        n++;
        break;
      }
    }
  }
  return n;
}

/** Mean SSIM of luma over 8 × 8 windows, stride 4 (the Rust golden test's metric). */
export function ssim(a: Decoded, b: Decoded): number {
  const luma = (d: Decoded) => {
    const out = new Float64Array(d.width * d.height);
    for (let i = 0; i < out.length; i++) {
      out[i] = 0.299 * (d.pixels[i * 4] ?? 0) + 0.587 * (d.pixels[i * 4 + 1] ?? 0) + 0.114 * (d.pixels[i * 4 + 2] ?? 0);
    }
    return out;
  };
  const [la, lb] = [luma(a), luma(b)];
  const { width: w, height: h } = a;
  const c1 = (0.01 * 255) ** 2;
  const c2 = (0.03 * 255) ** 2;
  const size = Math.min(8, w, h);
  let total = 0;
  let windows = 0;
  for (let y = 0; y + size <= h; y += 4) {
    for (let x = 0; x + size <= w; x += 4) {
      let [sa, sb, saa, sbb, sab] = [0, 0, 0, 0, 0];
      for (let yy = y; yy < y + size; yy++) {
        for (let xx = x; xx < x + size; xx++) {
          const p = la[yy * w + xx] ?? 0;
          const q = lb[yy * w + xx] ?? 0;
          sa += p;
          sb += q;
          saa += p * p;
          sbb += q * q;
          sab += p * q;
        }
      }
      const n = size * size;
      const [ma, mb] = [sa / n, sb / n];
      const [va, vb, cov] = [saa / n - ma * ma, sbb / n - mb * mb, sab / n - ma * mb];
      total += ((2 * ma * mb + c1) * (2 * cov + c2)) / ((ma * ma + mb * mb + c1) * (va + vb + c2));
      windows++;
    }
  }
  return windows === 0 ? 1 : total / windows;
}

/** Registers the contract suite for one backend. */
export function renderContract(options: ContractOptions): void {
  const { read } = options;
  let backend: RenderBackend;
  const open = async (path: string, password?: string): Promise<RenderDocument> =>
    backend.open(bytesSource(await read(path)), password === undefined ? {} : { password });

  describe("RenderBackend contract", () => {
    it("opens every corpus file with lamina's page count and first page size", async () => {
      backend = await options.backend();
      const expected = JSON.parse(new TextDecoder().decode(await read("fixtures/corpus/expected.json"))) as Record<
        string,
        { page_count: number; first_page_mediabox: number[]; encrypted: boolean }
      >;
      for (const [file, facts] of Object.entries(expected)) {
        using doc = await open(`fixtures/corpus/${file}`);
        expect(doc.pageCount, file).toBe(facts.page_count);
        const [x0 = 0, y0 = 0, x1 = 0, y1 = 0] = facts.first_page_mediabox;
        const page = doc.page(0);
        // The displayed size is the CropBox with /Rotate; every corpus file is upright and uncropped.
        expect([Math.round(page.width), Math.round(page.height)], file).toEqual([
          Math.round(x1 - x0),
          Math.round(y1 - y0),
        ]);
      }
    });

    it("renders white where hello.pdf is blank", async () => {
      using doc = await open("fixtures/corpus/hello.pdf");
      const tile = doc.render({ page: 0, scale: 1, clip: { x: 0, y: 0, width: 8, height: 8 } });
      expect([tile.width, tile.height, tile.pixels.length]).toEqual([8, 8, 256]);
      expect(tile.pixels.every((b) => b === 255)).toBe(true);
    });

    it("stitches four tiles into the whole page", async () => {
      using doc = await open("fixtures/corpus/imaging-demo.pdf");
      const whole = doc.render({ page: 0, scale: 0.5 });
      const [w, h] = [whole.width, whole.height];
      const [tw, th] = [Math.ceil(w / 2), Math.ceil(h / 2)];
      const stitched = new Uint8ClampedArray(whole.pixels.length);
      for (const [x, y] of [
        [0, 0],
        [tw, 0],
        [0, th],
        [tw, th],
      ] as const) {
        const clip = { x, y, width: Math.min(tw, w - x), height: Math.min(th, h - y) };
        const tile = doc.render({ page: 0, scale: 0.5, clip });
        for (let row = 0; row < tile.height; row++) {
          stitched.set(tile.pixels.subarray(row * tile.width * 4, (row + 1) * tile.width * 4), ((y + row) * w + x) * 4);
        }
      }
      expect(differing(whole.pixels, stitched)).toBe(0);
    });

    it("renders a quarter turn as the upright render rotated, with and without widgets", async () => {
      using doc = await open("fixtures/corpus/imaging-demo.pdf");
      for (const formWidgets of [true, false]) {
        const up = doc.render({ page: 0, scale: 0.5, flags: { formWidgets } });
        const turned: RenderedTile = doc.render({ page: 0, scale: 0.5, rotation: 90, flags: { formWidgets } });
        expect([turned.width, turned.height]).toEqual([up.height, up.width]);
        const expected = new Uint8ClampedArray(up.pixels.length);
        for (let y = 0; y < up.height; y++) {
          for (let x = 0; x < up.width; x++) {
            const src = (y * up.width + x) * 4;
            const dst = (x * turned.width + (up.height - 1 - y)) * 4;
            expected.set(up.pixels.subarray(src, src + 4), dst);
          }
        }
        expect(differing(expected, turned.pixels) / (up.width * up.height)).toBeLessThan(0.02);
      }
    });

    it("throws typed errors for passwords, pages and oversized requests", async () => {
      expect(
        isRenderError(
          thrown(() => backend.open(bytesSource(new TextEncoder().encode("nope")))),
          "render-failed",
        ),
      ).toBe(true);
      const r4 = await read("fixtures/encrypted/r4-aes-128-user.pdf");
      expect(
        isRenderError(
          thrown(() => backend.open(bytesSource(r4))),
          "password-required",
        ),
      ).toBe(true);
      expect(
        isRenderError(
          thrown(() => backend.open(bytesSource(r4), { password: "nope" })),
          "wrong-password",
        ),
      ).toBe(true);
      using encrypted = backend.open(bytesSource(r4), { password: "user-pw" });
      expect(encrypted.render({ page: 0, scale: 0.25 }).width).toBeGreaterThan(0);
      using doc = await open("fixtures/corpus/hello.pdf");
      expect(
        isRenderError(
          thrown(() => doc.page(doc.pageCount)),
          "page-index-out-of-range",
        ),
      ).toBe(true);
      expect(
        isRenderError(
          thrown(() => doc.render({ page: 0, scale: 1000 })),
          "render-too-large",
        ),
      ).toBe(true);
    });

    it("throws data-pending with the ranges a partial source lacks", async () => {
      const bytes = await read("fixtures/corpus/text-example.pdf");
      // Only the first and last KiB.
      const partial: RenderSource = {
        size: bytes.length,
        readSync: (offset, length) => {
          const end = Math.min(bytes.length, offset + length);
          return end <= 1024 || offset >= bytes.length - 1024 ? bytes.subarray(offset, end) : undefined;
        },
      };
      const e = thrown(() => {
        const doc = backend.open(partial);
        try {
          doc.render({ page: 0, scale: 0.25 });
        } finally {
          doc.close();
        }
      });
      expect(isRenderError(e, "data-pending")).toBe(true);
      expect((e as { ranges?: unknown[] }).ranges?.length ?? 0).toBeGreaterThan(0);
    });

    it("rejects an aborted request with AbortError and a closed document with disposed", async () => {
      const doc = await open("fixtures/corpus/hello.pdf");
      const controller = new AbortController();
      controller.abort();
      const e = thrown(() => doc.render({ page: 0, scale: 1 }, { signal: controller.signal }));
      expect(e instanceof DOMException && e.name === "AbortError").toBe(true);
      doc.close();
      doc.close();
      expect(
        isRenderError(
          thrown(() => doc.render({ page: 0, scale: 1 })),
          "disposed",
        ),
      ).toBe(true);
    });

    it("keeps memory bounded over 50 renders", async () => {
      const memory = options.memory;
      if (memory === undefined) return;
      using doc = await open("fixtures/corpus/imaging-demo.pdf");
      for (let i = 0; i < 5; i++) doc.render({ page: 0, scale: 1 });
      const warm = memory();
      for (let i = 0; i < 50; i++) doc.render({ page: 0, scale: 1 });
      // Linear memory never shrinks: a leak of one 612 × 792 tile per render would add ~95 MB.
      expect(memory() - warm).toBeLessThanOrEqual(4 * 1024 * 1024);
    });

    if (options.goldens !== undefined) {
      const { decode, minSsim } = options.goldens;
      it(`matches C2b's goldens of page 1 (SSIM ≥ ${minSsim})`, async () => {
        for (const file of GOLDEN_FILES) {
          using doc = await open(`fixtures/corpus/${file}`);
          const tile = doc.render({ page: 0, scale: 96 / 72 });
          const stem = file.replace(/\.pdf$/, "");
          const golden = await decode(await read(`fixtures/golden/render/${backend.name}/${stem}-p1.png`));
          expect([tile.width, tile.height], file).toEqual([golden.width, golden.height]);
          expect(ssim(tile, golden), file).toBeGreaterThanOrEqual(minSsim);
        }
      });
    }
  });
}
