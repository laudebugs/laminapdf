//! Experiment C: the same three pages with a minimal builder directly over laminapdf-cos.
//!
//! Reused building blocks: `pdf-writer::Content` for content-stream operators (bytes only, it
//! never allocates object numbers), `skrifa` for cmap and metrics, `subsetter` for the font
//! subset, `png` for decoding. Objects are lamina `Object`s held by a `laminapdf_cos::Writer`;
//! the font objects are *reserved* up front and filled in at `finish()`, which is all that
//! "subsetting needs the whole document" requires of the graph.
//!
//! Not done (B4/B5 work): shaping (no ligatures/kerning: one glyph per char via the cmap),
//! bidi, line breaking, tagging, XMP.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use anyhow::{Result, anyhow};
use laminapdf_cos::{Dict, Name, ObjRef, Object, PdfString, Stream};
use laminapdf_save::Writer;
use pdf_writer::{Content, Str};
use skrifa::instance::{LocationRef, Size};
use skrifa::raw::TableProvider;
use skrifa::{FontRef, MetadataProvider};
use subsetter::GlyphRemapper;

use crate::assets::{Assets, IMAGE_SIDE, LINK_URI, PAGE_H, PAGE_W, PAGES, TITLE};

fn dict<const N: usize>(entries: [(&str, Object); N]) -> Dict {
    entries.into_iter().collect()
}

fn reals(values: &[f32]) -> Object {
    Object::Array(values.iter().map(|&v| Object::Real(f64::from(v))).collect())
}

/// One embedded TrueType font, subset at `finish`.
struct EmbeddedFont<'f> {
    data: &'f [u8],
    font: FontRef<'f>,
    /// Reserved: the Type 0 font dictionary pages refer to before the subset exists.
    type0: ObjRef,
    remapper: GlyphRemapper,
    /// New glyph id (= CID, CIDToGIDMap /Identity) → text it stands for.
    unicode: BTreeMap<u16, String>,
    units_per_em: f32,
}

impl<'f> EmbeddedFont<'f> {
    fn new(data: &'f [u8], type0: ObjRef) -> Result<Self> {
        let font = FontRef::new(data)?;
        let units_per_em = f32::from(font.head()?.units_per_em());
        let mut remapper = GlyphRemapper::new();
        remapper.remap(0); // .notdef stays glyph 0
        Ok(Self {
            data,
            font,
            type0,
            remapper,
            unicode: BTreeMap::new(),
            units_per_em,
        })
    }

    /// CIDs for `text` (two bytes each, Identity-H) and its advance in text-space units / 1000.
    fn encode(&mut self, text: &str) -> (Vec<u8>, f32) {
        let charmap = self.font.charmap();
        let metrics = self.font.glyph_metrics(Size::unscaled(), LocationRef::default());
        let mut bytes = Vec::with_capacity(text.len() * 2);
        let mut advance = 0.0;
        for ch in text.chars() {
            let gid = charmap.map(ch).unwrap_or_default();
            let old = u16::try_from(gid.to_u32()).unwrap_or(0);
            let cid = self.remapper.remap(old);
            self.unicode.entry(cid).or_insert_with(|| ch.to_string());
            advance += metrics.advance_width(gid).unwrap_or(0.0) * 1000.0 / self.units_per_em;
            bytes.extend_from_slice(&cid.to_be_bytes());
        }
        (bytes, advance)
    }

