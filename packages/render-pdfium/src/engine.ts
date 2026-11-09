// The PDFium module as both files of this adapter use it.

import type { WrappedPdfiumModule } from "@embedpdf/pdfium";

/** The loaded PDFium module. */
export type Pdfium = WrappedPdfiumModule;

/** The module's memory: read it after each call that can grow it, never keep a view across one. */
export function heap(p: Pdfium): WebAssembly.Memory {
  return (p.pdfium.wasmExports as unknown as { memory: WebAssembly.Memory }).memory;
}

/** Runs `f` with `bytes` of scratch memory from the module, freed afterwards. */
export function withScratch<T>(p: Pdfium, bytes: number, f: (at: number) => T): T | undefined {
  const { malloc, free } = p.pdfium.wasmExports;
  const at = malloc(bytes);
  if (at === 0) return undefined;
  try {
    return f(at);
  } finally {
    free(at);
  }
}
