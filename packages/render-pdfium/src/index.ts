// ADR 0004's default web renderer (todo C2c): the pinned PDFium-WASM build behind
// @laminapdf/render's RenderBackend. It mirrors the native adapter (laminapdf-render-pdfium):
// - input through PDFium's custom file access over a RenderSource (no whole copy); a read the
//   source can't answer yet makes the call throw `data-pending` with the ranges;
// - a form-fill environment per document, so widgets are drawn by FPDF_FFLDraw and can be left
//   out; with widgets the page is laid out at its whole pixel size and shifted so only the tile
//   lands in a tile-sized bitmap, without them FPDF_RenderPageBitmapWithMatrix gets
//   pageToTile's matrix;
// - straight RGBA through FPDF_REVERSE_BYTE_ORDER, copied out once into the result.
// PDFium is synchronous: the AbortSignal is checked before and after the engine call.
import { init, type WrappedPdfiumModule } from "@embedpdf/pdfium";
import {
  checkOpen,
  checkPageIndex,
  flagsOf,
  limitsOf,
  MAX_WASM_INPUT,
  pageToTile,
  pendingError,
  planTile,
  type RenderBackend,
  type RenderDocument,
  RenderError,
  type RenderedTile,
  type RenderLimits,
  type RenderOpenOptions,
  type RenderPage,
  type RenderRequest,
  type RenderSource,
  readFailure,
  readRange,
  type TilePlan,
  throwIfAborted,
  type WasmSource,
  WHITE,
} from "@laminapdf/render";

/** The pinned package (ADR 0004 §3); its pdfium.wasm's SHA-256 is checked at build time. */
export const PDFIUM_PACKAGE = "@embedpdf/pdfium@2.15.1";

type Pdfium = WrappedPdfiumModule;

// fpdfview.h
const FPDF_ANNOT = 0x01;
const FPDF_GRAYSCALE = 0x08;
const FPDF_REVERSE_BYTE_ORDER = 0x10;
const FPDF_PRINTING = 0x800;
const FPDFBITMAP_BGRA = 4;
const FPDF_ERR_PASSWORD = 4;

/** A compiled module, or a URL or `Response` to fetch: the forms the glue can't take as `wasmBinary`. */
async function instantiate(
  wasm: Exclude<WasmSource, ArrayBuffer | ArrayBufferView<ArrayBuffer>>,
  imports: WebAssembly.Imports,
): Promise<WebAssembly.Instance> {
  if (wasm instanceof WebAssembly.Module) return WebAssembly.instantiate(wasm, imports);
  const awaited = await wasm;
  const response = awaited instanceof Response ? awaited : await fetch(awaited);
  if (!response.ok) throw new RenderError("render-backend-unavailable", `pdfium.wasm: HTTP ${response.status}`);
  // Streaming compile needs `application/wasm`; fall back to the bytes otherwise.
  if (response.headers.get("content-type")?.startsWith("application/wasm")) {
    return (await WebAssembly.instantiateStreaming(response, imports)).instance;
  }
  return (await WebAssembly.instantiate(await response.arrayBuffer(), imports)).instance;
}

/**
 * Loads PDFium. Call it inside the document worker (ADR 0009 §2), once per worker.
 *
 * @throws RenderError `render-backend-unavailable` when the module can't be fetched or compiled.
 */
export async function initPdfium(wasm: WasmSource): Promise<PdfiumBackend> {
  let fail: (e: unknown) => void = () => {};
  const failed = new Promise<never>((_, reject) => {
    fail = reject;
  });
  // Bytes go to the glue's own `wasmBinary`; the other forms through its instantiate hook.
  const overrides =
    wasm instanceof ArrayBuffer || ArrayBuffer.isView(wasm)
      ? { wasmBinary: wasm }
      : {
          instantiateWasm: (imports: WebAssembly.Imports, done: (instance: WebAssembly.Instance) => void) => {
            instantiate(wasm, imports).then(done, fail);
            return {};
          },
        };
  try {
    const pdfium = await Promise.race([failed, init(overrides as never)]);
    pdfium.PDFiumExt_Init();
    return new PdfiumBackend(pdfium);
  } catch (e) {
    if (e instanceof RenderError) throw e;
    throw new RenderError("render-backend-unavailable", "PDFium could not be loaded");
  }
}

/** PDFium-WASM as a RenderBackend. */
export class PdfiumBackend implements RenderBackend {
  readonly name = "pdfium";
  readonly version = PDFIUM_PACKAGE;
  readonly #pdfium: Pdfium;

  constructor(pdfium: Pdfium) {
    this.#pdfium = pdfium;
  }