    fn finish(self, w: &mut Writer) -> Result<usize> {
        let subset = subsetter::subset(self.data, 0, &self.remapper).map_err(|e| anyhow!("subsetter: {e:?}"))?;
        let subset_len = subset.len();
        let metrics = self.font.metrics(Size::unscaled(), LocationRef::default());
        let scale = 1000.0 / self.units_per_em;
        let ps_name = self
            .font
            .localized_strings(skrifa::string::StringId::POSTSCRIPT_NAME)
            .english_or_first()
            .map_or_else(|| "Font".to_owned(), |s| s.to_string());
        // Deterministic subset tag from the glyph set (§9.9.2 asks for six capitals).
        let mut h: u32 = 2_166_136_261;
        for g in self.remapper.remapped_gids() {
            h = (h ^ u32::from(g)).wrapping_mul(16_777_619);
        }
        let tag: String = (0..6).map(|i| char::from(b'A' + ((h >> (i * 5)) % 26) as u8)).collect();
        let base_font = Name::new(format!("{tag}+{ps_name}"));

        let file = w.add(Stream::flate(
            dict([("Length1", Object::Integer(subset_len as i64))]),
            &subset,
        ));
        let bbox = metrics.bounds.map_or([0.0; 4], |b| {
            [b.x_min * scale, b.y_min * scale, b.x_max * scale, b.y_max * scale]
        });
        let descriptor = w.add(dict([
            ("Type", Object::name("FontDescriptor")),
            ("FontName", Object::Name(base_font.clone())),
            ("Flags", 4.into()),
            ("FontBBox", reals(&bbox)),
            ("ItalicAngle", 0.into()),
            ("Ascent", Object::Real(f64::from(metrics.ascent * scale))),
            ("Descent", Object::Real(f64::from(metrics.descent * scale))),
            (
                "CapHeight",
                Object::Real(f64::from(metrics.cap_height.unwrap_or(metrics.ascent) * scale)),
            ),
            ("StemV", 80.into()),
            ("FontFile2", file.into()),
        ]));

        // /W as one run per CID: [cid [w]] (krilla compacts runs; good enough here).
        let glyph_metrics = self.font.glyph_metrics(Size::unscaled(), LocationRef::default());
        let mut widths = Vec::new();
        for (cid, old) in self.remapper.remapped_gids().enumerate() {
            let adv = glyph_metrics
                .advance_width(skrifa::GlyphId::new(u32::from(old)))
                .unwrap_or(0.0)
                * scale;
            widths.push(Object::Integer(cid as i64));
            widths.push(Object::Array(vec![Object::Real(f64::from(adv.round()))]));
        }
        let cid_font = w.add(dict([
            ("Type", Object::name("Font")),
            ("Subtype", Object::name("CIDFontType2")),
            ("BaseFont", Object::Name(base_font.clone())),
            (
                "CIDSystemInfo",
                Object::Dict(dict([
                    ("Registry", Object::String(PdfString::literal("Adobe"))),
                    ("Ordering", Object::String(PdfString::literal("Identity"))),
                    ("Supplement", 0.into()),
                ])),
            ),
            ("FontDescriptor", descriptor.into()),
            ("DW", 0.into()),
            ("W", Object::Array(widths)),
            ("CIDToGIDMap", Object::name("Identity")),
        ]));
        let to_unicode = w.add(Stream::flate(Dict::new(), &to_unicode_cmap(&self.unicode)));
        w.set(
            self.type0,
            dict([
                ("Type", Object::name("Font")),
                ("Subtype", Object::name("Type0")),
                ("BaseFont", Object::Name(base_font)),
                ("Encoding", Object::name("Identity-H")),
                ("DescendantFonts", Object::Array(vec![cid_font.into()])),
                ("ToUnicode", to_unicode.into()),
            ]),
        );
        Ok(subset_len)
    }
}

fn to_unicode_cmap(map: &BTreeMap<u16, String>) -> Vec<u8> {
    let mut s = String::from(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
         /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
         /CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n\
         1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n",
    );
    let entries: Vec<_> = map.iter().filter(|(cid, _)| **cid != 0).collect();
    for chunk in entries.chunks(100) {
        let _ = writeln!(s, "{} beginbfchar", chunk.len());
        for (cid, text) in chunk {
            let utf16: String = text.encode_utf16().map(|u| format!("{u:04X}")).collect();
            let _ = writeln!(s, "<{cid:04X}> <{utf16}>");
        }
        s.push_str("endbfchar\n");
    }
    s.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n");
    s.into_bytes()
}

/// Result of a lamina-native build.
pub struct Built {
    pub bytes: Vec<u8>,
    pub font_subset_len: usize,
}

