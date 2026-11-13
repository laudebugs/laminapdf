//! Unit tests of the XMP ops: custom properties, the mirrored fields routed through the
//! metadata writer, the read-only identification and the replaced packet.

use laminapdf_model::xmp::{LangText, PacketState, RdfValue, XmpValue};

use super::super::tests::{doc, info_text, packet, xmp};
use super::*;
use crate::Op;

const ACME: &str = "http://example.com/acme/";
const DC: &str = "http://purl.org/dc/elements/1.1/";
const PDFA_2B: &str = "<pdfaid:part>2</pdfaid:part><pdfaid:conformance>B</pdfaid:conformance>";

fn set(namespace: &str, name: &str, value: XmpValue) -> SetXmpProperty {
    SetXmpProperty::new(namespace, "acme", name, value)
}

#[test]
fn a_custom_property_is_set_in_the_packet_alone() {
    let mut d = doc("", Some("<< /Title (T) >>"), Some(packet("").as_bytes()));
    set(ACME, "batch", XmpValue::Integer(7)).apply_to(&mut d).unwrap();
    assert_eq!(xmp(&d).property(ACME, "batch"), Some(&RdfValue::Text("7".into())));
    assert_eq!(info_text(&d, "Title").as_deref(), Some("T"));
}

#[test]
fn a_file_without_a_packet_gets_one_that_agrees_with_info() {
    let mut d = doc("", Some("<< /Title (Minutes) >>"), None);
    set(ACME, "batch", XmpValue::Text("A".into())).apply_to(&mut d).unwrap();
    let p = xmp(&d);
    assert_eq!(
        (p.title().as_deref(), p.property(ACME, "batch").is_some()),
        (Some("Minutes"), true)
    );
}

#[test]
fn a_mirrored_property_is_written_to_info_too() {
    let mut d = doc("", Some("<< /Title (Old) >>"), Some(packet("").as_bytes()));
    SetXmpProperty::new(
        DC,
        "dc",
        "title",
        XmpValue::LangAlt(vec![LangText::new("x-default", "New")]),
    )
    .apply_to(&mut d)
    .unwrap();
    assert_eq!(
        (info_text(&d, "Title").as_deref(), xmp(&d).title().as_deref()),
        (Some("New"), Some("New"))
    );
    RemoveXmpProperty::new(DC, "title").apply_to(&mut d).unwrap();
    assert_eq!((info_text(&d, "Title"), xmp(&d).title()), (None, None));
}

#[test]
fn a_mirrored_property_of_the_wrong_shape_is_refused() {
    let mut d = doc("", None, Some(packet("").as_bytes()));
    let wrong = [
        ("title", XmpValue::Integer(1)),
        ("creator", XmpValue::Text("Ada".into())),
        ("title", XmpValue::LangAlt(vec![LangText::new("en", "Hi")])),
    ];
    for (name, value) in wrong {
        let e = SetXmpProperty::new(DC, "dc", name, value).apply_to(&mut d).unwrap_err();
        assert_eq!(e, Error::InvalidXmpProperty { reason: "shape" }, "{name}");
    }
}

#[test]
fn the_identification_is_read_only() {
    let mut d = doc("", None, Some(packet(PDFA_2B).as_bytes()));
    for op in [
        Op::from(SetXmpProperty::new(PDFA_ID, "pdfaid", "part", XmpValue::Integer(3))),
        Op::from(RemoveXmpProperty::new(PDFUA_ID, "part")),
    ] {
        assert_eq!(op.apply_to(&mut d).unwrap_err().code(), "xmp-claim-read-only");
    }
}

#[test]
fn removing_a_custom_property_reports_it_was_there() {
    let body = r#"<acme:batch xmlns:acme="http://example.com/acme/">7</acme:batch>"#;
    let mut d = doc("", None, Some(packet(body).as_bytes()));
    let removed = RemoveXmpProperty::new(ACME, "batch").apply_to(&mut d).unwrap();
    assert!(removed.removed && xmp(&d).property(ACME, "batch").is_none());
    let again = RemoveXmpProperty::new(ACME, "batch").apply_to(&mut d).unwrap();
    assert!(!again.removed);
}

#[test]
fn removing_from_a_file_without_a_packet_adds_none() {
    let mut d = doc("", None, None);
    RemoveXmpProperty::new(ACME, "batch").apply_to(&mut d).unwrap();
    assert!(matches!(d.xmp().unwrap(), PacketState::Absent));
}

