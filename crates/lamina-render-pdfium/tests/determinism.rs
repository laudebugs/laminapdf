//! Rendering the same page twice gives the same bytes, for each backend (ADR 0004 §1: goldens
//! depend on it). Ignored: needs the PDFium library (`cargo xtask render-golden`).
#![allow(clippy::unwrap_used, clippy::panic, reason = "tests: a panic is a test failure")]

mod common;

use common::{fixture, open, render};
use lamina_render::{RenderOpenOptions, RenderRequest};

#[test]
#[ignore = "needs the PDFium library: cargo xtask render-golden"]
fn the_same_request_renders_the_same_bytes() {
    for backend in common::backends() {
        for file in ["imaging-demo.pdf", "text-example.pdf"] {
            let bytes = fixture(&format!("corpus/{file}"));
            let req = RenderRequest::page(0, 1.0);
            let first = render(
                open(backend.as_ref(), bytes.clone(), &RenderOpenOptions::new())
                    .unwrap()
                    .as_ref(),
                &req,
            )
            .unwrap();
            // A second document, and the same document twice.
            let doc = open(backend.as_ref(), bytes, &RenderOpenOptions::new()).unwrap();
            let second = render(doc.as_ref(), &req).unwrap();
            let third = render(doc.as_ref(), &req).unwrap();
            assert!(
                first.rgba == second.rgba && second.rgba == third.rgba,
                "{} {file}",
                backend.name()
            );
        }
    }
}