  /** The size of PDFium's WebAssembly memory, in bytes. */
  memory(): number {
    return heap(this.#pdfium).buffer.byteLength;
  }

  /** Opens `source` (see {@link RenderBackend.open}). */
  open(source: RenderSource, options: RenderOpenOptions = {}): RenderDocument {
    return PdfiumDocument.open(this.#pdfium, source, options);
  }
}

function heap(p: Pdfium): WebAssembly.Memory {
  return (p.pdfium.wasmExports as unknown as { memory: WebAssembly.Memory }).memory;
}

/** What the source refused during one engine call: missing ranges, or a failed read. */
interface Reads {
  readonly pending: (readonly [number, number])[];
  failure: RenderError | undefined;
}

/** The error the refused reads make, if any: a failed read first, else `data-pending`. */
function refusal(reads: Reads): RenderError | undefined {
  if (reads.failure !== undefined) return reads.failure;
  return reads.pending.length > 0 ? pendingError(reads.pending) : undefined;
}

class PdfiumDocument implements RenderDocument {
  readonly pageCount: number;
  readonly #p: Pdfium;
  readonly #limits: RenderLimits;
  readonly #pages = new Map<number, RenderPage>();
  readonly #reads: Reads;
  #doc: number;
  #form: number;
  #formInfo: number;
  #access: number;
  #getBlock: number;

  private constructor(
    p: Pdfium,
    handles: { doc: number; form: number; formInfo: number; access: number; getBlock: number; count: number },
    reads: Reads,
    limits: RenderLimits,
  ) {
    this.#p = p;
    this.#doc = handles.doc;
    this.#form = handles.form;
    this.#formInfo = handles.formInfo;
    this.#access = handles.access;
    this.#getBlock = handles.getBlock;
    this.pageCount = handles.count;
    this.#reads = reads;
    this.#limits = limits;
  }

  static open(p: Pdfium, source: RenderSource, options: RenderOpenOptions): PdfiumDocument {
    if (source.size > MAX_WASM_INPUT) {
      throw new RenderError("render-too-large", "custom file access on wasm32 is limited to 4 GiB of input");
    }
    const reads: Reads = { pending: [], failure: undefined };
    const { malloc, free } = p.pdfium.wasmExports;
    // FPDF_FILEACCESS.m_GetBlock(param, position, pBuf, size): 1 on success, 0 on failure. It
    // never throws: an exception would unwind through PDFium's C++ frames. A throwing or short
    // read is recorded and reported after the engine call returns.
    const getBlock = p.pdfium.addFunction((_param: number, position: number, buf: number, size: number) => {
      const at = position >>> 0;
      const length = size >>> 0;
      try {
        const bytes = readRange(source, at, length);
        if (bytes === undefined) {
          reads.pending.push([at, length]);
          return 0;
        }
        new Uint8Array(heap(p).buffer, buf, length).set(bytes);
        return 1;
      } catch (e) {
        reads.failure ??= readFailure(at, length, e);
        return 0;
      }
    }, "iiiii");
    // FPDF_FILEACCESS on wasm32: m_FileLen (unsigned long), m_GetBlock, m_Param: 12 bytes.
    const access = malloc(12);
    if (access === 0) {
      p.pdfium.removeFunction(getBlock);
      throw new RenderError("render-failed", "the pdfium backend could not allocate memory");
    }
    p.pdfium.setValue(access, source.size >>> 0, "i32");
    p.pdfium.setValue(access + 4, getBlock, "i32");
    p.pdfium.setValue(access + 8, 0, "i32");
    const release = () => {
      free(access);
      p.pdfium.removeFunction(getBlock);
    };
    const doc = p.FPDF_LoadCustomDocument(access, options.password ?? "");
    if (doc === 0) {
      const error = p.FPDF_GetLastError();
      release();
      const refused = refusal(reads);
      if (refused !== undefined) throw refused;
      if (error === FPDF_ERR_PASSWORD) {
        throw options.password === undefined
          ? new RenderError("password-required", "the document is encrypted and needs a password")
          : new RenderError("wrong-password", "the password does not open the document");
      }
      throw new RenderError("render-failed", "the pdfium backend could not open the document");
    }
    const formInfo = p.PDFiumExt_OpenFormFillInfo();
    const form = p.PDFiumExt_InitFormFillEnvironment(doc, formInfo);
    const count = p.FPDF_GetPageCount(doc);
    // PDFium may open after refused reads by rebuilding the structure, and the form environment
    // and the page count read more: any refusal up to here makes the open fail, so a document
    // never keeps a page count read from a partial file.
    const refused = refusal(reads);
    if (refused !== undefined) {
      if (form !== 0) p.PDFiumExt_ExitFormFillEnvironment(form);
      if (formInfo !== 0) p.PDFiumExt_CloseFormFillInfo(formInfo);
      p.FPDF_CloseDocument(doc);
      release();
      throw refused;
    }
    return new PdfiumDocument(p, { doc, form, formInfo, access, getBlock, count }, reads, limitsOf(options.limits));
  }

  #live(): number {
    checkOpen(this.#doc !== 0);
    return this.#doc;
  }

  /**
   * Runs `f` with the refusals cleared; any read the source refused makes the outcome that
   * refusal (`source` or `data-pending`), success or not.
   */
  #reading<T>(f: () => T): T {
    this.#reads.pending.length = 0;
    this.#reads.failure = undefined;
    let outcome: T;
    try {
      outcome = f();
    } catch (e) {
      throw refusal(this.#reads) ?? e;
    }
    const refused = refusal(this.#reads);
    if (refused !== undefined) throw refused;
    return outcome;
  }

  #withPage<T>(index: number, f: (page: number) => T): T {
    const doc = this.#live();
    checkPageIndex(index, this.pageCount);
    const p = this.#p;
    const page = p.FPDF_LoadPage(doc, index);
    if (page === 0) throw new RenderError("render-failed", "the pdfium backend could not load the page");
    if (this.#form !== 0) p.FORM_OnAfterLoadPage(page, this.#form);
    try {
      return f(page);
    } finally {
      if (this.#form !== 0) p.FORM_OnBeforeClosePage(page, this.#form);
      p.FPDF_ClosePage(page);
    }
  }

  #geometry(page: number): RenderPage {
    const p = this.#p;
    return {
      width: p.FPDF_GetPageWidthF(page),
      height: p.FPDF_GetPageHeightF(page),
      rotate: (((p.FPDFPage_GetRotation(page) % 4) + 4) % 4) * 90,
    };
  }

  page(index: number): RenderPage {
    this.#live();
    const cached = this.#pages.get(index);
    if (cached !== undefined) return cached;
    const page = this.#reading(() => this.#withPage(index, (handle) => this.#geometry(handle)));
    this.#pages.set(index, page);
    return page;
  }

  plan(request: RenderRequest): TilePlan {
    return planTile(this.page(request.page), request, this.#limits);
  }

  render(request: RenderRequest, options: { readonly signal?: AbortSignal } = {}): RenderedTile {
    // As in every adapter: the abort first, then whether the document is open.
    throwIfAborted(options.signal);
    this.#live();
    const p = this.#p;
    const { malloc, free } = p.pdfium.wasmExports;
    const result = this.#reading(() =>
      this.#withPage(request.page, (page) => {
        const info = this.#geometry(page);
        const plan = planTile(info, request, this.#limits);
        const { width, height } = plan.tile;
        const flags = flagsOf(request.flags);
        const background = request.background === undefined ? WHITE : request.background;
        const renderFlags =
          (flags.annotations ? FPDF_ANNOT : 0) |
          (flags.print ? FPDF_PRINTING : 0) |
          (flags.grayscale ? FPDF_GRAYSCALE : 0) |
          FPDF_REVERSE_BYTE_ORDER;
        const buffer = malloc(plan.byteLength);
        if (buffer === 0) throw new RenderError("render-failed", "the pdfium backend could not allocate the tile");
        const bitmap = p.FPDFBitmap_CreateEx(width, height, FPDFBITMAP_BGRA, buffer, width * 4);
        let scratch = 0;
        try {
          if (bitmap === 0) throw new RenderError("render-failed", "the pdfium backend could not allocate the tile");
          // FillRect takes 0xAARRGGBB into BGRA memory; with R and B swapped it lands as RGBA, the
          // order the reversed-byte-order render writes.
          const c = background ?? { r: 0, g: 0, b: 0, a: 0 };
          p.FPDFBitmap_FillRect(bitmap, 0, 0, width, height, ((c.a << 24) | (c.b << 16) | (c.g << 8) | c.r) >>> 0);
          const rotate = (request.rotation ?? 0) / 90;
          if (flags.annotations && flags.formWidgets) {
            const [x, y] = [-plan.tile.x, -plan.tile.y];
            p.FPDF_RenderPageBitmap(bitmap, page, x, y, plan.pageWidth, plan.pageHeight, rotate, renderFlags);
            if (this.#form !== 0) {
              p.FPDF_FFLDraw(this.#form, bitmap, page, x, y, plan.pageWidth, plan.pageHeight, rotate, renderFlags);
            }
          } else {
            // FS_MATRIX (6 floats) then FS_RECTF (left, top, right, bottom).
            scratch = malloc(40);
            if (scratch === 0) throw new RenderError("render-failed", "the pdfium backend could not allocate memory");
            const floats = new Float32Array(heap(p).buffer, scratch, 10);
            floats.set([...pageToTile(plan, info, request.rotation ?? 0), 0, 0, width, height]);
            p.FPDF_RenderPageBitmapWithMatrix(bitmap, page, scratch, scratch + 24, renderFlags);
          }
          const pixels = new Uint8ClampedArray(plan.byteLength);
          pixels.set(new Uint8Array(heap(p).buffer, buffer, plan.byteLength));
          return { width, height, pixels };
        } finally {
          if (scratch !== 0) free(scratch);
          if (bitmap !== 0) p.FPDFBitmap_Destroy(bitmap);
          free(buffer);
        }
      }),
    );
    throwIfAborted(options.signal);
    return result;
  }

  close(): void {
    if (this.#doc === 0) return;
    const p = this.#p;
    if (this.#form !== 0) p.PDFiumExt_ExitFormFillEnvironment(this.#form);
    if (this.#formInfo !== 0) p.PDFiumExt_CloseFormFillInfo(this.#formInfo);
    p.FPDF_CloseDocument(this.#doc);
    p.pdfium.wasmExports.free(this.#access);
    p.pdfium.removeFunction(this.#getBlock);
    this.#doc = 0;
    this.#form = 0;
    this.#formInfo = 0;
  }

  [Symbol.dispose](): void {
    this.close();
  }
}