#[test]
fn a_malformed_packet_is_replaced_only_when_asked() {
    let mut d = doc("", None, Some(b"<x:xmpmeta"));
    let op = set(ACME, "batch", XmpValue::Integer(1));
    assert_eq!(op.apply_to(&mut d).unwrap_err(), Error::MalformedXmp);
    op.replacing_malformed().apply_to(&mut d).unwrap();
    assert!(xmp(&d).property(ACME, "batch").is_some());
}

#[test]
fn a_custom_property_in_a_pdfa_file_is_declared() {
    let mut d = doc("", None, Some(packet(PDFA_2B).as_bytes()));
    let effect = set(ACME, "batch", XmpValue::Integer(7)).apply_to(&mut d).unwrap();
    assert_eq!(effect.declared.len(), 1);
    assert_eq!(effect.declared.first().map(|p| p.value_type.as_str()), Some("Integer"));
}

#[test]
fn a_bad_name_is_refused_with_its_reason() {
    let mut d = doc("", None, Some(packet("").as_bytes()));
    let e = set(ACME, "not a name", XmpValue::Integer(1))
        .apply_to(&mut d)
        .unwrap_err();
    assert_eq!(e, Error::InvalidXmpProperty { reason: "name" });
    let e = set(ACME, "tags", XmpValue::Bag(vec![])).apply_to(&mut d).unwrap_err();
    assert_eq!(e, Error::InvalidXmpProperty { reason: "value" });
}

