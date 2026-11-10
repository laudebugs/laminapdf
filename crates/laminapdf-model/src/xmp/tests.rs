use super::*;

const RDF_OPEN: &str = r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">"#;

/// A packet around `descriptions`, with the wrapper and some padding.
fn wrapped(descriptions: &str) -> String {
    format!(
        "<?xpacket begin=\"\u{FEFF}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n {RDF_OPEN}\n{descriptions}\n </rdf:RDF>\n</x:xmpmeta>\n{}<?xpacket end=\"w\"?>",
        " ".repeat(200)
    )
}

fn packet(descriptions: &str) -> Packet {
    Packet::parse(wrapped(descriptions).as_bytes()).unwrap()
}

fn text(written: &Written) -> String {
    String::from_utf8(written.bytes.clone()).unwrap()
}

/// The packet's properties other than `except`, for comparing two packets property by property.
fn others(p: &Packet, except: (&str, &str)) -> Vec<RdfProperty> {
    p.properties()
        .filter(|q| (q.namespace.as_str(), q.name.as_str()) != except)
        .cloned()
        .collect()
}

const TITLE: (&str, &str) = (DC, "title");

#[test]
fn attribute_form_properties_are_read() {
    let p = packet(
        r#"  <rdf:Description rdf:about="" xmlns:pdf="http://ns.adobe.com/pdf/1.3/" xmlns:xmp="http://ns.adobe.com/xap/1.0/" pdf:Producer="P &amp; Q" xmp:CreateDate="2026-10-28T14:30:00+01:00"/>"#,
    );
    assert_eq!(
        (p.producer(), p.create_date()),
        (
            Some("P & Q".to_owned()),
            Some("D:20261028143000+01'00".parse().unwrap())
        )
    );
}

#[test]
fn element_form_properties_are_read() {
    let p = packet(
        r#"  <rdf:Description rdf:about="" xmlns:pdf="http://ns.adobe.com/pdf/1.3/">
   <pdf:Keywords>a, b</pdf:Keywords>
  </rdf:Description>"#,
    );
    assert_eq!(p.keywords().as_deref(), Some("a, b"));
}

#[test]
fn language_alternatives_prefer_x_default_then_the_first() {
    let p = packet(
        r#"  <rdf:Description rdf:about="" xmlns:dc="http://purl.org/dc/elements/1.1/">
   <dc:title><rdf:Alt><rdf:li xml:lang="fr">Titre</rdf:li><rdf:li xml:lang="x-default">Title</rdf:li></rdf:Alt></dc:title>
   <dc:description><rdf:Alt><rdf:li xml:lang="de">Thema</rdf:li><rdf:li xml:lang="en">Subject</rdf:li></rdf:Alt></dc:description>
  </rdf:Description>"#,
    );
    assert_eq!(
        (p.title(), p.subject()),
        (Some("Title".to_owned()), Some("Thema".to_owned()))
    );
}

#[test]
fn a_seq_reads_every_item_in_order() {
    let p = packet(
        r#"  <rdf:Description rdf:about="" xmlns:dc="http://purl.org/dc/elements/1.1/">
   <dc:creator><rdf:Seq><rdf:li>Ada</rdf:li><rdf:li>Grace</rdf:li></rdf:Seq></dc:creator>
  </rdf:Description>"#,
    );
    assert_eq!(p.authors(), ["Ada", "Grace"]);
}

#[test]
fn a_bag_reads_as_its_items() {
    let p = packet(
        r#"  <rdf:Description rdf:about="" xmlns:dc="http://purl.org/dc/elements/1.1/">
   <dc:subject><rdf:Bag><rdf:li>pdf</rdf:li><rdf:li>xmp</rdf:li></rdf:Bag></dc:subject>
  </rdf:Description>"#,
    );
    let items = match p.property(DC, "subject") {
        Some(RdfValue::Array {
            kind: ArrayKind::Bag,
            items,
        }) => items.iter().map(|i| i.value.clone()).collect::<Vec<_>>(),
        other => panic!("{other:?}"),
    };
    assert_eq!(items, [RdfValue::Text("pdf".into()), RdfValue::Text("xmp".into())]);
}

