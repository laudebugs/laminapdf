//! How an added object is drawn and found again (E6 plan §2.1): its own Form XObject, drawn by a
//! stream of its own in the page's `/Contents` with exactly this grammar,
//!
//! ```text
//! q a b c d e f cm [/GSn gs] [/Tag <<…>> BDC] /FmN Do [EMC] Q
//! ```
//!
//! and named by the form's page-piece dictionary (ISO 32000-2 §14.5):
//! `/PieceInfo << /LaminaPDF << /LastModified … /Private << /Id (…) /Kind /Image … >> >> >>`.
//! The scanner reads only streams that match the grammar and draw a form with that dictionary,
//! so a stream or form another app changed is left alone as the page's own content, and nothing
//! reads the page's own operators.

use std::fmt::Write as _;

use laminapdf_content::{ContentIter, Lexed, PageContentWriter};
use laminapdf_cos::{Dict, Name, ObjRef, Object, PdfString};
use laminapdf_model::{Matrix, PdfDate};
use laminapdf_syntax::format_real;
use laminapdf_syntax::text::{decode_text_string, encode_text_string};

use super::mark_types::PageMark;
use super::spec::{Alt, ContentId, ContentLayer, ContentObjectSpec};
use crate::annot::object_dict;
use crate::error::Result;

/// The key under `/PieceInfo` (§14.5): the name of the app that owns the data.
pub(crate) const APP: &str = "LaminaPDF";

/// The longest drawing stream the scanner reads: the grammar's stream, with room to spare. A
/// longer stream is the page's own content.
const MAX_DRAWING: usize = 512;

/// How an added object's drawing is marked in a tagged file (§14.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mark {
    /// Untagged: the file has no structure tree.
    None,
    /// `/Artifact << /Type /Layout >> BDC … EMC`: decoration (§14.8.2.2).
    Artifact,
    /// `/Figure << /MCID n >> BDC … EMC`: a `Figure` element owns it.
    Figure(i64),
    /// `/P << /MCID n >> BDC … EMC`: a `P` element owns it (added text).
    Paragraph(i64),
    /// `/Artifact << /Type /Pagination /Subtype … >> BDC … EMC`: a page mark, which a screen
    /// reader skips and says what it is (§14.8.2.2.2).
    Page(PageArtifact),
}

/// What a page mark's artifact says it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PageArtifact {
    Watermark,
    Background,
    Header,
    Footer,
    Bates,
    /// A mark that spans the header and the footer, or any other running mark.
    Pagination,
}

impl PageArtifact {
    /// The property list of the marked content.
    const fn props(self) -> &'static str {
        match self {
            Self::Watermark => "<</Type /Pagination /Subtype /Watermark>>",
            Self::Background => "<</Type /Background>>",
            Self::Header => "<</Type /Pagination /Subtype /Header>>",
            Self::Footer => "<</Type /Pagination /Subtype /Footer>>",
            Self::Bates => "<</Type /Pagination /Subtype /Bates>>",
            Self::Pagination => "<</Type /Pagination>>",
        }
    }

    /// The page mark an `/Artifact` property list names; `None` for any other artifact.
    fn of(props: &Dict) -> Option<Self> {
        match props.get_name("Type")? {
            b"Background" => Some(Self::Background),
            b"Pagination" => Some(match props.get_name("Subtype") {
                Some(b"Watermark") => Self::Watermark,
                Some(b"Header") => Self::Header,
                Some(b"Footer") => Self::Footer,
                Some(b"Bates") => Self::Bates,
                _ => Self::Pagination,
            }),
            _ => None,
        }
    }
}

/// One drawing stream as the grammar has it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Drawing {
    /// The form's space to the page's user space.
    pub(crate) cm: Matrix,
    /// The ExtGState that sets the opacity.
    pub(crate) gs: Option<Name>,
    pub(crate) mark: Mark,
    /// The form's name in the page's `/XObject` resources.
    pub(crate) form: Name,
}

