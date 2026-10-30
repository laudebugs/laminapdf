//! SPEC case 1: the incremental-update fixture and the overlay writers' goldens (B3b), byte for
//! byte. `hello.pdf` and `imaging-demo.pdf` themselves are checked by `lamina-build`'s
//! `goldens.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure; `expect` would be brittle as each test crate uses a different subset"
)]

mod common;

use common::fixture;
use common::samples;
use common::update;
use lamina_cos::{Dict, ObjRef, Stream};
use lamina_save::XrefEol;

#[test]
fn incremental_update_of_hello_reproduces_corpus_fixture_byte_for_byte() {
    let content = b"BT\n/F1 24 Tf\n72 720 Td\n(Hello, Update) Tj\nET".to_vec();
    let changes = [(ObjRef::new(5, 0), Stream::new(Dict::new(), content).into())];
    let updated = update(&fixture("golden/hello.pdf"), &changes, XrefEol::CrLf).unwrap();
    assert_eq!(
        String::from_utf8_lossy(&updated),
        String::from_utf8_lossy(&fixture("corpus/hello-incremental.pdf"))
    );
}

#[test]
fn incremental_set_info_is_byte_identical_to_golden_fixture() {
    let ours = samples::incremental_set_info().unwrap();
    assert_eq!(
        String::from_utf8_lossy(&ours),
        String::from_utf8_lossy(&fixture("golden/incremental-set-info.pdf"))
    );
}

#[test]
fn full_rewrite_hello_is_byte_identical_to_golden_fixture() {
    assert_eq!(
        samples::full_rewrite_hello().unwrap(),
        fixture("golden/full-rewrite-hello.pdf")
    );
}
