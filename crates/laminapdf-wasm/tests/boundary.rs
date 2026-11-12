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
use laminapdf_wasm::{WasmBuilder, WasmDocument};
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
fn the_set_metadata_sample_applies_and_reads_back_from_metadata() {
    let mut doc = hello();
    doc.apply(&js(&format!(
        "[{}]",
        include_str!("../../../fixtures/ops/v1/set-metadata.json")
    )))
    .unwrap();
    let meta = doc.metadata().unwrap();
    assert_eq!(
        (
            string(&meta, "title").as_deref(),
            get(&meta, "authors").dyn_into::<js_sys::Array>().unwrap().length(),
            string(&meta, "packet").as_deref(),
        ),
        (Some("Quarterly figures"), 2, Some("parsed"))
    );
}

#[wasm_bindgen_test]
fn a_plan_crosses_as_a_record_and_a_stamped_save_says_so() {
    let mut doc = hello();
    doc.apply(&js(&format!(
        "[{}]",
        include_str!("../../../fixtures/ops/v1/remove-metadata.json")
    )))
    .unwrap();
    doc.apply(&js(
        r#"[{ "op": "setMetadata", "args": { "patch": { "title": "T" } } }]"#,
    ))
    .unwrap();
    let options =
        js(r#"{ "metadataDate": "2026-11-10T09:30:00Z", "instanceId": "abababab-abab-abab-abab-abababababab" }"#);
    let plan = doc.plan(&options).unwrap();
    assert_eq!(
        (
            string(&plan, "constraint").as_deref(),
            string(&plan, "metadata").as_deref()
        ),
        (Some("free"), Some("stamp"))
    );
    let report = get(&doc.save(&options).unwrap(), "report");
    assert_eq!(get(&report, "metadataUpdated").as_bool(), Some(true));
}

#[wasm_bindgen_test]
fn a_bad_metadata_date_is_an_invalid_argument() {
    let doc = hello();
    let err = doc.plan(&js(r#"{ "metadataDate": "yesterday" }"#)).unwrap_err();
    assert_eq!(string(&err, "code").as_deref(), Some("invalid-argument"));
}

#[wasm_bindgen_test]
fn would_violate_answers_with_no_violation_on_an_unsigned_file() {
    let doc = hello();
    let op = js(r#"{ "op": "setMetadata", "args": { "patch": { "title": "T" } } }"#);
    let violations = doc.would_violate(&op).unwrap();
    assert_eq!(violations.dyn_into::<js_sys::Array>().unwrap().length(), 0);
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
            Some("https://docs.laminapdf.com/reference/codes/not-pdf/")
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
fn a_page_and_a_save_job_outliving_their_document_are_disposed() {
    let doc = hello();
    let page = doc.page(0).unwrap();
    let mut job = doc.save_job(&JsValue::UNDEFINED).unwrap();
    let id = page.id();
    drop(doc);
    let code = |e: JsValue| string(&e, "code");
    assert_eq!(
        (
            code(page.index().unwrap_err()),
            code(page.info().unwrap_err()),
            code(job.step(1).unwrap_err())
        ),
        (
            Some("disposed".to_owned()),
            Some("disposed".to_owned()),
            Some("disposed".to_owned())
        )
    );
    // The ID is the page's own: it needs no document.
    assert_eq!(page.id(), id);
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

/// `WasmDocument.outline` and `namedDestination` (the extract and text modules): nested items
/// with page targets, views tagged by `kind`, and `undefined` for an unknown name.
#[cfg(feature = "extract")]
#[wasm_bindgen_test]
fn the_outline_and_named_destinations_cross_as_records() {
    use js_sys::Array;
    const NAV: &[u8] = include_bytes!("../../../fixtures/golden/links-and-outlines.pdf");
    let doc = WasmDocument::open_bytes(NAV.to_vec(), &JsValue::UNDEFINED).unwrap();
    let outline: Array = doc.outline().unwrap().dyn_into().unwrap();
    let chapters = outline.get(2);
    assert_eq!(string(&chapters, "title").as_deref(), Some("Chapters"));
    let children: Array = get(&chapters, "children").dyn_into().unwrap();
    assert_eq!(get(&get(&children.get(0), "target"), "page").as_f64(), Some(2.0));
    let contents = doc.named_destination("contents").unwrap();
    assert_eq!(string(&get(&contents, "view"), "kind").as_deref(), Some("fit"));
    assert!(doc.named_destination("nowhere").unwrap().is_undefined());
}

/// `WasmDocument.properties` (the extract and text modules): Info entries, a nested `contains`,
/// encryption with its permissions, and fonts as records.
#[cfg(feature = "extract")]
#[wasm_bindgen_test]
fn properties_cross_as_one_record() {
    use js_sys::Array;
    const OUTLINE: &[u8] = include_bytes!("../../../fixtures/corpus/outline.pdf");
    const ENC: &[u8] = include_bytes!("../../../fixtures/corpus/enc.pdf");
    let doc = WasmDocument::open_bytes(OUTLINE.to_vec(), &JsValue::UNDEFINED).unwrap();
    let p = doc.properties().unwrap();
    assert_eq!(string(&p, "title").as_deref(), Some("Outline fixture"));
    assert_eq!(string(&p, "created").as_deref(), Some("2026-10-28T12:00:00Z"));
    assert_eq!(get(&p, "pageCount").as_f64(), Some(3.0));
    assert_eq!(get(&get(&p, "contains"), "forms").as_bool(), Some(false));
    let fonts: Array = get(&p, "fonts").dyn_into().unwrap();
    assert_eq!(string(&fonts.get(0), "name").as_deref(), Some("Helvetica"));
    // A missing value is left out, never `null`: the `.d.ts` types it as optional.
    assert!(Reflect::has(&p, &JsValue::from_str("encryption")).is_ok_and(|has| !has));
    let enc = WasmDocument::open_bytes(ENC.to_vec(), &JsValue::UNDEFINED).unwrap();
    let encryption = get(&enc.properties().unwrap(), "encryption");
    assert_eq!(get(&encryption, "revision").as_f64(), Some(6.0));
    assert!(get(&get(&encryption, "permissions"), "print").as_bool().is_some());
}

/// `WasmDocument.pageLinks` (the extract and text modules): a rectangle in page space, the kind
/// at the top level beside its fields, and `page-index-out-of-range` past the end.
#[cfg(feature = "extract")]
#[wasm_bindgen_test]
fn page_links_cross_as_records_tagged_by_kind() {
    use js_sys::Array;
    const LINKS: &[u8] = include_bytes!("../../../fixtures/links/links.pdf");
    let doc = WasmDocument::open_bytes(LINKS.to_vec(), &JsValue::UNDEFINED).unwrap();
    let links: Array = doc.page_links(0).unwrap().dyn_into().unwrap();
    let first = links.get(0);
    assert_eq!(string(&first, "kind").as_deref(), Some("dest"));
    assert_eq!(get(&get(&first, "target"), "page").as_f64(), Some(2.0));
    let rect: Array = get(&first, "rect").dyn_into().unwrap();
    assert_eq!(rect.get(1).as_f64(), Some(38.0));
    assert_eq!(string(&links.get(1), "name").as_deref(), Some("chapter"));
    assert_eq!(
        string(&links.get(3), "uri").as_deref(),
        Some("https://example.com/a?b=c#d")
    );
    assert_eq!(string(&links.get(3), "contents").as_deref(), Some("The project site"));
    assert_eq!(string(&links.get(10), "kind").as_deref(), Some("named-action"));
    assert_eq!(string(&links.get(12), "action").as_deref(), Some("Launch"));
    let err = doc.page_links(3).unwrap_err();
    assert_eq!(string(&err, "code").as_deref(), Some("page-index-out-of-range"));
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

/// `WasmDocument.reflow` (the extract module): reader mode's JSON in one string.
#[cfg(feature = "reflow")]
#[wasm_bindgen_test]
fn reflow_crosses_as_json_and_reads_an_untagged_file_approximately() {
    const MIXED: &[u8] = include_bytes!("../../../fixtures/reflow/mixed.pdf");
    let doc = WasmDocument::open_bytes(MIXED.to_vec(), &JsValue::UNDEFINED).unwrap();
    let json = doc.reflow().unwrap();
    assert!(json.starts_with(r#"{"available":true,"source":"tags","#), "{json}");
    assert!(json.contains(r#""kind":"figure""#) && json.contains(r#""alt":"Four coloured squares in a grid""#));
    let untagged = hello().reflow().unwrap();
    assert!(
        untagged.starts_with(r#"{"available":true,"source":"approximate","reason":"untagged""#),
        "{untagged}"
    );
}

#[wasm_bindgen_test]
fn an_unknown_tag_scope_is_an_invalid_argument() {
    let builder = WasmBuilder::new(&JsValue::UNDEFINED).unwrap();
    let mut page = builder.page(100.0, 100.0);
    let error = page
        .begin_tag("TH", None, None, None, None, None, Some("rows".to_owned()))
        .err()
        .unwrap();
    assert_eq!(string(&error, "code").as_deref(), Some("invalid-argument"));
}

#[wasm_bindgen_test]
fn an_unknown_tag_kind_is_an_invalid_argument() {
    let builder = WasmBuilder::new(&JsValue::UNDEFINED).unwrap();
    let mut page = builder.page(100.0, 100.0);
    let error = page
        .begin_tag("Heading", None, None, None, None, None, None)
        .err()
        .unwrap();
    assert_eq!(string(&error, "code").as_deref(), Some("invalid-argument"));
}

#[wasm_bindgen_test]
fn an_unknown_artifact_kind_is_an_invalid_argument() {
    let builder = WasmBuilder::new(&JsValue::UNDEFINED).unwrap();
    let mut page = builder.page(100.0, 100.0);
    let error = page.begin_artifact("sidebar").err().unwrap();
    assert_eq!(string(&error, "code").as_deref(), Some("invalid-argument"));
}

#[wasm_bindgen_test]
fn header_footer_and_watermark_are_artifact_kinds() {
    let builder = WasmBuilder::new(&JsValue::UNDEFINED).unwrap();
    let mut page = builder.page(100.0, 100.0);
    for kind in ["header", "footer", "watermark"] {
        assert!(page.begin_artifact(kind).is_ok(), "{kind}");
        page.end_tag().unwrap();
    }
}

/// `WasmBuilder.markdown` (the text module): its options record refuses an unknown key, the
/// template's too, and a Latin document builds in the standard 14 fonts.
#[cfg(feature = "text")]
#[wasm_bindgen_test]
fn markdown_options_refuse_unknown_keys_and_a_latin_document_builds() {
    use laminapdf_wasm::WasmMarkdownOptions;
    let builder = WasmBuilder::new(&JsValue::UNDEFINED).unwrap();
    let resources = WasmMarkdownOptions::new();
    let err = builder
        .markdown("# Hi\n", &resources, &js(r#"{"contentz":2}"#))
        .unwrap_err();
    assert_eq!(string(&err, "code").as_deref(), Some("unknown-option"));
    let err = builder
        .markdown("# Hi\n", &resources, &js(r#"{"page":{"size":[420,595],"footr":{}}}"#))
        .unwrap_err();
    assert_eq!(string(&err, "code").as_deref(), Some("unknown-option"));
    let report = builder
        .markdown("# Notes\n\nPlain *Latin* text.\n", &resources, &js(r#"{"contents":1}"#))
        .unwrap();
    let pages: js_sys::Array = get(&report, "pages").dyn_into().unwrap();
    assert!(pages.length() >= 1);
    assert_eq!(builder.finish().unwrap().page_count().unwrap(), pages.length());
}

/// `WasmBuilder.flow` (the text module): an unknown option is `unknown-option`, a misspelt key in
/// the document is `invalid-document` with the closest key, a template's missing value names its
/// pointer, and a plain document builds in the standard 14 fonts.
#[cfg(feature = "text")]
#[wasm_bindgen_test]
fn flow_refuses_unknown_keys_and_missing_data_and_a_plain_document_builds() {
    use laminapdf_wasm::WasmFlowResources;
    let builder = WasmBuilder::new(&JsValue::UNDEFINED).unwrap();
    let resources = WasmFlowResources::new();
    let plain = js(
        r#"{"schema":"laminapdf.document/1","styles":{"body":{"font":"Helvetica","size":10}},"blocks":[{"type":"paragraph","text":"Hi"}]}"#,
    );
    let err = builder.flow(&plain, &resources, &js(r#"{"dat":{}}"#)).unwrap_err();
    assert_eq!(string(&err, "code").as_deref(), Some("unknown-option"));
    let misspelt =
        js(r#"{"schema":"laminapdf.document/1","styles":{"body":{"font":"Helvetica","size":10}},"blokcs":[]}"#);
    let err = builder.flow(&misspelt, &resources, &JsValue::UNDEFINED).unwrap_err();
    assert_eq!(string(&err, "code").as_deref(), Some("invalid-document"));
    let suggestions: js_sys::Array = get(&err, "didYouMean").dyn_into().unwrap();
    assert!(suggestions.includes(&JsValue::from_str("blocks"), 0), "{suggestions:?}");
    let template = js(
        r#"{"schema":"laminapdf.document/1","styles":{"body":{"font":"Helvetica","size":10}},"blocks":[{"type":"paragraph","text":{"$data":"/name"}}]}"#,
    );
    let err = builder.flow(&template, &resources, &js(r#"{"data":{}}"#)).unwrap_err();
    assert_eq!(string(&err, "code").as_deref(), Some("template-data-missing"));
    let report = builder.flow(&plain, &resources, &JsValue::UNDEFINED).unwrap();
    let pages: js_sys::Array = get(&report, "pages").dyn_into().unwrap();
    assert_eq!(pages.length(), 1);
    assert_eq!(builder.finish().unwrap().page_count().unwrap(), 1);
}