/// Builds the sample. `compact` writes an object stream + xref stream.
pub fn generate(assets: &Assets, compact: bool) -> Result<Built> {
    let mut w = Writer::new(1, 7);
    let pages_ref = w.reserve();
    let outline_ref = w.reserve();
    let font_ref = w.reserve();
    let mut font = EmbeddedFont::new(&assets.font, font_ref)?;

    // Images: PNG → RGB + /SMask (Flate), JPEG → DCTDecode pass-through.
    let (png_rgb, png_alpha) = decode_png(&assets.png_alpha)?;
    let side = i64::from(IMAGE_SIDE);
    let image_dict = |cs: &str| {
        dict([
            ("Type", Object::name("XObject")),
            ("Subtype", Object::name("Image")),
            ("Width", side.into()),
            ("Height", side.into()),
            ("ColorSpace", Object::name(cs)),
            ("BitsPerComponent", 8.into()),
        ])
    };
    let smask = w.add(Stream::flate(image_dict("DeviceGray"), &png_alpha));
    let mut png_dict = image_dict("DeviceRGB");
    png_dict.insert("SMask", smask);
    let png_ref = w.add(Stream::flate(png_dict, &png_rgb));
    let (jw, jh) = assets.jpeg_size;
    let mut jpeg_dict = image_dict("DeviceRGB");
    jpeg_dict.insert("Width", i64::from(jw));
    jpeg_dict.insert("Height", i64::from(jh));
    jpeg_dict.insert("Filter", Object::name("DCTDecode"));
    let jpeg_ref = w.add(Stream::new(jpeg_dict, assets.jpeg.clone()));
    let gs_half = w.add(dict([("Type", Object::name("ExtGState")), ("ca", Object::Real(0.6))]));

    let mut page_refs = Vec::new();
    for (index, (heading, lines)) in PAGES.iter().enumerate() {
        let mut c = Content::new();
        let mut text = |c: &mut Content, x: f32, y_top: f32, size: f32, s: &str| {
            let (bytes, _) = font.encode(s);
            c.begin_text()
                .set_font(pdf_writer::Name(b"F1"), size)
                .next_line(x, PAGE_H - y_top)
                .show(Str(&bytes))
                .end_text();
        };
        text(&mut c, 56.0, 70.0, 20.0, heading);
        for (i, line) in lines.iter().enumerate() {
            text(&mut c, 56.0, 100.0 + (i as f32 + 1.0) * 18.0, 12.0, line);
        }
        let mut xobjects = Dict::new();
        match index {
            0 => draw_vectors(&mut c),
            1 => {
                let s = IMAGE_SIDE as f32;
                c.save_state()
                    .transform([s, 0.0, 0.0, s, 56.0, PAGE_H - 200.0 - s])
                    .x_object(pdf_writer::Name(b"Im1"))
                    .restore_state();
                let (jw, jh) = (jw as f32, jh as f32);
                c.save_state()
                    .transform([jw, 0.0, 0.0, jh, 300.0, PAGE_H - 200.0 - jh])
                    .x_object(pdf_writer::Name(b"Im2"))
                    .restore_state();
                xobjects.insert("Im1", png_ref);
                xobjects.insert("Im2", jpeg_ref);
            }
            _ => {}
        }
        let content = w.add(Stream::flate(Dict::new(), &c.finish().into_vec()));
        let mut resources = dict([
            ("Font", Object::Dict(dict([("F1", font_ref.into())]))),
            ("ExtGState", Object::Dict(dict([("GS1", gs_half.into())]))),
        ]);
        if !xobjects.is_empty() {
            resources.insert("XObject", xobjects);
        }
        let mut page = dict([
            ("Type", Object::name("Page")),
            ("Parent", pages_ref.into()),
            ("MediaBox", reals(&[0.0, 0.0, PAGE_W, PAGE_H])),
            ("Resources", Object::Dict(resources)),
            ("Contents", content.into()),
        ]);
        if index == 0 {
            let y = PAGE_H - (100.0 + 3.0 * 18.0);
            let annot = w.add(dict([
                ("Type", Object::name("Annot")),
                ("Subtype", Object::name("Link")),
                ("Rect", reals(&[56.0, y - 4.0, 386.0, y + 13.0])),
                ("Border", reals(&[0.0, 0.0, 0.0])),
                ("Contents", Object::String(PdfString::literal("Visit example.com"))),
                (
                    "A",
                    Object::Dict(dict([
                        ("S", Object::name("URI")),
                        ("URI", Object::String(PdfString::literal(LINK_URI))),
                    ])),
                ),
            ]));
            page.insert("Annots", vec![Object::from(annot)]);
        }
        page_refs.push(w.add(page));
    }

    // Outline: one item per page, /Dest [page /XYZ 0 top null].
    let items: Vec<ObjRef> = (0..page_refs.len()).map(|_| w.reserve()).collect();
    for (i, (&item, &page)) in items.iter().zip(&page_refs).enumerate() {
        let mut d = dict([
            ("Title", Object::String(PdfString::literal(PAGES[i].0))),
            ("Parent", outline_ref.into()),
            (
                "Dest",
                Object::Array(vec![
                    page.into(),
                    Object::name("XYZ"),
                    0.into(),
                    Object::Real(f64::from(PAGE_H)),
                    Object::Null,
                ]),
            ),
        ]);
        if i > 0 {
            d.insert("Prev", items[i - 1]);
        }
        if let Some(&next) = items.get(i + 1) {
            d.insert("Next", next);
        }
        w.set(item, d);
    }
    w.set(
        outline_ref,
        dict([
            ("Type", Object::name("Outlines")),
            ("First", items[0].into()),
            ("Last", items[items.len() - 1].into()),
            ("Count", (items.len() as i64).into()),
        ]),
    );
    w.set(
        pages_ref,
        dict([
            ("Type", Object::name("Pages")),
            ("Kids", Object::Array(page_refs.iter().map(|&r| r.into()).collect())),
            ("Count", (page_refs.len() as i64).into()),
        ]),
    );
    let font_subset_len = font.finish(&mut w)?;
    let catalog = w.add(dict([
        ("Type", Object::name("Catalog")),
        ("Pages", pages_ref.into()),
        ("Outlines", outline_ref.into()),
        ("Lang", Object::String(PdfString::literal("en"))),
    ]));
    let info = w.add(dict([
        ("Title", Object::String(PdfString::literal(TITLE))),
        (
            "Producer",
            Object::String(PdfString::literal("lamina krilla-fit spike")),
        ),
        ("CreationDate", Object::String(PdfString::literal("D:20261028000000Z"))),
    ]));
    let trailer = dict([("Root", catalog.into()), ("Info", info.into())]);
    let bytes = if compact {
        w.finish_compact(&trailer)?
    } else {
        w.finish(&trailer)?
    };
    Ok(Built { bytes, font_subset_len })
}