#[test]
fn a_parse_type_resource_structure_is_kept_whole() {
    let structure = r#"<xmpMM:DerivedFrom rdf:parseType="Resource"><stRef:documentID>uuid:1</stRef:documentID></xmpMM:DerivedFrom>"#;
    let p = packet(&format!(
        r#"  <rdf:Description rdf:about="" xmlns:xmpMM="http://ns.adobe.com/xap/1.0/mm/" xmlns:stRef="http://ns.adobe.com/xap/1.0/sType/ResourceRef#">
   {structure}
  </rdf:Description>"#
    ));
    assert_eq!(
        p.property(XMP_MM, "DerivedFrom"),
        Some(&RdfValue::Other(structure.to_owned()))
    );
}

#[test]
fn several_descriptions_are_read_together() {
    let p = packet(
        r#"  <rdf:Description rdf:about="" xmlns:pdf="http://ns.adobe.com/pdf/1.3/"><pdf:Producer>P</pdf:Producer></rdf:Description>
  <rdf:Description rdf:about="" xmlns:xmp="http://ns.adobe.com/xap/1.0/"><xmp:CreatorTool>C</xmp:CreatorTool></rdf:Description>"#,
    );
    assert_eq!(
        (p.producer(), p.creator_tool()),
        (Some("P".to_owned()), Some("C".to_owned()))
    );
}

#[test]
fn namespaces_are_matched_by_uri_whatever_the_prefix() {
    let p = packet(
        r#"  <r:Description xmlns:r="http://www.w3.org/1999/02/22-rdf-syntax-ns#" r:about="" xmlns:d="http://purl.org/dc/elements/1.1/">
   <d:title><r:Alt><r:li xml:lang="x-default">T</r:li></r:Alt></d:title>
  </r:Description>"#,
    );
    assert_eq!(p.title().as_deref(), Some("T"));
}

#[test]
fn a_packet_without_wrapper_or_xmpmeta_is_read() {
    let bare = format!(
        r#"{RDF_OPEN}<rdf:Description rdf:about="" xmlns:pdf="http://ns.adobe.com/pdf/1.3/" pdf:Producer="P"/></rdf:RDF>"#
    );
    assert_eq!(Packet::parse(bare.as_bytes()).unwrap().producer().as_deref(), Some("P"));
}

#[test]
fn identification_schemas_are_read() {
    let p = packet(
        r#"  <rdf:Description rdf:about="" xmlns:pdfaid="http://www.aiim.org/pdfa/ns/id/" xmlns:pdfuaid="http://www.aiim.org/pdfua/ns/id/" pdfaid:part="2" pdfaid:conformance="B">
   <pdfuaid:part>1</pdfuaid:part>
  </rdf:Description>"#,
    );
    assert_eq!(
        (p.pdfa_part(), p.pdfa_conformance(), p.pdfua_part()),
        (Some(2), Some("B".to_owned()), Some(1))
    );
}

#[test]
fn ids_and_dates_without_seconds_are_read() {
    let p = packet(
        r#"  <rdf:Description rdf:about="" xmlns:xmpMM="http://ns.adobe.com/xap/1.0/mm/" xmlns:xmp="http://ns.adobe.com/xap/1.0/">
   <xmpMM:DocumentID>uuid:d</xmpMM:DocumentID><xmpMM:InstanceID>uuid:i</xmpMM:InstanceID>
   <xmp:ModifyDate>2026-10-28T14:30Z</xmp:ModifyDate><xmp:MetadataDate>not a date</xmp:MetadataDate>
  </rdf:Description>"#,
    );
    assert_eq!(
        (p.document_id(), p.instance_id(), p.modify_date(), p.metadata_date()),
        (
            Some("uuid:d".to_owned()),
            Some("uuid:i".to_owned()),
            Some("D:20261028143000Z".parse().unwrap()),
            None
        )
    );
}

#[test]
fn character_references_and_cdata_are_decoded() {
    let p = packet(
        r#"  <rdf:Description rdf:about="" xmlns:pdf="http://ns.adobe.com/pdf/1.3/">
   <pdf:Keywords>&#x41;&#66;&lt;<![CDATA[&c]]></pdf:Keywords>
  </rdf:Description>"#,
    );
    assert_eq!(p.keywords().as_deref(), Some("AB<&c"));
}