impl Drawing {
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let name = |n: &Name| String::from_utf8_lossy(n.as_bytes()).into_owned();
        let mut s = String::from("q ");
        let cm = self.cm.to_array().map(format_real).join(" ");
        let _ = write!(s, "{cm} cm");
        if let Some(gs) = &self.gs {
            let _ = write!(s, " /{} gs", name(gs));
        }
        match self.mark {
            Mark::None => {}
            Mark::Artifact => s.push_str(" /Artifact <</Type /Layout>> BDC"),
            Mark::Figure(mcid) => {
                let _ = write!(s, " /Figure <</MCID {mcid}>> BDC");
            }
            Mark::Paragraph(mcid) => {
                let _ = write!(s, " /P <</MCID {mcid}>> BDC");
            }
            Mark::Page(kind) => {
                let _ = write!(s, " /Artifact {} BDC", kind.props());
            }
        }
        let _ = write!(s, " /{} Do", name(&self.form));
        if self.mark != Mark::None {
            s.push_str(" EMC");
        }
        s.push_str(" Q");
        s.into_bytes()
    }

    /// The drawing `bytes` hold, when they hold exactly the grammar.
    pub(crate) fn parse(bytes: &[u8]) -> Option<Self> {
        let mut ops = Vec::new();
        for item in ContentIter::new(bytes) {
            match item.ok()? {
                Lexed::Op(op) => ops.push((op.operator.keyword().to_vec(), op.operands)),
                Lexed::Issue(_) => return None,
            }
        }
        let mut ops = ops.into_iter().peekable();
        let mut next = |keyword: &[u8]| ops.next_if(|(k, _)| k == keyword).map(|(_, operands)| operands);
        let q = next(b"q")?;
        let cm = next(b"cm")?;
        let gs = next(b"gs");
        let bdc = next(b"BDC");
        let draw = next(b"Do")?;
        let emc = next(b"EMC");
        let end = next(b"Q")?;
        if ops.next().is_some() || !q.is_empty() || !end.is_empty() || bdc.is_some() != emc.is_some() {
            return None;
        }
        let numbers: Vec<f64> = cm.iter().map(Object::as_f64).collect::<Option<_>>()?;
        let cm = Matrix::from_slice(&numbers).filter(|_| numbers.len() == 6)?;
        let gs = match gs.as_deref() {
            None => None,
            Some([Object::Name(n)]) => Some(n.clone()),
            Some(_) => return None,
        };
        let mark = match bdc.as_deref() {
            None => Mark::None,
            Some([Object::Name(tag), Object::Dict(props)]) => match tag.as_bytes() {
                b"Artifact" => PageArtifact::of(props).map_or(Mark::Artifact, Mark::Page),
                b"Figure" => Mark::Figure(props.get_int("MCID")?),
                b"P" => Mark::Paragraph(props.get_int("MCID")?),
                _ => return None,
            },
            Some(_) => return None,
        };
        if emc.is_some_and(|operands| !operands.is_empty()) {
            return None;
        }
        let [Object::Name(form)] = draw.as_slice() else {
            return None;
        };
        Some(Self {
            cm,
            gs,
            mark,
            form: form.clone(),
        })
    }
}

/// What an added object's form says of it in `/PieceInfo /LaminaPDF /Private`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Private {
    pub(crate) id: ContentId,
    /// `/Kind`: `Image`, `Shape` or `Form`, kept when the spec can't be read.
    pub(crate) kind: String,
    /// The object as the op that added it, or last changed it, gave it; `None` when the stored
    /// form can't be read as this version's spec.
    pub(crate) spec: Option<ContentObjectSpec>,
    pub(crate) alt: Option<Alt>,
    /// `/Layer`: whether the op put the object behind the page's content. Recorded rather than
    /// worked out from the streams' order, since another op may append content after an object
    /// in front.
    pub(crate) layer: ContentLayer,
    /// `/Mark`: what a page mark (a watermark, a header, a Bates number) was made from, which
    /// `UpdatePageMark` replaces; `None` for an object `AddContentObject` or `AddText` added.
    pub(crate) mark: Option<PageMark>,
}

