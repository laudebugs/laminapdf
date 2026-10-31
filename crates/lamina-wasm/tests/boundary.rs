//! The boundary types in a wasm runtime (Node, through `wasm-bindgen-test-runner`; `cargo xtask
//! wasm` runs this for the base, text and extract modules): the op samples of `fixtures/ops/v1` cross
//! as JS values exactly as their JSON wire format says, records refuse unknown keys with
//! `unknown-option`, and every error is the ADR 0026 §1 object.
#![cfg(target_arch = "wasm32")]
#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test code: a panic is a test failure"
)]

use js_sys::{JSON, Reflect};
use lamina_wasm::{WasmBuilder, WasmDocument};
use wasm_bindgen::{JsCast as _, JsValue};
use wasm_bindgen_test::wasm_bindgen_test;

const HELLO: &[u8] = include_bytes!("../../../fixtures/golden/examples/hello.pdf");

fn js(json: &str) -> JsValue {
    JSON::parse(json).unwrap()
}

fn get(value: &JsValue, key: &str) -> JsValue {
    Reflect::get(value, &key.into()).unwrap()
}

fn string(value: &JsValue, key: &str) -> Option<String> {
    get(value, key).as_string()
}

fn hello() -> WasmDocument {
    WasmDocument::open_bytes(HELLO.to_vec(), &JsValue::UNDEFINED).unwrap()
}

#[wasm_bindgen_test]
fn the_set_info_sample_applies() {
    let mut doc = hello();
    doc.apply(&js(&format!(
        "[{}]",
        include_str!("../../../fixtures/ops/v1/set-info.json")
    )))
    .unwrap();
    assert_eq!(
        string(&doc.info().unwrap(), "title").as_deref(),
        Some("Quarterly report")
    );
}

#[wasm_bindgen_test]
fn the_rotate_page_sample_applies_to_this_documents_page() {
    let mut doc = hello();
    let id = doc.page(0).unwrap().id();
    let sample = include_str!("../../../fixtures/ops/v1/rotate-page.json").replace("p3.0", &id);
    doc.apply(&js(&format!("[{sample}]"))).unwrap();
    assert_eq!(get(&doc.page_info(0).unwrap(), "rotation").as_f64(), Some(90.0));
}

#[wasm_bindgen_test]
fn the_envelope_sample_applies_as_a_log() {
    let mut doc = hello();
    let id = doc.page(0).unwrap().id();
    let log = include_str!("../../../fixtures/ops/v1/envelope.json").replace("p4.0", &id);
    doc.apply_log(&log).unwrap();
    assert_eq!(get(&doc.page_info(0).unwrap(), "rotation").as_f64(), Some(270.0));
}

