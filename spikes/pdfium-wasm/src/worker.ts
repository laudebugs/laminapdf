// The spike's one module worker: PDFium-WASM (@embedpdf/pdfium) and lamina's WebAssembly
// (@lamina/pdf, local build) side by side. The page sends it files; it measures and answers with
// plain JSON (and the rendered page as a transferred ImageBitmap).
import { init as initPdfium, type WrappedPdfiumModule } from "@embedpdf/pdfium";
import pdfiumWasmUrl from "@embedpdf/pdfium/pdfium.wasm?url";
import { callbackSource, Document, init as initLamina, wasmMemory } from "@lamina/pdf";

type Pdfium = WrappedPdfiumModule;

let pdfium: Pdfium | undefined;

/** PDFium's linear memory (Emscripten exports it as `memory`). */
function heap(p: Pdfium): WebAssembly.Memory {
  return (p.pdfium.wasmExports as unknown as { memory: WebAssembly.Memory }).memory;
}

function memory(): Record<string, number> {
  const out: Record<string, number> = { ...wasmMemory() };
  if (pdfium !== undefined) out.pdfium = heap(pdfium).buffer.byteLength;
  return out;
}

/** Instantiates both modules in parallel; each one's time and the combined time. */
async function boot(): Promise<Record<string, number>> {
  const start = performance.now();
  let laminaMs = 0;
  let pdfiumMs = 0;
  await Promise.all([
    (async () => {
      const t = performance.now();
      await initLamina();
      laminaMs = performance.now() - t;
    })(),
    (async () => {
      const t = performance.now();
      const wasmBinary = await (await fetch(pdfiumWasmUrl)).arrayBuffer();
      pdfium = await initPdfium({ wasmBinary });
      pdfium.PDFiumExt_Init();
      pdfiumMs = performance.now() - t;
    })(),
  ]);
  return { laminaMs, pdfiumMs, combinedMs: performance.now() - start };
}

/** Copies `bytes` into PDFium's heap and opens them; the pointer stays alive with the document. */
function pdfiumOpen(p: Pdfium, bytes: Uint8Array, password = ""): { doc: number; ptr: number } {
  const ptr = p.pdfium.wasmExports.malloc(bytes.length);
  new Uint8Array(heap(p).buffer, ptr, bytes.length).set(bytes);
  const doc = p.FPDF_LoadMemDocument(ptr, bytes.length, password);
  if (doc === 0) {
    p.pdfium.wasmExports.free(ptr);
    throw new Error(`PDFium could not open the file (FPDF_GetLastError ${p.FPDF_GetLastError()})`);
  }
  return { doc, ptr };
}

/**
 * Opens `file` in PDFium without copying it into the heap: FPDF_LoadCustomDocument with a
 * getBlock callback that reads File.slice() synchronously (FileReaderSync, workers only).
 */
function pdfiumOpenCustom(p: Pdfium, file: File, password = ""): { doc: number; release: () => void } {
  const reader = new FileReaderSync();
  const getBlock = p.pdfium.addFunction((_param: number, position: number, buf: number, size: number) => {
    const chunk = new Uint8Array(reader.readAsArrayBuffer(file.slice(position, position + size)));
    new Uint8Array(heap(p).buffer, buf, chunk.length).set(chunk);
    return chunk.length === size ? 1 : 0;
  }, "iiiii");
  // FPDF_FILEACCESS: m_FileLen (unsigned long), m_GetBlock, m_Param: 12 bytes on wasm32.
  const access = p.pdfium.wasmExports.malloc(12);
  p.pdfium.setValue(access, file.size, "i32");
  p.pdfium.setValue(access + 4, getBlock, "i32");
  p.pdfium.setValue(access + 8, 0, "i32");
  const doc = p.FPDF_LoadCustomDocument(access, password);
  if (doc === 0) throw new Error(`PDFium could not open the file (FPDF_GetLastError ${p.FPDF_GetLastError()})`);
  return {
    doc,
    release: () => {
      p.FPDF_CloseDocument(doc);
      p.pdfium.wasmExports.free(access);
      p.pdfium.removeFunction(getBlock);
    },
  };
}