impl Private {
    /// `/Private`'s dictionary.
    fn to_dict(&self) -> Dict {
        let mut d = Dict::new();
        d.insert("Id", PdfString::literal(self.id.as_str().as_bytes().to_vec()));
        d.insert("Kind", Object::name(&self.kind));
        // A spec always serialises (no map with non-string keys, no failing `Serialize`); were
        // it not to, the object would only lose what it is redrawn from, as a foreign one has.
        if let Some(spec) = &self.spec
            && let Ok(json) = serde_json::to_string(spec)
        {
            d.insert("Spec", PdfString::literal(json.into_bytes()));
        }
        if let Some(mark) = &self.mark
            && let Ok(json) = serde_json::to_string(mark)
        {
            d.insert("Mark", PdfString::literal(json.into_bytes()));
        }
        if self.layer == ContentLayer::Behind {
            d.insert("Layer", Object::name("Behind"));
        }
        match &self.alt {
            Some(Alt::Text(text)) => {
                d.insert("Alt", PdfString::literal(encode_text_string(text)));
            }
            Some(Alt::Decorative) => {
                d.insert("Decorative", Object::Bool(true));
            }
            None => {}
        }
        d
    }

    fn from_dict(d: &Dict) -> Option<Self> {
        let string = |key: &str| match d.get(key) {
            Some(Object::String(s)) => Some(s.bytes.as_slice()),
            _ => None,
        };
        let id = ContentId::new(String::from_utf8(string("Id")?.to_vec()).ok()?)?;
        let kind = String::from_utf8_lossy(d.get_name("Kind")?).into_owned();
        // Read as a string, so the spec is parsed by the same code the ops' JSON is.
        let spec = string("Spec")
            .and_then(|s| std::str::from_utf8(s).ok())
            .and_then(|s| serde_json::from_str(s).ok());
        let mark = string("Mark")
            .and_then(|s| std::str::from_utf8(s).ok())
            .and_then(|s| serde_json::from_str(s).ok());
        let layer = if d.get_name("Layer") == Some(b"Behind".as_slice()) {
            ContentLayer::Behind
        } else {
            ContentLayer::Front
        };
        let alt = match (string("Alt"), d.get("Decorative")) {
            (Some(text), _) => Some(Alt::Text(decode_text_string(text))),
            (None, Some(Object::Bool(true))) => Some(Alt::Decorative),
            _ => None,
        };
        Some(Self {
            id,
            kind,
            spec,
            alt,
            layer,
            mark,
        })
    }
}

/// Gives the form `form` its page-piece dictionary and `/LastModified` (both dated `modified`,
/// ISO 32000-2 §14.5 and Table 93), naming it by `private`.
pub(crate) fn mark_form(
    doc: &mut laminapdf_model::Document,
    form: ObjRef,
    private: &Private,
    modified: &PdfDate,
) -> Result<()> {
    let date = PdfString::literal(modified.to_string().into_bytes());
    let mut app = Dict::new();
    app.insert("LastModified", date.clone());
    app.insert("Private", private.to_dict());
    let mut piece = Dict::new();
    piece.insert(APP, app);
    let mut cos = doc.cos_mut().edit();
    let Some(current) = cos.get(form)? else {
        return Ok(());
    };
    if let Object::Stream(mut stream) = (*current).clone() {
        stream.dict.insert("PieceInfo", piece);
        stream.dict.insert("LastModified", date);
        cos.set(form, Object::Stream(stream))?;
    }
    Ok(())
}

/// An object Lamina PDF added that a page draws.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Placed {
    /// The drawing stream.
    pub(crate) stream: ObjRef,
    /// Its index in `/Contents`.
    pub(crate) index: usize,
    pub(crate) drawing: Drawing,
    /// The form it draws.
    pub(crate) form: ObjRef,
    /// The form's dictionary (its `/BBox`, `/Resources`, `/PieceInfo`).
    pub(crate) form_dict: Dict,
    pub(crate) private: Private,
    /// The opacity its ExtGState sets.
    pub(crate) opacity: f64,
}

