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

/// A field whose name has more parts than the writer can nest and the reader read back is left
/// out, however the file nested it; one at the limit is kept and reads back.
#[test]
fn field_names_deeper_than_the_writer_can_nest_are_left_out() {
    use laminapdf_interchange::{read, write_fdf, write_xfdf};
    use laminapdf_syntax::Limits;
    let parts = |n: usize| vec!["a"; n].join(".");
    let xfdf = |name: &str| {
        format!(
            "<xfdf xmlns=\"http://ns.adobe.com/xfdf/\"><fields><field name=\"{name}\"><value>1</value></field></fields></xfdf>"
        )
    };
    let fdf = |name: &str| {
        format!(
            "%FDF-1.2\n1 0 obj\n<< /FDF << /Fields [<< /T ({name}) /V (1) >>] >> >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n"
        )
    };
    let limits = Limits::new();
    let fields_read = |file: &str, write: fn(&laminapdf_interchange::Exchange) -> Vec<u8>| {
        let once = read(file.as_bytes(), &limits).unwrap();
        let again = read(&write(&once), &limits).unwrap();
        (once.fields.len(), again.fields.len())
    };
    let max = limits.max_tree_depth();
    assert_eq!(
        [
            fields_read(&xfdf(&parts(61)), write_xfdf),
            fields_read(&xfdf(&parts(100)), write_xfdf),
            fields_read(&fdf(&parts(max)), write_fdf),
            fields_read(&fdf(&parts(100)), write_fdf),
        ],
        [(1, 1), (0, 0), (1, 1), (0, 0)]
    );
}
