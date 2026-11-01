//! The XMP metadata packet (ISO 32000-2 §14.3.2; ISO 16684-1; research 04 §5.2), written
//! from the same [`Metadata`] as the Info dictionary so the two agree.
//!
//! The packet is deterministic: one `rdf:Description` with the namespaces in a fixed order,
//! properties in a fixed order, values escaped for XML, no padding.

use std::fmt::Write as _;

use laminapdf_model::PdfDate;

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

/// `&`, `<`, `>` and `"` escaped; tab, line feed and carriage return as character references;
/// other characters XML 1.0 can't hold (C0 controls, U+FFFE, U+FFFF) are left out.
pub(crate) fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\t' => out.push_str("&#x9;"),
            '\n' => out.push_str("&#xA;"),
            '\r' => out.push_str("&#xD;"),
            c if super::info::not_xml(c) => {}
            _ => out.push(ch),
        }
    }
    out
}

/// A date as XMP writes it (ISO 8601): `2026-10-29T12:00:00Z`, `…+01:00`, or no zone when
/// the PDF date gives none.
pub(crate) fn date(d: &PdfDate) -> String {
    let mut s = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        d.year(),
        d.month(),
        d.day(),
        d.hour(),
        d.minute(),
        d.second()
    );
    match d.utc_offset_minutes() {
        None => {}
        Some(0) => s.push('Z'),
        Some(m) => {
            let sign = if m < 0 { '-' } else { '+' };
            let m = m.unsigned_abs();
            let _ = write!(s, "{sign}{:02}:{:02}", m / 60, m % 60);
        }
    }
    s
}

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
    if let Some(title) = &meta.title {
        line(&format!(
            "<dc:title><rdf:Alt><rdf:li xml:lang=\"x-default\">{}</rdf:li></rdf:Alt></dc:title>",
            escape(title)
        ));
    }
    if let Some(author) = &meta.author {
        line(&format!(
            "<dc:creator><rdf:Seq><rdf:li>{}</rdf:li></rdf:Seq></dc:creator>",
            escape(author)
        ));
    }
    if let Some(subject) = &meta.subject {
        line(&format!(
            "<dc:description><rdf:Alt><rdf:li xml:lang=\"x-default\">{}</rdf:li></rdf:Alt></dc:description>",
            escape(subject)
        ));
    }
    for (tag, value) in [
        ("pdf:Keywords", &meta.keywords),
        ("pdf:Producer", &meta.producer),
        ("xmp:CreatorTool", &meta.creator),
    ] {
        if let Some(v) = value {
            line(&format!("<{tag}>{}</{tag}>", escape(v)));
        }
    }
    for (tag, value) in [
        ("xmp:CreateDate", &meta.creation_date),
        ("xmp:ModifyDate", &meta.mod_date),
    ] {
        if let Some(d) = value {
            line(&format!("<{tag}>{}</{tag}>", date(d)));
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
    fn values_are_escaped() {
        assert_eq!(escape("a<b & \"c\">\u{1}\n"), "a&lt;b &amp; &quot;c&quot;&gt;&#xA;");
    }

    #[test]
    fn dates_carry_their_zone() {
        let d = |s: &str| date(&s.parse().unwrap());
        assert_eq!(
            [d("D:20261029120000Z"), d("D:20261029120000-05'30'"), d("D:2026")],
            [
                "2026-10-29T12:00:00Z",
                "2026-10-29T12:00:00-05:30",
                "2026-01-01T00:00:00"
            ]
        );
    }

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