/// The objects Lamina PDF added that `page` draws, in the order they paint.
pub(crate) fn scan(doc: &laminapdf_model::Document, page: &laminapdf_cos::Page) -> Result<Vec<Placed>> {
    let Some(r) = page.obj_ref else {
        return Ok(Vec::new());
    };
    let Some(dict) = object_dict(doc, r)? else {
        return Ok(Vec::new());
    };
    let contents = match dict.get("Contents") {
        Some(value) => match &*doc.deref(value)? {
            Object::Array(items) => items.clone(),
            _ => vec![value.clone()],
        },
        None => Vec::new(),
    };
    // The page's own resources as they are now (an op may have just written them), else what it
    // inherits.
    let resources = match dict.get("Resources") {
        Some(value) => doc.deref(value)?.as_dict().cloned(),
        None => page.resources_dict().cloned(),
    };
    let category = |key: &str| -> Result<Dict> {
        Ok(match resources.as_ref().and_then(|r| r.get(key)) {
            Some(x) => doc.deref(x)?.as_dict().cloned().unwrap_or_default(),
            None => Dict::new(),
        })
    };
    let (xobjects, states) = (category("XObject")?, category("ExtGState")?);
    // Each entry: the object it adds, or `None` for the page's own content.
    let mut found: Vec<Option<Placed>> = Vec::with_capacity(contents.len());
    for (index, entry) in contents.iter().enumerate() {
        found.push(match entry {
            Object::Ref(stream) => placed(doc, *stream, index, &xobjects, &states)?,
            _ => None,
        });
    }
    let out: Vec<Placed> = found.into_iter().flatten().collect();
    Ok(out)
}

/// The added object the stream `stream` draws, if it draws one.
fn placed(
    doc: &laminapdf_model::Document,
    stream: ObjRef,
    index: usize,
    xobjects: &Dict,
    states: &Dict,
) -> Result<Option<Placed>> {
    let Some(object) = laminapdf_cos::tolerant(doc.cos().resolve(stream))? else {
        return Ok(None);
    };
    let Object::Stream(s) = &*object else {
        return Ok(None);
    };
    if s.data.len() > MAX_DRAWING {
        return Ok(None);
    }
    let Ok(bytes) = doc.cos().decode_stream(s) else {
        return Ok(None);
    };
    let Some(drawing) = Drawing::parse(&bytes) else {
        return Ok(None);
    };
    let Some(Object::Ref(form)) = std::str::from_utf8(drawing.form.as_bytes())
        .ok()
        .and_then(|n| xobjects.get(n))
    else {
        return Ok(None);
    };
    let Some(form_object) = laminapdf_cos::tolerant(doc.cos().resolve(*form))? else {
        return Ok(None);
    };
    let Object::Stream(form_stream) = &*form_object else {
        return Ok(None);
    };
    let private = form_stream
        .dict
        .get("PieceInfo")
        .and_then(Object::as_dict)
        .and_then(|p| p.get(APP))
        .and_then(Object::as_dict)
        .and_then(|d| d.get("Private"))
        .and_then(Object::as_dict)
        .and_then(Private::from_dict);
    let Some(private) = private else {
        return Ok(None);
    };
    // `/ca` is the fill's opacity, which the drawing sets with `/CA` to one value (Table 57).
    let opacity = match drawing.gs.as_ref().and_then(|n| std::str::from_utf8(n.as_bytes()).ok()) {
        Some(name) => match states.get(name) {
            Some(gs) => doc
                .deref(gs)?
                .as_dict()
                .and_then(|d| d.get("ca").or_else(|| d.get("CA")))
                .and_then(Object::as_f64)
                .map_or(1.0, |v| v.clamp(0.0, 1.0)),
            None => 1.0,
        },
        None => 1.0,
    };
    Ok(Some(Placed {
        stream,
        index,
        drawing,
        form: *form,
        form_dict: form_stream.dict.clone(),
        private,
        opacity,
    }))
}

/// How [`place_form`] draws a form on a page.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Placement {
    /// The form's space to the page's user space.
    pub(crate) cm: Matrix,
    /// Below 1, an ExtGState sets it around the drawing.
    pub(crate) opacity: f64,
    pub(crate) mark: Mark,
    pub(crate) layer: ContentLayer,
}

/// Draws the form `form` on `page` (E6 plan §2.1) as `placement` says: wraps the page's content
/// once, registers the form and, for an opacity below 1, an ExtGState, and adds the drawing
/// stream in front of the content or behind it, over the objects in `behind`, those already added
/// behind it. Returns the stream.
///
/// The other content ops that put a form on a page (flattening a form, resizing a page) are
/// to call it too, so every form Lamina PDF puts on a page is drawn one way.
pub(crate) fn place_form(
    doc: &mut laminapdf_model::Document,
    page: &laminapdf_cos::Page,
    form: ObjRef,
    placement: Placement,
    behind: &[ObjRef],
) -> Result<ObjRef> {
    let mut writer = PageContentWriter::new(doc.cos_mut().edit(), page)?;
    writer.wrap_existing()?;
    let gs = fade(&mut writer, placement.opacity)?;
    let drawing = drawing(&mut writer, form, placement.cm, gs, placement.mark)?;
    let stream = match placement.layer {
        ContentLayer::Front => writer.append(&drawing.to_bytes())?,
        ContentLayer::Behind => {
            let at = behind_slot(writer.contents(), behind);
            writer.insert(at, &drawing.to_bytes())?
        }
    };
    Ok(stream)
}