const FPDF_ANNOT = 0x01;
const FPDF_REVERSE_BYTE_ORDER = 0x10; // RGBA instead of BGRA, so ImageData takes it as it is

interface RenderTimes {
  renderMs: number;
  copyMs: number;
  bitmapMs: number;
  width: number;
  height: number;
}

/** Renders page `index` at `scale` × 72 dpi: PDFium bitmap → ImageData → ImageBitmap. */
async function render(p: Pdfium, doc: number, index: number, scale: number): Promise<{ times: RenderTimes; bitmap: ImageBitmap }> {
  const page = p.FPDF_LoadPage(doc, index);
  if (page === 0) throw new Error(`PDFium could not load page ${index}`);
  const width = Math.max(1, Math.floor(p.FPDF_GetPageWidthF(page) * scale));
  const height = Math.max(1, Math.floor(p.FPDF_GetPageHeightF(page) * scale));
  const t0 = performance.now();
  const bmp = p.FPDFBitmap_Create(width, height, 1);
  p.FPDFBitmap_FillRect(bmp, 0, 0, width, height, 0xffffffff);
  p.FPDF_RenderPageBitmap(bmp, page, 0, 0, width, height, 0, FPDF_ANNOT | FPDF_REVERSE_BYTE_ORDER);
  const t1 = performance.now();
  const ptr = p.FPDFBitmap_GetBuffer(bmp);
  const stride = p.FPDFBitmap_GetStride(bmp);
  const pixels = new Uint8ClampedArray(width * height * 4);
  const src = new Uint8Array(heap(p).buffer, ptr, stride * height);
  for (let y = 0; y < height; y++) pixels.set(src.subarray(y * stride, y * stride + width * 4), y * width * 4);
  const t2 = performance.now();
  const bitmap = await createImageBitmap(new ImageData(pixels, width, height));
  const t3 = performance.now();
  p.FPDFBitmap_Destroy(bmp);
  p.FPDF_ClosePage(page);
  return { times: { renderMs: t1 - t0, copyMs: t2 - t1, bitmapMs: t3 - t2, width, height }, bitmap };
}

function stats(values: number[]): { median: number; p95: number; n: number } {
  const s = [...values].sort((a, b) => a - b);
  const at = (q: number) => s[Math.min(s.length - 1, Math.floor(q * s.length))] ?? 0;
  return { median: at(0.5), p95: at(0.95), n: s.length };
}

interface BenchRequest {
  file: File;
  password?: string;
  /**
   * How the file reaches the engines: `copy` (arrayBuffer, then a copy into each module) or
   * `custom` (no whole copy: PDFium reads blocks through FPDF_LoadCustomDocument over
   * File.slice(), lamina ranges through openRange over File.slice()).
   */
  strategy?: "copy" | "custom";
  /** Render every page (scroll-through) instead of a sample. */
  allPages?: boolean;
  /** Also run the reload-after-incremental-save cycle (copy strategy only). */
  reload?: boolean;
}

interface Opened {
  pdfiumDoc: number;
  lamina: Document;
  close: () => void;
}

async function openBoth(p: Pdfium, req: BenchRequest): Promise<Opened & { bytes?: Uint8Array }> {
  const { file, password = "" } = req;
  const options = password === "" ? {} : { password };
  if (req.strategy === "custom") {
    const custom = pdfiumOpenCustom(p, file, password);
    const read = (offset: number, length: number) =>
      file.slice(offset, offset + length).arrayBuffer().then((b) => new Uint8Array(b));
    const lamina = await Document.openRange(callbackSource(file.size, read), options);
    return {
      pdfiumDoc: custom.doc,
      lamina,
      close: () => {
        custom.release();
        lamina.free();
      },
    };
  }
  const bytes = new Uint8Array(await file.arrayBuffer());
  const lamina = Document.open(bytes, options);
  const opened = pdfiumOpen(p, bytes, password);
  return {
    pdfiumDoc: opened.doc,
    lamina,
    bytes,
    close: () => {
      p.FPDF_CloseDocument(opened.doc);
      p.pdfium.wasmExports.free(opened.ptr);
      lamina.free();
    },
  };
}

