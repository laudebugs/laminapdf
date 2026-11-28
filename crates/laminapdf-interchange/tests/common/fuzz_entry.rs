//! The `xfdf`, `fdf` and `form_json` fuzz targets' entry points (ADR 0018 tier 9). The targets in
//! `fuzz/fuzz_targets/` call these through `fuzz/src/lib.rs`, which includes this file, and
//! `tests/regressions.rs` replays every saved crash input (`fixtures/regressions/<target>/`)
//! through the same function, so a regression test runs exactly what the fuzzer ran.

use laminapdf_interchange::{FormDataFormat, read, read_form_data, write_fdf, write_form_data, write_xfdf};
use laminapdf_syntax::Limits;

/// A fuzz target's entry point.
pub type Entry = fn(&[u8]);

/// The targets, by name.
pub const TARGETS: [(&str, Entry); 4] = [
    ("xfdf", xfdf),
    ("fdf", fdf),
    ("form_json", form_json),
    ("submit_request", submit_request),
];

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

/// Reads `data` as a `SubmitForm` action's flags (its first two bytes) and address (the rest) on a
/// small form: a request that is built is to an `https:` host or a `mailto:` address, carries no
/// document, and builds the same way twice; an address that isn't either is refused.
pub fn submit_request(data: &[u8]) {
    use laminapdf_interchange::{SubmitTarget, submit_request};
    use laminapdf_model::{FieldSelection, Form, SubmitFlags, SubmitForm};
    let (flags, url) = match data {
        [a, b, rest @ ..] => (
            u32::from(*a) | (u32::from(*b) << 8),
            String::from_utf8_lossy(rest).into_owned(),
        ),
        _ => return,
    };
    let action = SubmitForm::new(url, SubmitFlags(flags), FieldSelection::new(Vec::new(), false));
    let form = Form::default();
    let first = submit_request(&form, &action);
    assert_eq!(
        first,
        submit_request(&form, &action),
        "a request builds the same way twice"
    );
    if let Ok(request) = first {
        match request.target {
            SubmitTarget::Web => assert!(request.url.to_ascii_lowercase().starts_with("https://")),
            _ => assert!(request.body.is_empty() && request.url.to_ascii_lowercase().starts_with("mailto:")),
        }
    }
}
