//! The `xfdf`, `fdf` and `form_json` fuzz targets' entry points (ADR 0018 tier 9). The targets in
//! `fuzz/fuzz_targets/` call these through `fuzz/src/lib.rs`, which includes this file, and
//! `tests/regressions.rs` replays every saved crash input (`fixtures/regressions/<target>/`)
//! through the same function, so a regression test runs exactly what the fuzzer ran.

use laminapdf_interchange::{FormDataFormat, read, read_form_data, write_fdf, write_form_data, write_xfdf};
use laminapdf_syntax::Limits;

/// A fuzz target's entry point.
pub type Entry = fn(&[u8]);

/// The targets, by name.
pub const TARGETS: [(&str, Entry); 3] = [("xfdf", xfdf), ("fdf", fdf), ("form_json", form_json)];

/// Small limits, so a fuzz input can't take long: what a hostile file meets in a browser tab,
/// scaled down.
fn limits() -> Limits {
    Limits::new()
        .with_max_stream_decoded(1 << 20)
        .with_max_decoded_total(4 << 20)
        .with_max_object_number(10_000)
}

/// Reads `data` as XFDF; a file that reads writes XFDF that reads back to the same comments and
/// fields, and writing that again gives the same bytes.
pub fn xfdf(data: &[u8]) {
    round_trip(data, write_xfdf);
}

/// Reads `data` as FDF, with the same checks in FDF.
pub fn fdf(data: &[u8]) {
    round_trip(data, write_fdf);
}

fn round_trip(data: &[u8], write: fn(&laminapdf_interchange::Exchange) -> Vec<u8>) {
    let Ok(read_once) = read(data, &limits()) else {
        return;
    };
    let written = write(&read_once);
    let read_again = match read(&written, &limits()) {
        Ok(e) => e,
        Err(e) => panic!("what was written doesn't read back: {e}"),
    };
    assert_eq!(
        read_again.comments.len(),
        read_once.comments.len(),
        "the comments written didn't all read back"
    );
    assert_eq!(
        read_again.fields.len(),
        read_once.fields.len(),
        "the fields written didn't all read back"
    );
    assert_eq!(
        write(&read_again),
        written,
        "writing what was read back gave other bytes"
    );
}

/// Reads `data` as form data JSON; a file that reads writes JSON that reads back to the same
/// fields, and writing that again gives the same bytes.
pub fn form_json(data: &[u8]) {
    if !data.trim_ascii_start().starts_with(b"{") {
        return;
    }
    let Ok(read_once) = read_form_data(data, &limits()) else {
        return;
    };
    let write =
        |fields: Vec<laminapdf_interchange::FieldData>| write_form_data(FormDataFormat::Json, fields, None, None);
    let written = write(read_once.clone());
    let read_again = match read_form_data(&written, &limits()) {
        Ok(f) => f,
        Err(e) => panic!("what was written doesn't read back: {e}"),
    };
    assert_eq!(read_again, read_once, "the fields written didn't read back the same");
    assert_eq!(
        write(read_again),
        written,
        "writing what was read back gave other bytes"
    );
}