/// The drawing of `form`, its names registered with `writer`.
pub(crate) fn drawing(
    writer: &mut PageContentWriter<'_>,
    form: ObjRef,
    cm: Matrix,
    gs: Option<Name>,
    mark: Mark,
) -> Result<Drawing> {
    let name = writer.add_xobject(form)?;
    Ok(Drawing {
        cm,
        gs,
        mark,
        form: name,
    })
}

/// The ExtGState that sets `opacity`, registered with `writer`; none for an opaque object.
pub(crate) fn fade(writer: &mut PageContentWriter<'_>, opacity: f64) -> Result<Option<Name>> {
    if opacity >= 1.0 {
        return Ok(None);
    }
    let mut gs = Dict::new();
    gs.insert("Type", Object::name("ExtGState"));
    gs.insert("CA", Object::number(opacity));
    gs.insert("ca", Object::number(opacity));
    // An object of its own, so the name is the drawing's alone and goes with it: a direct
    // dictionary could share the name of an equal one the page's own content uses.
    let gs = writer.cos().create(Object::Dict(gs))?;
    Ok(Some(writer.add_extgstate(Object::Ref(gs))?))
}

/// Where a new object behind the content goes: over the last object already behind it, else at
/// index 1, right after the opening `q` that [`PageContentWriter::wrap_existing`] puts first.
pub(crate) fn behind_slot(contents: &[Object], behind: &[ObjRef]) -> usize {
    contents
        .iter()
        .rposition(|o| o.as_ref().is_some_and(|r| behind.contains(&r)))
        .map_or(1, |i| i + 1)
        .min(contents.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_drawing_reads_back_as_it_was_written() {
        for mark in [
            Mark::None,
            Mark::Artifact,
            Mark::Figure(3),
            Mark::Paragraph(4),
            Mark::Page(PageArtifact::Watermark),
            Mark::Page(PageArtifact::Background),
            Mark::Page(PageArtifact::Header),
            Mark::Page(PageArtifact::Footer),
            Mark::Page(PageArtifact::Bates),
            Mark::Page(PageArtifact::Pagination),
        ] {
            for gs in [None, Some(Name::from("Gs1"))] {
                let drawing = Drawing {
                    cm: Matrix::new([144.0, 0.0, 0.0, -44.0, 72.0, 700.5]),
                    gs: gs.clone(),
                    mark,
                    form: Name::from("Fm2"),
                };
                assert_eq!(Drawing::parse(&drawing.to_bytes()), Some(drawing));
            }
        }
    }

    #[test]
    fn anything_besides_the_grammar_is_the_pages_own_content() {
        for bytes in [
            &b"q 1 0 0 1 0 0 cm /Fm1 Do Q 0 0 m"[..],
            b"q 1 0 0 1 0 0 cm 1 0 0 rg /Fm1 Do Q",
            b"q 1 0 0 1 0 0 cm /Span <<>> BDC /Fm1 Do EMC Q",
            b"q 1 0 0 1 0 cm /Fm1 Do Q",
            b"q 1 0 0 1 0 0 cm /Figure <</MCID 1>> BDC /Fm1 Do Q",
            b"q /Fm1 Do Q",
            b"BT ET",
        ] {
            assert_eq!(Drawing::parse(bytes), None, "{}", String::from_utf8_lossy(bytes));
        }
    }

    #[test]
    fn a_new_object_behind_goes_over_the_others_behind() {
        let r = |n| ObjRef::new(n, 0);
        let contents: Vec<Object> = [1, 2, 3, 4, 5].map(|n| Object::Ref(r(n))).into();
        assert_eq!(behind_slot(&contents, &[]), 1);
        assert_eq!(behind_slot(&contents, &[r(2), r(3)]), 3);
    }
}