#[test]
fn a_dtd_is_refused() {
    let evil = r#"<?xml version="1.0"?><!DOCTYPE lolz [<!ENTITY lol "lol"><!ENTITY lol2 "&lol;&lol;&lol;">]><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"/>"#;
    assert!(matches!(
        Packet::parse(evil.as_bytes()),
        Err(ParseError::Dtd { offset: 21 })
    ));
}

#[test]
fn an_undefined_entity_is_not_expanded() {
    let p = wrapped(
        r#"  <rdf:Description rdf:about="" xmlns:pdf="http://ns.adobe.com/pdf/1.3/"><pdf:Producer>&xxe;</pdf:Producer></rdf:Description>"#,
    );
    assert!(matches!(
        Packet::parse(p.as_bytes()),
        Err(ParseError::UndefinedEntity { .. })
    ));
}

#[test]
fn an_undefined_entity_in_an_attribute_is_not_expanded() {
    let p = wrapped(r#"  <rdf:Description rdf:about="&xxe;"/>"#);
    assert!(matches!(Packet::parse(p.as_bytes()), Err(ParseError::Syntax { .. })));
}

#[test]
fn malformed_packets_name_their_problem() {
    let cases: [(&[u8], &str); 6] = [
        (
            b"<rdf:RDF xmlns:rdf='http://www.w3.org/1999/02/22-rdf-syntax-ns#'><a></b></rdf:RDF>",
            "syntax",
        ),
        (b"<x:xmpmeta xmlns:x='adobe:ns:meta/'/>", "no-rdf"),
        (b"<rdf:RDF/>", "prefix"),
        (b"\xFF\x00\x80", "encoding"),
        (
            b"<rdf:RDF xmlns:rdf='http://www.w3.org/1999/02/22-rdf-syntax-ns#'>",
            "syntax",
        ),
        (
            b"<rdf:RDF xmlns:rdf='http://www.w3.org/1999/02/22-rdf-syntax-ns#' a='1'b='2'/>",
            "syntax",
        ),
    ];
    let reasons: Vec<_> = cases
        .iter()
        .map(|(b, _)| Packet::parse(b).unwrap_err().reason())
        .collect();
    assert_eq!(reasons, cases.map(|(_, r)| r));
}

#[test]
fn nesting_beyond_the_limit_is_refused() {
    let deep = format!("{RDF_OPEN}{}{}</rdf:RDF>", "<a>".repeat(100), "</a>".repeat(100));
    assert!(matches!(
        Packet::parse(deep.as_bytes()),
        Err(ParseError::TooDeep { .. })
    ));
}

#[test]
fn arbitrary_bytes_never_panic() {
    let seed = wrapped(
        r#"  <rdf:Description rdf:about="" xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title><rdf:Alt><rdf:li xml:lang="x-default">T</rdf:li></rdf:Alt></dc:title></rdf:Description>"#,
    );
    let mut rng = laminapdf_test_fixtures::Rng::new(7);
    for _ in 0..2000 {
        let mut bytes = seed.clone().into_bytes();
        for _ in 0..=rng.below(4) {
            let at = rng.below(bytes.len());
            bytes[at] = u8::try_from(rng.below(256)).unwrap();
        }
        if let Ok(mut p) = Packet::parse(&bytes) {
            p.set_title(Some("edited"));
            p.set_authors(&["a"]);
            let _ = Packet::parse(&p.write().bytes);
        }
    }
}

#[test]
fn an_unchanged_packet_writes_back_byte_for_byte() {
    let source = wrapped(
        r#"  <rdf:Description rdf:about=""  xmlns:dc="http://purl.org/dc/elements/1.1/"><!-- kept --></rdf:Description>"#,
    );
    let mut p = Packet::parse(source.as_bytes()).unwrap();
    p.set_title(Some("x"));
    p.set_title(None);
    assert_eq!((p.write().bytes, p.is_changed()), (source.into_bytes(), false));
}

#[test]
fn setting_an_element_replaces_only_its_bytes_and_keeps_the_length() {
    let source = wrapped(
        r#"  <rdf:Description rdf:about="" xmlns:dc="http://purl.org/dc/elements/1.1/">
   <!-- a comment -->
   <dc:title><rdf:Alt><rdf:li xml:lang="x-default">Old</rdf:li></rdf:Alt></dc:title>
   <dc:format>application/pdf</dc:format>
  </rdf:Description>"#,
    );
    let mut p = Packet::parse(source.as_bytes()).unwrap();
    p.set_title(Some("New"));
    // "New" is as long as "Old", so the padding stays as it was too.
    assert_eq!(text(&p.write()).replace("New", "Old"), source);
}

#[test]
fn a_shorter_packet_is_padded_back_to_its_length() {
    let source = wrapped(
        r#"  <rdf:Description rdf:about="" xmlns:pdf="http://ns.adobe.com/pdf/1.3/"><pdf:Producer>A long producer name</pdf:Producer></rdf:Description>"#,
    );
    let mut p = Packet::parse(source.as_bytes()).unwrap();
    p.set_producer(None);
    let out = text(&p.write());
    assert_eq!((out.len(), out.contains("Producer")), (source.len(), false));
}

#[test]
fn setting_an_attribute_keeps_its_quotes() {
    let mut p =
        packet(r"  <rdf:Description rdf:about='' xmlns:pdf='http://ns.adobe.com/pdf/1.3/' pdf:Producer='Old'/>");
    p.set_producer(Some("It's new"));
    assert!(text(&p.write()).contains("pdf:Producer='It&apos;s new'"));
}

#[test]
fn an_array_set_on_an_attribute_becomes_an_element_of_an_empty_description() {
    let mut p =
        packet(r#"  <rdf:Description rdf:about="" xmlns:dc="http://purl.org/dc/elements/1.1/" dc:title="Old"/>"#);
    p.set_title(Some("New"));
    let out = text(&p.write());
    assert!(
        out.contains(r#"xmlns:dc="http://purl.org/dc/elements/1.1/">"#)
            && out.contains(r#"<dc:title><rdf:Alt><rdf:li xml:lang="x-default">New</rdf:li></rdf:Alt></dc:title>"#)
            && out.contains("</rdf:Description>")
            && !out.contains("dc:title=")
    );
}

#[test]
fn a_new_property_goes_to_the_description_declaring_its_namespace() {
    let mut p = packet(
        r#"  <rdf:Description rdf:about="" xmlns:pdf="http://ns.adobe.com/pdf/1.3/"><pdf:Producer>P</pdf:Producer></rdf:Description>
  <rdf:Description rdf:about="" xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:format>application/pdf</dc:format></rdf:Description>"#,
    );
    p.set_title(Some("T"));
    let out = text(&p.write());
    assert!(out.contains("<dc:format>application/pdf</dc:format>\n<dc:title>"));
}

#[test]
fn a_new_namespace_is_declared_with_its_standard_prefix() {
    let mut p = packet(r#"  <rdf:Description rdf:about="" xmlns:pdf="http://ns.adobe.com/pdf/1.3/"/>"#);
    p.set_creator_tool(Some("Tool"));
    let out = Packet::parse(&p.write().bytes).unwrap();
    assert_eq!(out.creator_tool().as_deref(), Some("Tool"));
}

#[test]
fn a_standard_prefix_bound_elsewhere_gets_a_fresh_one() {
    let mut p = packet(
        r#"  <rdf:Description rdf:about="" xmlns:dc="urn:not-dublin-core"><dc:title>x</dc:title></rdf:Description>"#,
    );
    p.set_title(Some("T"));
    let out = text(&p.write());
    let reread = Packet::parse(out.as_bytes()).unwrap();
    assert!(out.contains(r#"xmlns:dc1="http://purl.org/dc/elements/1.1/""#) && reread.title().as_deref() == Some("T"));
}

#[test]
fn removing_a_property_leaves_no_blank_line() {
    let mut p = packet(
        r#"  <rdf:Description rdf:about="" xmlns:pdf="http://ns.adobe.com/pdf/1.3/">
   <pdf:Producer>P</pdf:Producer>
   <pdf:Keywords>k</pdf:Keywords>
  </rdf:Description>"#,
    );
    p.set_keywords(None);
    assert!(text(&p.write()).contains("<pdf:Producer>P</pdf:Producer>\n  </rdf:Description>"));
}

#[test]
fn duplicates_of_a_property_are_replaced_by_one() {
    let mut p = packet(
        r#"  <rdf:Description rdf:about="" xmlns:pdf="http://ns.adobe.com/pdf/1.3/" pdf:Producer="A"/>
  <rdf:Description rdf:about="" xmlns:pdf="http://ns.adobe.com/pdf/1.3/"><pdf:Producer>B</pdf:Producer></rdf:Description>"#,
    );
    p.set_producer(Some("C"));
    let out = Packet::parse(&p.write().bytes).unwrap();
    assert_eq!(out.properties().filter(|q| q.name == "Producer").count(), 1);
}

#[test]
fn a_longer_packet_gets_fresh_padding() {
    let mut p = packet(r#"  <rdf:Description rdf:about="" xmlns:pdf="http://ns.adobe.com/pdf/1.3/"/>"#);
    p.set_keywords(Some(&"k".repeat(500)));
    let out = text(&p.write());
    let padding =
        out.len() - out.trim_end_matches("<?xpacket end=\"w\"?>").trim_end().len() - "<?xpacket end=\"w\"?>".len();
    assert_eq!(padding, 2048);
}

#[test]
fn a_bytes_attribute_in_the_header_is_dropped_on_edit() {
    let source = wrapped(r#"  <rdf:Description rdf:about="" xmlns:pdf="http://ns.adobe.com/pdf/1.3/"/>"#).replace(
        "id=\"W5M0MpCehiHzreSzNTczkc9d\"",
        "id=\"W5M0MpCehiHzreSzNTczkc9d\" bytes=\"1234\"",
    );
    let mut p = Packet::parse(source.as_bytes()).unwrap();
    p.set_producer(Some("P"));
    assert!(!text(&p.write()).contains("bytes="));
}

#[test]
fn a_utf16_packet_is_read_and_rewritten_canonically() {
    let source = wrapped(
        r#"  <rdf:Description rdf:about="" xmlns:pdf="http://ns.adobe.com/pdf/1.3/" pdf:Keywords="k"><!-- c --><pdf:Producer>P</pdf:Producer></rdf:Description>"#,
    );
    let utf16: Vec<u8> = [0xFE, 0xFF]
        .into_iter()
        .chain(source.encode_utf16().flat_map(u16::to_be_bytes))
        .collect();
    let mut p = Packet::parse(&utf16).unwrap();
    assert_eq!(p.producer().as_deref(), Some("P"));
    p.set_title(Some("T"));
    let written = p.write();
    let reread = Packet::parse(&written.bytes).unwrap();
    assert_eq!(
        (
            written.canonical,
            reread.title().as_deref(),
            others(&reread, TITLE),
            others(&p, TITLE)
        ),
        (true, Some("T"), others(&p, TITLE), others(&p, TITLE))
    );
}

#[test]
fn values_of_the_wrong_shape_are_refused() {
    let mut p = Packet::new_minimal();
    assert_eq!(
        p.set(MappedField::Title, Some(Value::List(vec!["a".into()]))),
        Err(KindMismatch {
            field: MappedField::Title
        })
    );
}

#[test]
fn the_minimal_packet_declares_the_standard_namespaces_and_pads() {
    let p = Packet::new_minimal();
    let written = text(&p.write());
    assert!(
        written.starts_with(write::HEADER)
            && written.ends_with(write::TRAILER)
            && [DC, XMP, PDF, XMP_MM].iter().all(|ns| written.contains(ns))
            && !written.contains("bytes=")
            && written.len() > write::PADDING
    );
}

#[test]
fn every_mapped_field_round_trips_through_a_minimal_packet() {
    let date: PdfDate = "D:20261028143000+01'00".parse().unwrap();
    let mut p = Packet::new_minimal();
    p.set_title(Some("T"));
    p.set_authors(&["A", "B"]);
    p.set_subject(Some("S"));
    p.set_keywords(Some("K"));
    p.set_creator_tool(Some("C"));
    p.set_producer(Some("P"));
    p.set_create_date(Some(date));
    p.set_modify_date(Some(date));
    p.set_metadata_date(Some(date));
    p.set(MappedField::DocumentId, Some(Value::Text("uuid:d".into())))
        .unwrap();
    p.set(MappedField::InstanceId, Some(Value::Text("uuid:i".into())))
        .unwrap();
    let written = p.write();
    let reread = Packet::parse(&written.bytes).unwrap();
    let read = |q: &Packet| MAPPINGS.map(|m| q.get(m.field));
    assert_eq!(
        (read(&reread), written.bytes.len(), written.canonical),
        (read(&p), Packet::new_minimal().original().len(), false)
    );
}
