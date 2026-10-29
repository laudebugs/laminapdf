//! Shared helpers for `lamina-text`'s tests and its golden generator: fixture fonts, a minimal
//! test page writer (B4a needs real PDFs before the builder, B5, exists) and the sample
//! documents behind the `text-*.pdf` goldens.
//!
//! The page writer places glyphs with `pdf_writer::Content` on one A4 page of a document
//! whose base is an empty catalog (written by `lamina-save`'s spike-era `Writer`); the page,
//! its content and the fonts go into the overlay, and a full rewrite produces the file.
#![allow(
    dead_code,
    unreachable_pub,
    reason = "each test crate uses a different subset of the helpers"
)]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure"
)]

pub mod samples;

use std::path::{Path, PathBuf};

use lamina_cos::{Dict, Document, Name, ObjRef, Object, OpenOptions, Stream};
use lamina_save::{FullWriter, Renumber, SaveOptions, Writer, XrefKind};
use lamina_text::{EmbedPolicy, EmbeddedFont, Font, FontKey, FontRegistry, GlyphId, Result, Warning};
use pdf_writer::{Content, Str};

/// The repository's shared `fixtures/` directory (docs/guidelines/code-organization.md §6).
pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

pub fn fixture(rel: &str) -> Vec<u8> {
    let path = fixtures_dir().join(rel);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

/// A font from `fixtures/fonts/`.
pub fn font(name: &str) -> Font {
    Font::from_bytes(fixture(&format!("fonts/{name}"))).unwrap()
}

/// `bytes` with `OS/2.fsType` set to `fs_type` (the table checksum is left stale; nothing
/// checks it).
pub fn with_fs_type(bytes: &[u8], fs_type: u16) -> Vec<u8> {
    let num_tables = usize::from(u16::from_be_bytes([bytes[4], bytes[5]]));
    let mut out = bytes.to_vec();
    for i in 0..num_tables {
        let record = 12 + 16 * i;
        if &bytes[record..record + 4] == b"OS/2" {
            let offset = u32::from_be_bytes(bytes[record + 8..record + 12].try_into().unwrap()) as usize;
            out[offset + 8..offset + 10].copy_from_slice(&fs_type.to_be_bytes());
            return out;
        }
    }
    panic!("no OS/2 table");
}

/// A4, in points.
const PAGE: (f32, f32) = (595.0, 842.0);

/// One page of text, one line per call, built into a document's overlay.
pub struct TestPdf {
    doc: Document,
    fonts: FontRegistry,
    content: Content,
    /// Resource name → font object, in order of first use.
    resources: Vec<(String, ObjRef)>,
    y: f32,
}

impl TestPdf {
    /// A document whose base is an empty catalog and page tree.
    pub fn new() -> Self {
        let mut w = Writer::new(1, 7);
        let pages = w.add(
            [
                ("Type", Object::name("Pages")),
                ("Kids", Object::Array(Vec::new())),
                ("Count", 0.into()),
            ]
            .into_iter()
            .collect::<Dict>(),
        );
        let root = w.add(
            [("Type", Object::name("Catalog")), ("Pages", pages.into())]
                .into_iter()
                .collect::<Dict>(),
        );
        let base = w.finish(&[("Root", root)].into_iter().collect()).unwrap();
        Self {
            doc: Document::open(base, OpenOptions::default()).unwrap(),
            fonts: FontRegistry::new(),
            content: Content::new(),
            resources: Vec::new(),
            y: PAGE.1 - 72.0,
        }
    }

    pub fn add_font(&mut self, font: Font) -> Result<FontKey> {
        self.add_font_with(font, EmbedPolicy::Respect)
    }

    pub fn add_font_with(&mut self, font: Font, policy: EmbedPolicy) -> Result<FontKey> {
        self.fonts.add(font, policy)
    }

    pub fn registry(&self) -> &FontRegistry {
        &self.fonts
    }

    /// One line of `text`, one glyph per character (the font's `cmap`).
    pub fn line(&mut self, key: FontKey, size: f32, text: &str) -> Result<()> {
        let encoded = self.fonts.encode(key, text)?;
        self.show(key, size, &encoded.bytes)
    }

    /// One line of explicit glyphs, each with the text it stands for (what a shaper produces:
    /// a ligature glyph standing for "ffi").
    pub fn glyphs(&mut self, key: FontKey, size: f32, glyphs: &[(GlyphId, &str)]) -> Result<()> {
        let mut bytes = Vec::new();
        for &(gid, text) in glyphs {
            let cid = self.fonts.use_glyph(key, gid, text)?;
            bytes.extend_from_slice(&cid.to_be_bytes());
        }
        self.show(key, size, &bytes)
    }

    fn show(&mut self, key: FontKey, size: f32, bytes: &[u8]) -> Result<()> {
        let object = self.fonts.reference(key, &self.doc)?;
        let name = if let Some((name, _)) = self.resources.iter().find(|(_, r)| *r == object) {
            name.clone()
        } else {
            let name = format!("F{}", self.resources.len() + 1);
            self.resources.push((name.clone(), object));
            name
        };
        self.y -= size * 1.5;
        self.content
            .begin_text()
            .set_font(pdf_writer::Name(name.as_bytes()), size)
            .next_line(72.0, self.y)
            .show(Str(bytes))
            .end_text();
        Ok(())
    }

    /// Writes the page and the fonts into the overlay and saves a full rewrite (compact
    /// numbering, classic xref). Returns the file, what was embedded and the warnings.
    pub fn finish(self) -> Result<(Vec<u8>, Vec<EmbeddedFont>, Vec<Warning>)> {
        let Self {
            mut doc,
            fonts,
            content,
            resources,
            ..
        } = self;
        let warnings = fonts.warnings().to_vec();
        let root = match doc.trailer().get("Root") {
            Some(Object::Ref(r)) => *r,
            other => panic!("no /Root: {other:?}"),
        };
        let pages = match doc.resolve(root)?.as_ref() {
            Object::Dict(d) => match d.get("Pages") {
                Some(Object::Ref(r)) => *r,
                other => panic!("no /Pages: {other:?}"),
            },
            other => panic!("catalog is not a dictionary: {other:?}"),
        };
        let embedded = {
            let mut cos = doc.edit();
            let stream = cos.create(Object::Stream(Stream::flate(Dict::new(), &content.finish().into_vec())))?;
            let font_dict: Dict = resources
                .iter()
                .map(|(name, r)| (Name::from(name.as_str()), Object::from(*r)))
                .collect();
            let mut resources_dict = Dict::new();
            resources_dict.insert("Font", font_dict);
            let mut page = Dict::new();
            page.insert("Type", Name::from("Page"));
            page.insert("Parent", pages);
            page.insert(
                "MediaBox",
                Object::Array(vec![
                    0.into(),
                    0.into(),
                    Object::Real(f64::from(PAGE.0)),
                    Object::Real(f64::from(PAGE.1)),
                ]),
            );
            page.insert("Resources", resources_dict);
            page.insert("Contents", stream);
            let page = cos.create(Object::Dict(page))?;
            let mut tree = Dict::new();
            tree.insert("Type", Name::from("Pages"));
            tree.insert("Kids", Object::Array(vec![page.into()]));
            tree.insert("Count", 1);
            cos.set(pages, Object::Dict(tree))?;
            fonts.finish_fonts(&mut cos)?
        };
        let mut out = Vec::new();
        let options = SaveOptions::new()
            .with_renumber(Renumber::Compact)
            .with_xref(XrefKind::Classic);
        FullWriter::new(&doc, options).write(&mut out).unwrap();
        Ok((out, embedded, warnings))
    }
}

impl Default for TestPdf {
    fn default() -> Self {
        Self::new()
    }
}

/// A Type 0 font read back from a generated file with `lamina-cos`.
pub struct ReadFont {
    pub type0: Dict,
    pub cid_font: Dict,
    pub descriptor: Dict,
    /// The decoded font program (`/FontFile2` or `/FontFile3`).
    pub program: Vec<u8>,
    /// The font file stream's dictionary.
    pub program_dict: Dict,
    /// The decoded ToUnicode CMap.
    pub to_unicode: String,
}

/// Opens `pdf` with lamina-cos and reads every font in the first page's `/Resources /Font`,
/// sorted by resource name.
pub fn read_fonts(pdf: &[u8]) -> Vec<ReadFont> {
    let doc = Document::open(pdf.to_vec(), OpenOptions::default()).unwrap();
    let page = doc.pages().unwrap().remove(0);
    let resources = page.resources_dict().unwrap().clone();
    let fonts = doc.deref(resources.get("Font").unwrap()).unwrap();
    let dict = |o: &Object| doc.deref(o).unwrap().as_dict().unwrap().clone();
    let stream = |o: &Object| match &*doc.deref(o).unwrap() {
        Object::Stream(s) => (s.dict.clone(), doc.decode_stream(s).unwrap()),
        other => panic!("not a stream: {other:?}"),
    };
    fonts
        .as_dict()
        .unwrap()
        .iter()
        .map(|(_, r)| {
            let type0 = dict(r);
            let cid_font = dict(&type0.get("DescendantFonts").unwrap().as_array().unwrap()[0]);
            let descriptor = dict(cid_font.get("FontDescriptor").unwrap());
            let file = descriptor
                .get("FontFile2")
                .or_else(|| descriptor.get("FontFile3"))
                .unwrap();
            let (program_dict, program) = stream(file);
            let (_, cmap) = stream(type0.get("ToUnicode").unwrap());
            ReadFont {
                type0,
                cid_font,
                descriptor,
                program,
                program_dict,
                to_unicode: String::from_utf8(cmap).unwrap(),
            }
        })
        .collect()
}

/// A name entry's text.
pub fn name_of(dict: &Dict, key: &str) -> String {
    match dict.get(key) {
        Some(Object::Name(n)) => String::from_utf8_lossy(n.as_bytes()).into_owned(),
        other => panic!("/{key} is not a name: {other:?}"),
    }
}

/// Expands a `/W` array (§9.7.4.3) into CID → width.
pub fn expand_widths(w: &[Object]) -> std::collections::BTreeMap<u16, f64> {
    let mut out = std::collections::BTreeMap::new();
    let mut i = 0;
    while i < w.len() {
        let first = u16::try_from(w[i].as_int().unwrap()).unwrap();
        match &w[i + 1] {
            Object::Array(ws) => {
                for (cid, width) in (first..).zip(ws) {
                    out.insert(cid, width.as_f64().unwrap());
                }
                i += 2;
            }
            last => {
                let last = u16::try_from(last.as_int().unwrap()).unwrap();
                let width = w[i + 2].as_f64().unwrap();
                for cid in first..=last {
                    out.insert(cid, width);
                }
                i += 3;
            }
        }
    }
    out
}

/// Parses the `bfchar` and `bfrange` entries of a ToUnicode CMap (the subset lamina writes:
/// two-byte codes, incrementing ranges) into code → text, and returns the entry count of
/// every block.
pub fn parse_cmap(cmap: &str) -> (std::collections::BTreeMap<u16, String>, Vec<usize>) {
    let hex = |s: &str| -> Vec<u16> {
        let s = s.trim_matches(|c| c == '<' || c == '>');
        (0..s.len() / 4)
            .map(|i| u16::from_str_radix(&s[i * 4..i * 4 + 4], 16).unwrap())
            .collect()
    };
    let mut map = std::collections::BTreeMap::new();
    let mut blocks = Vec::new();
    let mut mode = None;
    for line in cmap.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        match parts.as_slice() {
            [n, "beginbfchar"] => {
                blocks.push(n.parse().unwrap());
                mode = Some("char");
            }
            [n, "beginbfrange"] => {
                blocks.push(n.parse().unwrap());
                mode = Some("range");
            }
            ["endbfchar" | "endbfrange"] => mode = None,
            [src, dst] if mode == Some("char") => {
                map.insert(hex(src)[0], String::from_utf16(&hex(dst)).unwrap());
            }
            [lo, hi, dst] if mode == Some("range") => {
                let (lo, hi, dst) = (hex(lo)[0], hex(hi)[0], hex(dst));
                for (i, code) in (lo..=hi).enumerate() {
                    let mut units = dst.clone();
                    *units.last_mut().unwrap() += u16::try_from(i).unwrap();
                    map.insert(code, String::from_utf16(&units).unwrap());
                }
            }
            _ => {}
        }
    }
    (map, blocks)
}

/// `bytes` (two-byte codes) mapped through `cmap`: the text an extractor gets.
pub fn extract(bytes: &[u8], cmap: &std::collections::BTreeMap<u16, String>) -> String {
    bytes
        .chunks(2)
        .map(|c| cmap.get(&u16::from_be_bytes([c[0], c[1]])).cloned().unwrap_or_default())
        .collect()
}
