//! `Document::xfa` over `fixtures/forms/xfa-static.pdf` and `xfa-dynamic.pdf` (written by
//! `tools/fixtures/forms.py`) and small files for each packet layout: the packets, and the three
//! signals that make a form dynamic.
#![expect(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

use laminapdf_model::{Document, Limits, OpenOptions, XDP_PACKET, Xfa, XfaKind, XfaSignal};
use laminapdf_test_fixtures::fixture;

const TEMPLATE: &str = r#"<template xmlns="http://www.xfa.org/schema/xfa-template/3.3/"><subform name="f"><field name="a"/></subform></template>"#;
const EMPTY_TEMPLATE: &str =
    r#"<template xmlns="http://www.xfa.org/schema/xfa-template/3.3/"><subform name="f"/></template>"#;
const REQUIRED: &str =
    "<config><acrobat><acrobat7><dynamicRender>required</dynamicRender></acrobat7></acrobat></config>";
const FORBIDDEN: &str =
    "<config><acrobat><acrobat7><dynamicRender>forbidden</dynamicRender></acrobat7></acrobat></config>";

/// A file whose catalog adds `catalog`, whose form is `form` (its `/Fields` included) and whose
/// objects 10.. are the streams, as `(stream dictionary entries, data)`.
fn file(catalog: &str, form: &str, streams: &[(&str, &[u8])]) -> Vec<u8> {
    let mut objects: Vec<(u32, Vec<u8>)> = vec![
        (
            1,
            format!("<< /Type /Catalog /Pages 2 0 R /AcroForm 5 0 R {catalog} >>").into_bytes(),
        ),
        (2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec()),
        (
            3,
            b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Annots [6 0 R] >>".to_vec(),
        ),
        (5, form.as_bytes().to_vec()),
        (
            6,
            b"<< /FT /Tx /T (a) /Subtype /Widget /Rect [10 10 100 30] /P 3 0 R >>".to_vec(),
        ),
    ];
    for (i, (entries, data)) in streams.iter().enumerate() {
        let mut body = format!("<< {entries} /Length {} >>\nstream\n", data.len()).into_bytes();
        body.extend_from_slice(data);
        body.extend_from_slice(b"\nendstream");
        objects.push((10 + u32::try_from(i).unwrap(), body));
    }
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (num, body) in &objects {
        offsets.push((*num, out.len()));
        out.extend_from_slice(format!("{num} 0 obj\n").as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let size = offsets.last().unwrap().0 + 1;
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {size}\n0000000000 65535 f\r\n").as_bytes());
    for num in 1..size {
        match offsets.iter().find(|(n, _)| *n == num) {
            Some((_, at)) => out.extend_from_slice(format!("{at:010} 00000 n\r\n").as_bytes()),
            None => out.extend_from_slice(b"0000000000 65535 f\r\n"),
        }
    }
    out.extend_from_slice(format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    out
}

fn xfa_of(bytes: Vec<u8>) -> Option<Xfa> {
    Document::open(bytes, OpenOptions::default()).unwrap().xfa().unwrap()
}

fn open(name: &str) -> Option<Xfa> {
    xfa_of(fixture(&format!("forms/{name}")))
}

fn names(xfa: &Xfa) -> Vec<&str> {
    xfa.packets.iter().map(|p| p.name.as_str()).collect()
}

const WITH_FIELD: &str = "<< /Fields [6 0 R] /XFA [(template) 10 0 R (config) 11 0 R] >>";
const NO_FIELDS: &str = "<< /Fields [] /XFA [(template) 10 0 R (config) 11 0 R] >>";

#[test]
fn a_form_without_xfa_has_none() {
    assert_eq!(xfa_of(file("", "<< /Fields [6 0 R] >>", &[])), None);
    assert_eq!(open("fields.pdf"), None);
}

/// capability: E.17 core read
#[test]
fn the_static_fixture_is_static_with_its_packets_as_bytes() {
    let xfa = open("xfa-static.pdf").unwrap();
    assert_eq!(
        (xfa.kind, xfa.signals.clone(), names(&xfa)),
        (XfaKind::Static, vec![], vec!["template", "datasets"])
    );
    assert!(xfa.packet("template").unwrap().bytes.starts_with(b"<template xmlns="));
    assert!(xfa.packet("config").is_none());
}

#[test]
fn the_dynamic_fixture_is_dynamic_on_two_signals() {
    let xfa = open("xfa-dynamic.pdf").unwrap();
    assert_eq!(
        (xfa.kind, xfa.signals.clone(), names(&xfa)),
        (
            XfaKind::Dynamic,
            vec![XfaSignal::NeedsRendering, XfaSignal::DynamicRender],
            vec!["preamble", "config", "template", "datasets", "postamble"]
        )
    );
}

#[test]
fn needs_rendering_alone_makes_a_form_dynamic() {
    let xfa = xfa_of(file(
        "/NeedsRendering true",
        WITH_FIELD,
        &[("", TEMPLATE.as_bytes()), ("", FORBIDDEN.as_bytes())],
    ));
    let xfa = xfa.unwrap();
    assert_eq!(
        (xfa.kind, xfa.signals),
        (XfaKind::Dynamic, vec![XfaSignal::NeedsRendering])
    );
}

#[test]
fn needs_rendering_false_does_not() {
    let xfa = xfa_of(file(
        "/NeedsRendering false",
        WITH_FIELD,
        &[("", TEMPLATE.as_bytes()), ("", FORBIDDEN.as_bytes())],
    ));
    assert_eq!(xfa.unwrap().kind, XfaKind::Static);
}

#[test]
fn a_config_that_requires_dynamic_rendering_makes_a_form_dynamic() {
    let xfa = xfa_of(file(
        "",
        WITH_FIELD,
        &[("", TEMPLATE.as_bytes()), ("", REQUIRED.as_bytes())],
    ))
    .unwrap();
    assert_eq!(
        (xfa.kind, xfa.signals),
        (XfaKind::Dynamic, vec![XfaSignal::DynamicRender])
    );
}

#[test]
fn a_config_that_forbids_or_leaves_it_open_is_static() {
    for config in [
        FORBIDDEN,
        "<config/>",
        "<config><dynamicRender>1</dynamicRender></config>",
    ] {
        let xfa = xfa_of(file(
            "",
            WITH_FIELD,
            &[("", TEMPLATE.as_bytes()), ("", config.as_bytes())],
        ))
        .unwrap();
        assert_eq!(xfa.kind, XfaKind::Static, "{config}");
    }
}

#[test]
fn no_acroform_field_beside_a_template_with_fields_is_dynamic() {
    let xfa = xfa_of(file(
        "",
        NO_FIELDS,
        &[("", TEMPLATE.as_bytes()), ("", FORBIDDEN.as_bytes())],
    ))
    .unwrap();
    assert_eq!(
        (xfa.kind, xfa.signals),
        (XfaKind::Dynamic, vec![XfaSignal::NoAcroFormFields])
    );
}

#[test]
fn no_acroform_field_and_a_template_without_fields_is_static() {
    let xfa = xfa_of(file(
        "",
        NO_FIELDS,
        &[("", EMPTY_TEMPLATE.as_bytes()), ("", FORBIDDEN.as_bytes())],
    ))
    .unwrap();
    assert_eq!(xfa.kind, XfaKind::Static);
}

#[test]
fn one_stream_holds_the_whole_xdp_as_a_single_packet() {
    let xdp = |config: &str| format!(r#"<xdp:xdp xmlns:xdp="http://ns.adobe.com/xdp/">{config}{TEMPLATE}</xdp:xdp>"#);
    let form = "<< /Fields [6 0 R] /XFA 10 0 R >>";
    let dynamic = xfa_of(file("", form, &[("", xdp(REQUIRED).as_bytes())])).unwrap();
    let plain = xfa_of(file("", form, &[("", xdp(FORBIDDEN).as_bytes())])).unwrap();
    assert_eq!(
        (
            names(&dynamic),
            dynamic.kind,
            plain.kind,
            plain.packet(XDP_PACKET).is_some()
        ),
        (vec![XDP_PACKET], XfaKind::Dynamic, XfaKind::Static, true)
    );
}

#[test]
fn a_compressed_packet_is_decoded() {
    let packed = laminapdf_syntax::filters::flate_encode(TEMPLATE.as_bytes());
    let xfa = xfa_of(file(
        "",
        WITH_FIELD,
        &[("/Filter /FlateDecode", &packed), ("", FORBIDDEN.as_bytes())],
    ))
    .unwrap();
    assert_eq!(xfa.packet("template").unwrap().bytes, TEMPLATE.as_bytes());
}

#[test]
fn packets_that_aren_t_streams_or_are_unnamed_are_skipped() {
    let form = "<< /Fields [6 0 R] /XFA [(template) 10 0 R (broken) 99 0 R 7 11 0 R (config) (text) (datasets)] >>";
    let xfa = xfa_of(file("", form, &[("", TEMPLATE.as_bytes()), ("", b"x")])).unwrap();
    assert_eq!((xfa.kind, names(&xfa)), (XfaKind::Static, vec!["template"]));
}

#[test]
fn a_config_that_isn_t_well_formed_gives_no_signal_and_a_dtd_isn_t_read() {
    for config in [
        "<config><dynamicRender>required",
        "<!DOCTYPE c [<!ENTITY a \"required\">]><config><dynamicRender>&a;</dynamicRender></config>",
    ] {
        let xfa = xfa_of(file(
            "",
            WITH_FIELD,
            &[("", TEMPLATE.as_bytes()), ("", config.as_bytes())],
        ))
        .unwrap();
        assert_eq!(
            (xfa.kind, names(&xfa)),
            (XfaKind::Static, vec!["template", "config"]),
            "{config}"
        );
    }
}

#[test]
fn a_packet_over_the_decode_limit_is_not_read() {
    let packed = laminapdf_syntax::filters::flate_encode(&vec![b' '; 4096]);
    let bytes = file(
        "",
        WITH_FIELD,
        &[("/Filter /FlateDecode", &packed), ("", FORBIDDEN.as_bytes())],
    );
    let options = OpenOptions::default().with_limits(Limits::default().with_max_stream_decoded(1024));
    let xfa = Document::open(bytes, options).unwrap().xfa().unwrap().unwrap();
    assert_eq!(names(&xfa), vec!["config"]);
}
