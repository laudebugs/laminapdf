//! The XMP metadata packet (ISO 32000-2 §14.3.2; ISO 16684-1; research 04 §5.2), written
//! from the same [`Metadata`] as the Info dictionary so the two agree.
//!
//! The packet is deterministic: one `rdf:Description` with the namespaces in a fixed order,
//! properties in a fixed order, values escaped for XML, no padding.

use laminapdf_model::xmp::{MAPPINGS, MappedField, Value, property_element};

use super::info::Metadata;
use crate::profile::Conformance;

/// The PDF/A extension schema describing `pdfuaid:part` (ISO 19005-2 §6.6.2.3.1): PDF/A
/// allows only predefined XMP properties unless a schema declares them, and the PDF/UA
/// identification isn't predefined. Written only when the document is both.
const PDFUA_EXTENSION: &str = "  <rdf:Description rdf:about=\"\" \
     xmlns:pdfaExtension=\"http://www.aiim.org/pdfa/ns/extension/\" \
     xmlns:pdfaSchema=\"http://www.aiim.org/pdfa/ns/schema#\" \
     xmlns:pdfaProperty=\"http://www.aiim.org/pdfa/ns/property#\">
   <pdfaExtension:schemas>
    <rdf:Bag>
     <rdf:li rdf:parseType=\"Resource\">
      <pdfaSchema:schema>PDF/UA Universal Accessibility Schema</pdfaSchema:schema>
      <pdfaSchema:namespaceURI>http://www.aiim.org/pdfua/ns/id/</pdfaSchema:namespaceURI>
      <pdfaSchema:prefix>pdfuaid</pdfaSchema:prefix>
      <pdfaSchema:property>
       <rdf:Seq>
        <rdf:li rdf:parseType=\"Resource\">
         <pdfaProperty:name>part</pdfaProperty:name>
         <pdfaProperty:valueType>Integer</pdfaProperty:valueType>
         <pdfaProperty:category>internal</pdfaProperty:category>
         <pdfaProperty:description>Indicates, which part of ISO 14289 standard is followed</pdfaProperty:description>
        </rdf:li>
       </rdf:Seq>
      </pdfaSchema:property>
     </rdf:li>
    </rdf:Bag>
   </pdfaExtension:schemas>
  </rdf:Description>
";

/// The packet: `<?xpacket … ?>` around `x:xmpmeta`, UTF-8, with the profiles'
/// identification (`pdfaid:part` and `pdfaid:conformance`, ISO 19005-2 §6.6.4; `pdfuaid:part`,
/// ISO 14289-1 §5) when there are any.
pub(crate) fn packet(meta: &Metadata, conformance: Conformance) -> String {
    let mut body = String::new();
    let mut line = |s: &str| {
        body.push_str("   ");
        body.push_str(s);
        body.push('\n');
    };
    line("<dc:format>application/pdf</dc:format>");
    // The fields in the shared table's order, each written as the editor writes it.
    for mapping in &MAPPINGS {
        let value = match mapping.field {
            MappedField::Title => meta.title.clone().map(Value::Text),
            MappedField::Authors => meta.author.clone().map(|a| Value::List(vec![a])),
            MappedField::Subject => meta.subject.clone().map(Value::Text),
            MappedField::Keywords => meta.keywords.clone().map(Value::Text),
            MappedField::Producer => meta.producer.clone().map(Value::Text),
            MappedField::Creator => meta.creator.clone().map(Value::Text),
            MappedField::Created => meta.creation_date.map(Value::Date),
            MappedField::Modified => meta.mod_date.map(Value::Date),
            // Not part of `Metadata`: the editor stamps them (E1 plan §2).
            _ => None,
        };
        if let Some(value) = value {
            line(&property_element(mapping, &value));
        }
    }
    let mut namespaces = String::new();
    if let Some(pdfa) = conformance.pdfa {
        namespaces.push_str(" xmlns:pdfaid=\"http://www.aiim.org/pdfa/ns/id/\"");
        line(&format!("<pdfaid:part>{}</pdfaid:part>", pdfa.part()));
        line(&format!(
            "<pdfaid:conformance>{}</pdfaid:conformance>",
            pdfa.conformance()
        ));
    }
    if conformance.ua {
        namespaces.push_str(" xmlns:pdfuaid=\"http://www.aiim.org/pdfua/ns/id/\"");
        line("<pdfuaid:part>1</pdfuaid:part>");
    }
    let extension = if conformance.ua && conformance.pdfa.is_some() {
        PDFUA_EXTENSION
    } else {
        ""
    };
    format!(
        "<?xpacket begin=\"\u{FEFF}\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?>\n\
         <x:xmpmeta xmlns:x=\"adobe:ns:meta/\">\n \
         <rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\">\n  \
         <rdf:Description rdf:about=\"\" \
         xmlns:dc=\"http://purl.org/dc/elements/1.1/\" \
         xmlns:pdf=\"http://ns.adobe.com/pdf/1.3/\" \
         xmlns:xmp=\"http://ns.adobe.com/xap/1.0/\"{namespaces}>\n\
         {body}  </rdf:Description>\n\
         {extension} \
         </rdf:RDF>\n\
         </x:xmpmeta>\n\
         <?xpacket end=\"w\"?>"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_packet_holds_every_value_once() {
        let meta = Metadata::default().with_title("T").with_producer("P");
        let p = packet(&meta, Conformance::default());
        assert!(
            p.contains("<rdf:li xml:lang=\"x-default\">T</rdf:li>") && p.contains("<pdf:Producer>P</pdf:Producer>")
        );
        assert!(!p.contains("dc:creator"));
    }

    #[test]
    fn a_pdfa_and_ua_packet_identifies_both_and_declares_the_ua_schema() {
        use crate::profile::Profile;
        let c = Conformance::default().with(Profile::PdfA2u).with(Profile::PdfUa1);
        let p = packet(&Metadata::default(), c);
        assert!(
            p.contains("<pdfaid:part>2</pdfaid:part>")
                && p.contains("<pdfaid:conformance>U</pdfaid:conformance>")
                && p.contains("<pdfuaid:part>1</pdfuaid:part>")
                && p.contains("<pdfaSchema:prefix>pdfuaid</pdfaSchema:prefix>")
        );
    }

    #[test]
    fn a_pdfa_only_packet_declares_no_extension_schema() {
        use crate::profile::Profile;
        let p = packet(&Metadata::default(), Conformance::default().with(Profile::PdfA3b));
        assert!(p.contains("<pdfaid:part>3</pdfaid:part>") && !p.contains("pdfaExtension"));
    }
}