#[wasm_bindgen_test]
fn an_unknown_op_is_an_invalid_argument() {
    let mut doc = hello();
    let e = doc.apply(&js(r#"[{ "op": "rotatePag", "args": {} }]"#)).unwrap_err();
    assert_eq!(string(&e, "code").as_deref(), Some("invalid-argument"));
}

#[wasm_bindgen_test]
fn a_key_with_backquotes_never_reaches_did_you_mean() {
    // serde's message quotes the key in backquotes: the suggestions must not come from it.
    let options = js(r#"{ "strict`, `IgnorePrevious": true }"#);
    let e = WasmDocument::open_bytes(HELLO.to_vec(), &options)
        .err()
        .expect("an error");
    assert_eq!(string(&e, "code").as_deref(), Some("unknown-option"));
    let suggestions = JSON::stringify(&get(&e, "didYouMean")).unwrap().as_string();
    assert!(!suggestions.unwrap_or_default().contains("Ignore"));
}

#[wasm_bindgen_test]
fn an_unknown_option_key_is_unknown_option() {
    let e = WasmDocument::open_bytes(HELLO.to_vec(), &js(r#"{ "strickt": true }"#))
        .err()
        .expect("an error");
    assert_eq!(
        (string(&e, "code").as_deref(), string(&e, "kind").as_deref()),
        (Some("unknown-option"), Some("usage"))
    );
    assert_eq!(string(&get(&e, "data"), "option").as_deref(), Some("strickt"));
}

#[wasm_bindgen_test]
fn a_core_error_is_the_adr_0026_object() {
    let e = WasmDocument::open_bytes(b"not a pdf".to_vec(), &JsValue::UNDEFINED)
        .err()
        .expect("an error");
    assert_eq!(
        (
            string(&e, "code").as_deref(),
            string(&e, "kind").as_deref(),
            string(&e, "docsUrl").as_deref()
        ),
        (
            Some("not-pdf"),
            Some("input"),
            Some("https://github.com/laudebugs/lamina/blob/main/docs/errors/not-pdf.md")
        )
    );
}

#[wasm_bindgen_test]
fn save_options_cross_as_a_record() {
    let mut doc = hello();
    let saved = doc
        .save(&js(r#"{ "mode": "fullRewrite", "xref": "classic" }"#))
        .unwrap();
    let report = get(&saved, "report");
    assert_eq!(string(&report, "mode").as_deref(), Some("fullRewrite"));
    assert!(get(&saved, "bytes").is_instance_of::<js_sys::Uint8Array>());
}

#[wasm_bindgen_test]
fn warnings_are_diagnostics() {
    // A header-less copy of hello.pdf opens with the `missing-header` warning.
    let bytes = HELLO.get(8..).unwrap().to_vec();
    let doc = WasmDocument::open_bytes(bytes, &JsValue::UNDEFINED).unwrap();
    let batch: js_sys::Array = doc.warnings().next_batch(10).into();
    let codes: Vec<String> = batch.iter().filter_map(|w| string(&w, "code")).collect();
    assert!(codes.iter().any(|c| c == "missing-header"), "{codes:?}");
}

#[wasm_bindgen_test]
fn a_finished_page_refuses_more_drawing_and_a_second_finish() {
    let b = WasmBuilder::new(&JsValue::UNDEFINED).unwrap();
    let mut page = b.page(100.0, 100.0);
    page.finish().unwrap();
    let code = |e: JsValue| string(&e, "code");
    assert_eq!(
        (code(page.save().unwrap_err()), code(page.finish().unwrap_err())),
        (Some("builder-finished".to_owned()), Some("builder-finished".to_owned()))
    );
}

#[wasm_bindgen_test]
fn a_finished_builder_refuses_more_pages() {
    let b = WasmBuilder::new(&JsValue::UNDEFINED).unwrap();
    let mut page = b.page(100.0, 100.0);
    b.finish().unwrap();
    let e = page.finish().unwrap_err();
    assert_eq!(string(&e, "code").as_deref(), Some("builder-finished"));
}

#[wasm_bindgen_test]
fn a_path_cut_short_is_an_invalid_argument() {
    let b = WasmBuilder::new(&JsValue::UNDEFINED).unwrap();
    let mut page = b.page(100.0, 100.0);
    let e = page.fill_path(&[1.0, 2.0], "nonZero").unwrap_err();
    assert_eq!(string(&e, "code").as_deref(), Some("invalid-argument"));
}

#[cfg(feature = "text")]
#[wasm_bindgen_test]
fn a_misspelt_standard_font_suggests_the_name() {
    let b = WasmBuilder::new(&JsValue::UNDEFINED).unwrap();
    let e = b.standard_font("Helvetika").err().expect("an error");
    assert_eq!(
        JSON::stringify(&get(&e, "didYouMean")).unwrap().as_string().as_deref(),
        Some(r#"["Helvetica"]"#)
    );
}

#[cfg(feature = "text")]
#[wasm_bindgen_test]
fn a_text_call_that_fails_draws_nothing() {
    let b = WasmBuilder::new(&js(r#"{ "compress": false }"#)).unwrap();
    let font = b.standard_font("Helvetica").unwrap();
    let mut page = b.page(200.0, 200.0);
    let e = page
        .text("Hello", &font, 12.0, 10.0, 20.0, &js(r#"{ "features": ["liga=x"] }"#))
        .unwrap_err();
    assert_eq!(string(&e, "code").as_deref(), Some("invalid-feature"));
    page.finish().unwrap();
    let saved = b.finish().unwrap().save(&JsValue::UNDEFINED).unwrap();
    let bytes = js_sys::Uint8Array::new(&get(&saved, "bytes")).to_vec();
    assert!(!bytes.windows(5).any(|w| w == b"Hello"));
}

/// `WasmPage.text` (the extract and text modules): lines, runs as records, one quad per glyph
/// in a `Float32Array`, glyph text with UTF-16 offsets.
#[cfg(feature = "extract")]
#[wasm_bindgen_test]
fn page_text_crosses_as_a_struct_of_arrays() {
    use js_sys::{Array, Float32Array, Uint32Array};
    let doc = hello();
    let text = doc.page(0).unwrap().text(&JsValue::UNDEFINED).unwrap();
    let lines: Array = get(&text, "lines").dyn_into().unwrap();
    assert_eq!(lines.get(0).as_string().as_deref(), Some("Hello, world!"));
    let runs: Array = get(&text, "runs").dyn_into().unwrap();
    assert_eq!(string(&runs.get(0), "text").as_deref(), Some("Hello, world!"));
    let offsets: Uint32Array = get(&text, "glyphOffsets").dyn_into().unwrap();
    let glyphs = offsets.length() - 1;
    assert_eq!(glyphs, 13);
    let quads: Float32Array = get(&text, "quads").dyn_into().unwrap();
    assert_eq!(quads.length(), glyphs * 8);
    assert_eq!(string(&text, "glyphText").as_deref(), Some("Hello, world!"));
    let err = doc
        .page(0)
        .unwrap()
        .text(&js(r#"{"skipInvisibles":true}"#))
        .unwrap_err();
    assert_eq!(string(&err, "code").as_deref(), Some("unknown-option"));
}

#[wasm_bindgen_test]
fn a_tag_scope_out_of_range_is_an_invalid_argument() {
    let builder = WasmBuilder::new(&JsValue::UNDEFINED).unwrap();
    let mut page = builder.page(100.0, 100.0);
    // 16 is `TH`; scopes are 0 (none) to 3 (both).
    let error = page.begin_tag(16, None, None, None, None, None, 4).err().unwrap();
    assert_eq!(string(&error, "code").as_deref(), Some("invalid-argument"));
}

#[wasm_bindgen_test]
fn a_tag_kind_out_of_range_is_an_invalid_argument() {
    let builder = WasmBuilder::new(&JsValue::UNDEFINED).unwrap();
    let mut page = builder.page(100.0, 100.0);
    let error = page.begin_tag(25, None, None, None, None, None, 0).err().unwrap();
    assert_eq!(string(&error, "code").as_deref(), Some("invalid-argument"));
}
