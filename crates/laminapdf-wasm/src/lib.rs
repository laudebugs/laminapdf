//! WebAssembly bindings of the `laminapdf` façade (ADR 0002, ADR 0028), built into
//! `@laminapdf/wasm` by `cargo xtask wasm`.
//!
//! One crate, built three times ([ADR 0016] §4):
//!
//! * **base** (`lamina_pdf_base.wasm`, no features): [`WasmDocument`] (open, read, edit with ops,
//!   undo, save, save jobs, warnings), [`WasmPage`], and [`WasmBuilder`] /
//!   [`WasmPageBuilder`] / [`WasmImage`] for vector and image pages and the document's
//!   navigation and metadata.
//! * **text** (`lamina_pdf_text.wasm`, feature `text`): all of the base plus fonts (`WasmFont`),
//!   shaped text runs and text boxes, and text extraction (`WasmPage.text`).
//! * **extract** (`lamina_pdf_extract.wasm`, feature `extract`; B9c): all of the base plus text
//!   extraction (`WasmPage.text`), without fonts for building or shaping.
//!
//! The rules the bindings follow ([ADR 0016] §7): classes are the façade's types with a
//! `Wasm` prefix; methods are the façade's, camelCase (F12); bytes are `Uint8Array` in and
//! out (F7); options and results are plain JSON records with `.d.ts` types from tsify (F5,
//! F6: optional fields, unknown keys refused with `unknown-option`); every error is the
//! [ADR 0026] §1 object (thrown as a plain object; `@laminapdf/sdk` wraps it in `LaminaPdfError`);
//! the API is synchronous once the module is instantiated.
//!
//! This is an edge crate: `unsafe` exists only in wasm-bindgen's generated glue.
//!
//! [ADR 0016]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0016-package-design.md
//! [ADR 0026]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md

#[cfg(feature = "build")]
mod build_dto;
#[cfg(feature = "build")]
mod builder;
mod convert;
mod document;
mod dto;
mod error;
mod source;
#[cfg(feature = "extract")]
mod text;

#[cfg(feature = "text")]
pub use builder::WasmFont;
#[cfg(feature = "build")]
pub use builder::{WasmBuilder, WasmImage, WasmPageBuilder};
pub use document::{WasmDocument, WasmPage, WasmSaveJob, WasmWarningCursor};
pub use source::WasmSparseSource;

use wasm_bindgen::prelude::*;

/// The ADR 0026 §1 object every error and warning is, declared once for the `.d.ts`.
#[wasm_bindgen(typescript_custom_section)]
const DIAGNOSTIC_TS: &str = r#"
/** An error or warning (ADR 0026 §1). Errors are thrown as this object. */
export interface Diagnostic {
  code: string;
  kind: "input" | "io" | "unsupported" | "limit" | "security" | "usage" | "state" | "cancelled" | "internal";
  message: string;
  hint?: string;
  didYouMean?: string[];
  location?: { offset?: number; object?: number; streamOffset?: number; pageIndex?: number; id?: string };
  data?: Record<string, unknown>;
  docsUrl: string;
}
"#;

/// The crate version (`@laminapdf/wasm`'s).
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

/// Whether this module is the text module (fonts, `text`, `textBox`).
#[wasm_bindgen(js_name = hasText)]
pub fn has_text() -> bool {
    cfg!(feature = "text")
}
