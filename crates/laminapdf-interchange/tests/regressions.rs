//! Replays every saved crash input of the `xfdf`, `fdf` and `form_json` fuzz targets
//! (`fixtures/regressions/<target>/`) through the target's own entry point, and the field names
//! a review found that wrote back otherwise.

#![allow(clippy::panic, clippy::unwrap_used, reason = "test code: a panic is a test failure")]

mod common;

use common::fuzz_entry::TARGETS;

#[test]
fn every_saved_crash_input_runs_clean() {
    for (name, entry) in TARGETS {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../fixtures/regressions/{name}"));
        let Ok(inputs) = std::fs::read_dir(&dir) else {
            continue;
        };
        for input in inputs {
            entry(&std::fs::read(input.unwrap().path()).unwrap());
        }
    }
}

/// Field names that once read back as other fields: a name starting with an empty part, two
/// entries for one name, and an empty parent beside its child.
#[test]
fn awkward_field_names_round_trip_in_xfdf_fdf_and_json() {
    use common::fuzz_entry::{fdf, form_json, xfdf};
    let field = |name: &str, value: &str| format!("<field name=\"{name}\">{value}</field>");
    for fields in [
        field(".a", "<value>1</value>"),
        field(".a", "<value>1</value>") + &field("a", "<value>2</value>"),
        field("a", "") + &field("a.b", "<value>x</value>"),
        field("a", "<value>1</value>") + &field("a", "<value>2</value>"),
    ] {
        xfdf(format!("<xfdf xmlns=\"http://ns.adobe.com/xfdf/\"><fields>{fields}</fields></xfdf>").as_bytes());
    }
    for fields in [
        "<< /T () /Kids [<< /T (a) /V (1) >>] >> << /T (a) /V (2) >>",
        "<< /T (a) >> << /T (a) /Kids [<< /T (b) /V (x) >>] >>",
    ] {
        fdf(
            format!("%FDF-1.2\n1 0 obj\n<< /FDF << /Fields [{fields}] >> >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n")
                .as_bytes(),
        );
    }
    form_json(br#"{ "fields": { "a": null, "a.b": "x", ".a": "1", "a": "2" } }"#);
}
