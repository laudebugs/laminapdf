// Declarations for the two runtime-specific imports of src/loader/, so the build needs neither
// @types/node nor a workers types package (`types: []` keeps Node's globals out of src/).

// In workerd (loader/workerd.ts), a `.wasm` import is a compiled `WebAssembly.Module`.
declare module "@laminapdf/wasm/lamina_pdf_base.wasm" {
  const module: WebAssembly.Module;
  export default module;
}
declare module "@laminapdf/wasm/lamina_pdf_text.wasm" {
  const module: WebAssembly.Module;
  export default module;
}

// The one Node API, in loader/node.ts.
declare module "node:fs/promises" {
  export function readFile(path: URL): Promise<Uint8Array>;
}