async function bench(id: number, req: BenchRequest): Promise<Record<string, unknown>> {
  const p = pdfium;
  if (p === undefined) throw new Error("boot first");
  const { password = "" } = req;
  const result: Record<string, unknown> = {
    file: req.file.name,
    size: req.file.size,
    strategy: req.strategy ?? "copy",
    memoryBefore: memory(),
  };

  // First page visible: the file in, both engines open, page 1 rendered and sent to the page.
  const t0 = performance.now();
  const opened = await openBoth(p, req);
  const pages = await opened.lamina.ensure(() => opened.lamina.pageCount);
  const first = await render(p, opened.pdfiumDoc, 0, 1);
  result.firstPageInWorkerMs = performance.now() - t0;
  self.postMessage({ id, first: true, bitmap: first.bitmap }, { transfer: [first.bitmap] });
  result.pages = pages;
  result.pdfiumPages = p.FPDF_GetPageCount(opened.pdfiumDoc);

  // Page 1 and a middle page at 1× and 2× DPR.
  const steps: Record<string, RenderTimes> = {};
  for (const [name, index] of [["first", 0], ["middle", Math.floor(pages / 2)]] as const) {
    for (const scale of [1, 2]) {
      const r = await render(p, opened.pdfiumDoc, index, scale);
      r.bitmap.close();
      steps[`${name}@${scale}x`] = r.times;
    }
  }
  result.steps = steps;

  // Per-page render times: every page, or up to 50 spread across the document.
  const sample = req.allPages ? pages : Math.min(pages, 50);
  const times: number[] = [];
  let peak = memory();
  for (let k = 0; k < sample; k++) {
    const index = req.allPages ? k : Math.floor((k * pages) / sample);
    const r = await render(p, opened.pdfiumDoc, index, 1);
    r.bitmap.close();
    times.push(r.times.renderMs + r.times.copyMs + r.times.bitmapMs);
    const now = memory();
    peak = Object.fromEntries(Object.entries(now).map(([k2, v]) => [k2, Math.max(v, peak[k2] ?? 0)]));
  }
  result.renderMs = stats(times);
  result.peakWasmMemory = peak;

  // Reload after an incremental save: lamina edits and saves, PDFium reopens the new bytes.
  if (req.reload && opened.bytes !== undefined) {
    const lamina = opened.lamina;
    const r0 = performance.now();
    lamina.transact((tx) => tx.setInfo({ title: "reloaded" }));
    const saved = lamina.save({ mode: "incremental" });
    const r1 = performance.now();
    p.FPDF_CloseDocument(opened.pdfiumDoc);
    const r2 = performance.now();
    const reopened = pdfiumOpen(p, saved, password);
    const r3 = performance.now();
    const again = await render(p, reopened.doc, 0, 1);
    again.bitmap.close();
    const r4 = performance.now();
    result.reload = {
      savedBytes: saved.length,
      appendedBytes: saved.length - opened.bytes.length,
      saveMs: r1 - r0,
      closeMs: r2 - r1,
      copyAndReopenMs: r3 - r2,
      rerenderMs: r4 - r3,
      totalMs: r4 - r0,
      memoryAfter: memory(),
    };
    p.FPDF_CloseDocument(reopened.doc);
    p.pdfium.wasmExports.free(reopened.ptr);
    lamina.free();
  } else {
    opened.close();
  }
  return result;
}

self.onmessage = async (event: MessageEvent) => {
  const { id, type, payload } = event.data as { id: number; type: string; payload?: BenchRequest };
  try {
    if (type === "boot") {
      self.postMessage({ id, ok: true, result: { ...(await boot()), memory: memory(), isolated: self.crossOriginIsolated } });
    } else if (type === "bench" && payload !== undefined) {
      self.postMessage({ id, ok: true, result: await bench(id, payload) });
    }
  } catch (e) {
    self.postMessage({ id, ok: false, error: e instanceof Error ? `${e.message}\n${e.stack}` : String(e) });
  }
};