fn draw_vectors(c: &mut Content) {
    let y = |top: f32| PAGE_H - top;
    c.save_state().set_parameters(pdf_writer::Name(b"GS1"));
    c.set_fill_rgb(30.0 / 255.0, 120.0 / 255.0, 200.0 / 255.0)
        .set_stroke_rgb(0.08, 0.08, 0.08)
        .set_line_width(2.0);
    c.rect(56.0, y(340.0), 200.0, 120.0)
        .fill_nonzero_and_stroke()
        .restore_state();
    c.set_fill_rgb(220.0 / 255.0, 80.0 / 255.0, 40.0 / 255.0);
    c.move_to(300.0, y(340.0))
        .line_to(400.0, y(220.0))
        .line_to(500.0, y(340.0))
        .close_path();
    let (cx, cy, r, k) = (420.0, 450.0, 60.0, 0.5523 * 60.0);
    c.move_to(cx + r, y(cy));
    c.cubic_to(cx + r, y(cy + k), cx + k, y(cy + r), cx, y(cy + r));
    c.cubic_to(cx - k, y(cy + r), cx - r, y(cy + k), cx - r, y(cy));
    c.cubic_to(cx - r, y(cy - k), cx - k, y(cy - r), cx, y(cy - r));
    c.cubic_to(cx + k, y(cy - r), cx + r, y(cy - k), cx + r, y(cy));
    c.close_path().fill_nonzero();
}

/// Decodes an 8-bit RGBA PNG into (RGB, alpha) planes.
fn decode_png(data: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
    let mut reader = png::Decoder::new(std::io::Cursor::new(data)).read_info()?;
    let mut buf = vec![0; reader.output_buffer_size().ok_or_else(|| anyhow!("png size"))?];
    let info = reader.next_frame(&mut buf)?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return Err(anyhow!("spike only handles 8-bit RGBA PNGs"));
    }
    let px = buf.get(..info.buffer_size()).ok_or_else(|| anyhow!("png buffer"))?;
    let mut rgb = Vec::with_capacity(px.len() / 4 * 3);
    let mut alpha = Vec::with_capacity(px.len() / 4);
    for &[r, g, b, a] in px.as_chunks::<4>().0 {
        rgb.extend_from_slice(&[r, g, b]);
        alpha.push(a);
    }
    Ok((rgb, alpha))
}