#[test]
fn a_replaced_packet_brings_info_into_step() {
    let old = packet(r#"<dc:title><rdf:Alt><rdf:li xml:lang="x-default">Old</rdf:li></rdf:Alt></dc:title>"#);
    let mut d = doc("", Some("<< /Title (Old) /Producer (P) >>"), Some(old.as_bytes()));
    let new = packet(
        r#"<dc:title><rdf:Alt><rdf:li xml:lang="x-default">New</rdf:li></rdf:Alt></dc:title><acme:x xmlns:acme="http://example.com/acme/">1</acme:x>"#,
    );
    let effect = ReplaceXmpPacket::new(new.clone()).apply_to(&mut d).unwrap();
    assert_eq!(xmp(&d).original(), new.as_bytes());
    assert_eq!(info_text(&d, "Title").as_deref(), Some("New"));
    assert_eq!(info_text(&d, "Producer"), None, "the new packet has no producer");
    assert!(effect.removed, "the old packet is still in the earlier revision");
}

#[test]
fn a_replaced_packet_that_does_not_parse_is_refused() {
    let mut d = doc("", None, Some(packet("").as_bytes()));
    let e = ReplaceXmpPacket::new("<x:xmpmeta><rdf:RDF/></x:xmpmeta>")
        .apply_to(&mut d)
        .unwrap_err();
    assert_eq!(e.code(), "invalid-xmp-packet");
}

#[test]
fn a_replaced_packet_keeps_the_identification() {
    let mut d = doc("", None, Some(packet(PDFA_2B).as_bytes()));
    let changed = packet("<pdfaid:part>3</pdfaid:part><pdfaid:conformance>B</pdfaid:conformance>");
    let e = ReplaceXmpPacket::new(changed).apply_to(&mut d).unwrap_err();
    assert_eq!(e, Error::XmpClaimReadOnly);
    let added = packet("<pdfuaid:part>1</pdfuaid:part>");
    let mut plain = doc("", None, Some(packet("").as_bytes()));
    assert_eq!(
        ReplaceXmpPacket::new(added).apply_to(&mut plain).unwrap_err(),
        Error::XmpClaimReadOnly
    );
}

#[test]
fn a_replaced_pdfa_packet_gets_its_custom_properties_declared() {
    let mut d = doc("", None, Some(packet(PDFA_2B).as_bytes()));
    let new = packet(&format!(
        "{PDFA_2B}<acme:batch xmlns:acme=\"http://example.com/acme/\">7</acme:batch>"
    ));
    let effect = ReplaceXmpPacket::new(new).apply_to(&mut d).unwrap();
    assert_eq!(
        effect.declared.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
        ["batch"]
    );
    let written = String::from_utf8(xmp(&d).original().to_vec()).unwrap();
    assert!(written.contains("pdfaExtension:schemas"));
}

#[test]
fn replacing_with_the_same_packet_changes_nothing() {
    let same = packet("");
    let mut d = doc("", None, Some(same.as_bytes()));
    let effect = ReplaceXmpPacket::new(same).apply_to(&mut d).unwrap();
    assert_eq!(effect, Effect::default());
    assert!(!d.cos().is_dirty());
}

#[test]
fn every_xmp_op_is_a_metadata_change_saved_incrementally() {
    let ops = [
        Op::from(set(ACME, "a", XmpValue::Boolean(true))),
        Op::from(RemoveXmpProperty::new(ACME, "a")),
        Op::from(ReplaceXmpPacket::new(packet(""))),
    ];
    for op in ops {
        assert_eq!(
            (op.class(), op.requires_rewrite()),
            (crate::ModificationClass::Metadata, false)
        );
    }
}

#[test]
fn a_malformed_pdfa_packet_can_be_repaired_keeping_its_claim() {
    let broken = format!("{}<unclosed>", packet(PDFA_2B));
    let mut d = doc("", None, Some(broken.as_bytes()));
    assert!(d.xmp().unwrap().is_malformed());
    ReplaceXmpPacket::new(packet(PDFA_2B)).apply_to(&mut d).unwrap();
    assert_eq!(xmp(&d).pdfa_part(), Some(2));
}

#[test]
fn a_malformed_packet_can_neither_gain_nor_lose_a_claim() {
    let mut plain = doc("", None, Some(b"<x:xmpmeta"));
    let claimed = ReplaceXmpPacket::new(packet(PDFA_2B)).apply_to(&mut plain);
    assert_eq!(claimed.unwrap_err(), Error::XmpClaimReadOnly);
    let broken = format!("{}<unclosed>", packet(PDFA_2B));
    let mut pdfa = doc("", None, Some(broken.as_bytes()));
    assert_eq!(
        ReplaceXmpPacket::new(packet("")).apply_to(&mut pdfa).unwrap_err(),
        Error::XmpClaimReadOnly
    );
}

#[test]
fn the_extension_schemas_own_namespaces_are_refused() {
    let mut d = doc("", None, Some(packet("").as_bytes()));
    for ns in [
        "http://www.aiim.org/pdfa/ns/type#",
        "http://www.aiim.org/pdfa/ns/field#",
    ] {
        let e = SetXmpProperty::new(ns, "t", "type", XmpValue::Text("x".into()))
            .apply_to(&mut d)
            .unwrap_err();
        assert_eq!(e, Error::InvalidXmpProperty { reason: "namespace" }, "{ns}");
    }
}

#[test]
fn removing_a_property_the_packet_lacks_writes_nothing() {
    let mut d = doc("", None, Some(packet("").as_bytes()));
    assert_eq!(
        RemoveXmpProperty::new(ACME, "batch").apply_to(&mut d).unwrap(),
        Effect::default()
    );
    assert!(!d.cos().is_dirty());
}

#[test]
fn setting_the_value_a_property_holds_writes_nothing() {
    let body = r#"<acme:batch xmlns:acme="http://example.com/acme/">7</acme:batch>"#;
    let mut d = doc("", None, Some(packet(body).as_bytes()));
    let effect = set(ACME, "batch", XmpValue::Integer(7)).apply_to(&mut d).unwrap();
    assert_eq!((effect, d.cos().is_dirty()), (Effect::default(), false));
}

#[test]
fn an_empty_list_for_a_mirrored_property_is_refused() {
    let mut d = doc("", None, Some(packet("").as_bytes()));
    let e = SetXmpProperty::new(DC, "dc", "creator", XmpValue::Seq(vec![]))
        .apply_to(&mut d)
        .unwrap_err();
    assert_eq!(e, Error::InvalidXmpProperty { reason: "value" });
}

#[test]
fn a_malformed_packets_repair_cannot_change_its_level() {
    let broken = format!("{}<unclosed>", packet(PDFA_2B));
    let mut d = doc("", None, Some(broken.as_bytes()));
    let other_level = packet("<pdfaid:part>1</pdfaid:part><pdfaid:conformance>A</pdfaid:conformance>");
    assert_eq!(
        ReplaceXmpPacket::new(other_level).apply_to(&mut d).unwrap_err(),
        Error::XmpClaimReadOnly
    );
    let attributes = format!(
        "{}<unclosed>",
        packet("").replace(
            r#"rdf:about="""#,
            r#"rdf:about="" pdfaid:part="2" pdfaid:conformance="B""#
        )
    );
    let mut d = doc("", None, Some(attributes.as_bytes()));
    ReplaceXmpPacket::new(packet(PDFA_2B)).apply_to(&mut d).unwrap();
}
