//! The append API: adds content to a page without reading what its existing content
//! does. [`PageContentWriter`] wraps a page's content in `q … Q`, adds streams before or after
//! it and registers resources under unused names, all through a [`CosMut`], so the caller's
//! transaction undoes it through the journal (ADR 0006).
//!
//! The only change it makes to existing content is turning each `Q` that [`scan_balance`]
//! finds at depth 0 into `q Q` (a renderer ignores such a `Q`, but inside a wrap it would
//! restore the wrap's own state). Every other stream is left as it is, and a stream it changes is
//! written as a new object, so a stream other pages share stays theirs.

mod scan;

pub use scan::{Balance, StreamOffset, scan_balance};

use std::collections::BTreeSet;

use laminapdf_cos::{CosMut, Error, Limit, ObjRef, Page, Result};
use laminapdf_syntax::{Dict, Limits, Name, Object, Stream};

use crate::lexer::{ContentIter, Lexed};

/// The resource categories whose names a new name must not repeat (§7.8.3, Table 34).
const CATEGORIES: [&str; 7] = [
    "ExtGState",
    "ColorSpace",
    "Pattern",
    "Shading",
    "XObject",
    "Font",
    "Properties",
];

/// What [`PageContentWriter::wrap_existing`] did.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Wrapped {
    /// The content was already wrapped ([`Balance::wrapped`]): nothing was written.
    Already,
    /// The content is now wrapped.
    Now {
        /// The `Q`s the closing stream holds: one for the wrap and one per `q` the content left
        /// open ([`Balance::depth`]).
        closing: usize,
        /// The unmatched `Q` tokens that were neutralised (each now `q Q`), by their place in
        /// the old `/Contents`.
        neutralised: Vec<StreamOffset>,
        /// Whether the closing stream ends a text object the content left open (`ET`).
        ended_text: bool,
        /// The marked-content sequences the content left open, which the closing stream ends
        /// (`EMC`) so that nothing added after the wrap is marked as part of them.
        ended_marked: usize,
    },
}

/// Adds content to one page through a [`CosMut`].
///
/// Each call writes the page at once; the writer keeps the page's `/Contents` and
/// `/Resources` as it wrote them. A page's first write gives it its own `/Resources` (a copy of
/// what it had or inherited, a resource category it adds to copied too), so a dictionary other
/// pages share never changes.
///
/// Call [`wrap_existing`](Self::wrap_existing) before [`append`](Self::append) when the new
/// content must not see the state the page's content leaves behind:
///
/// ```
/// use laminapdf_content::PageContentWriter;
/// use laminapdf_cos::{Dict, Document, Object, Page, Result};
///
/// /// Draws a half-transparent square in the page's bottom-left corner, over its content.
/// fn stamp(doc: &mut Document, page: &Page) -> Result<()> {
///     let mut writer = PageContentWriter::new(doc.edit(), page)?;
///     writer.wrap_existing()?;
///     let mut gs = Dict::new();
///     gs.insert("ca", 0.5);
///     let name = writer.add_extgstate(Object::Dict(gs))?;
///     let name = String::from_utf8_lossy(name.as_bytes()).into_owned();
///     writer.append(format!("q /{name} gs 0 0 72 72 re f Q").as_bytes())?;
///     Ok(())
/// }
/// ```
///
/// Added in 0.2.0.
#[derive(Debug)]
pub struct PageContentWriter<'d> {
    cos: CosMut<'d>,
    page: ObjRef,
    dict: Dict,
    contents: Vec<Object>,
    resources: Dict,
    limits: Limits,
    /// Every name the page's content uses as an operand, read at the first resource added: a
    /// new name must not be one the content already draws with, even when its resources lack it.
    content_names: Option<BTreeSet<Name>>,
}

impl<'d> PageContentWriter<'d> {
    /// A writer for `page`, read from the document `cos` edits.
    ///
    /// # Errors
    /// [`Error::InvalidStructure`] for a page that is a direct dictionary (no object to
    /// rewrite) or whose object is gone; [`Error::Type`] when the page or its `/Contents` is
    /// malformed; errors reading them.
    ///
    /// Added in 0.2.0.
    pub fn new(cos: CosMut<'d>, page: &Page) -> Result<Self> {
        let r = page.obj_ref.ok_or(Error::InvalidStructure {
            kind: "page",
            message: "a page that is a direct dictionary can't be edited",
        })?;
        let dict = match cos.get(r)?.as_deref() {
            Some(Object::Dict(d)) => d.clone(),
            Some(_) => {
                return Err(Error::Type {
                    expected: "dictionary",
                    context: "page",
                });
            }
            None => {
                return Err(Error::InvalidStructure {
                    kind: "page",
                    message: "the page object no longer exists",
                });
            }
        };
        let doc = cos.document();
        let contents = match dict.get("Contents") {
            None => Vec::new(),
            Some(value) => match &*doc.deref(value)? {
                Object::Array(items) => items.clone(),
                Object::Null => Vec::new(),
                _ => vec![value.clone()],
            },
        };
        // The page's own `/Resources` as it is now (an earlier writer may have written it), else
        // what it inherits.
        let resources = match dict.get("Resources") {
            Some(own) => match &*doc.deref(own)? {
                Object::Dict(d) => d.clone(),
                _ => page.resources_dict().cloned().unwrap_or_default(),
            },
            None => page.resources_dict().cloned().unwrap_or_default(),
        };
        let limits = *doc.limits();
        Ok(Self {
            cos,
            page: r,
            dict,
            contents,
            resources,
            limits,
            content_names: None,
        })
    }

    /// The editor the writer writes through, to create the objects it registers (a Form
    /// XObject's stream, a font).
    ///
    /// Added in 0.2.0.
    pub fn cos(&mut self) -> &mut CosMut<'d> {
        &mut self.cos
    }

    /// Wraps the page's content in `q … Q`: a stream holding `q` before it and one holding
    /// `Q` once plus once per `q` it leaves open after it (with `ET` first when it ends in a
    /// text object). Unmatched `Q`s are turned into `q Q` first, each in a new copy of its stream; no
    /// other stream changes. A page already wrapped is left alone ([`Wrapped::Already`]).
    ///
    /// # Errors
    /// [`Error::InvalidStructure`] when the content ends inside a string, array, dictionary or
    /// inline image, so a closing stream could not be read as `Q`s; [`Error::Type`] for a
    /// `/Contents` entry that isn't a stream; errors decoding the streams or writing.
    ///
    /// Added in 0.2.0.
    pub fn wrap_existing(&mut self) -> Result<Wrapped> {
        let mut streams = self.decoded_contents()?;
        let balance = {
            let views: Vec<&[u8]> = streams.iter().map(Vec::as_slice).collect();
            scan_balance(&views, &self.limits)?
        };
        if balance.wrapped {
            return Ok(Wrapped::Already);
        }
        // A neutralised `Q` (`q Q`) opens one level where it stands.
        let deepest = balance.max_depth.max(usize::from(!balance.unmatched.is_empty()));
        if deepest >= self.limits.max_nesting_depth() {
            return Err(Error::LimitExceeded {
                limit: Limit::NestingDepth,
                at: None,
            });
        }
        let mut changed: Vec<usize> = balance.unmatched.iter().map(|at| at.stream).collect();
        changed.dedup();
        for &i in &changed {
            if let Some(stream) = streams.get_mut(i) {
                let offsets: Vec<usize> = balance
                    .unmatched
                    .iter()
                    .filter(|at| at.stream == i)
                    .map(|at| at.offset)
                    .collect();
                *stream = scan::neutralise(stream, &offsets);
            }
        }
        if !changed.is_empty() {
            // The neutralised content must read as the old one did, less the unmatched `Q`s.
            let views: Vec<&[u8]> = streams.iter().map(Vec::as_slice).collect();
            let after = scan_balance(&views, &self.limits)?;
            let same = after.unmatched.is_empty()
                && (after.depth, after.in_text, after.marked) == (balance.depth, balance.in_text, balance.marked);
            if !same {
                return Err(Error::InvalidStructure {
                    kind: "content stream",
                    message: "the page's content can't be read the same way once its unmatched Q operators are neutralised",
                });
            }
        }
        let closing = balance.depth + 1;
        let mut close = Vec::with_capacity(4 + 4 * balance.marked + 2 * closing);
        close.push(b'\n');
        for _ in 0..balance.marked {
            close.extend_from_slice(b"EMC\n");
        }
        if balance.in_text {
            close.extend_from_slice(b"ET\n");
        }
        for _ in 0..closing {
            close.extend_from_slice(b"Q\n");
        }
        let open = b"q\n".to_vec();

        // Read as a renderer reads it, the wrapped content must end with the closing stream's
        // own operators: content that ends inside a string, array, dictionary or inline image
        // would swallow them.
        let mut check: Vec<&[u8]> = vec![&open];
        check.extend(streams.iter().map(Vec::as_slice));
        check.push(&close);
        if !scan_balance(&check, &self.limits)?.wrapped {
            return Err(Error::InvalidStructure {
                kind: "content stream",
                message: "the page's content ends inside a string, array, dictionary or inline image, so nothing can follow it",
            });
        }

        // Checked before anything is written: only a stream object can be replaced by a copy, and
        // skipping one would leave its `Q` to end the wrap early.
        if changed
            .iter()
            .any(|&i| !matches!(self.contents.get(i), Some(Object::Ref(_))))
        {
            return Err(Error::InvalidStructure {
                kind: "content stream",
                message: "a /Contents entry holding an unmatched Q is not a stream object",
            });
        }
        for i in changed {
            let (Some(Object::Ref(old)), Some(bytes)) = (self.contents.get(i), streams.get(i)) else {
                continue;
            };
            let copy = self.copy_stream(*old, bytes)?;
            if let Some(slot) = self.contents.get_mut(i) {
                *slot = Object::Ref(copy);
            }
        }
        let open = self.cos.create(Object::Stream(Stream::new(Dict::new(), open)))?;
        let close_ref = self.cos.create(Object::Stream(Stream::new(Dict::new(), close)))?;
        self.contents.insert(0, Object::Ref(open));
        self.contents.push(Object::Ref(close_ref));
        self.write_page()?;
        Ok(Wrapped::Now {
            closing,
            neutralised: balance.unmatched,
            ended_text: balance.in_text,
            ended_marked: balance.marked,
        })
    }

    /// Adds `content` as a new Flate-compressed stream before the page's content: right after a
    /// wrap's `q` on a wrapped page (so the page stays wrapped and the content stays inside
    /// it), else before every other stream.
    ///
    /// # Errors
    /// [`Error::InvalidStructure`] when `content` has an unmatched `Q`, leaves a `q`, a text
    /// object or a marked-content sequence open, or ends inside a string, array, dictionary,
    /// inline image or comment; errors decoding the page's streams or writing.
    ///
    /// Added in 0.2.0.
    pub fn prepend(&mut self, content: &[u8]) -> Result<ObjRef> {
        let r = self.new_stream(content)?;
        let at = usize::from(self.is_wrapped()?);
        self.contents.insert(at, Object::Ref(r));
        self.write_page()?;
        Ok(r)
    }

    /// Adds `content`, such as `q 1 0 0 1 72 72 cm /Fm1 Do Q`, as a new Flate-compressed
    /// stream after every other stream of the page.
    ///
    /// # Errors
    /// As [`prepend`](Self::prepend).
    ///
    /// Added in 0.2.0.
    pub fn append(&mut self, content: &[u8]) -> Result<ObjRef> {
        let r = self.new_stream(content)?;
        self.contents.push(Object::Ref(r));
        self.write_page()?;
        Ok(r)
    }

    /// Registers the Form XObject `form` under `/XObject` with an unused name (`Fm1`, `Fm2`, …),
    /// or returns the name it already has there.
    ///
    /// # Errors
    /// Errors resolving the resources or writing.
    ///
    /// Added in 0.2.0.
    pub fn add_xobject(&mut self, form: ObjRef) -> Result<Name> {
        self.add_resource("XObject", "Fm", Object::Ref(form))
    }

    /// Registers the graphics state parameter dictionary `gs` (direct or a reference) under
    /// `/ExtGState` with an unused name (`Gs1`, …), or returns the name it already has there.
    ///
    /// # Errors
    /// As [`add_xobject`](Self::add_xobject).
    ///
    /// Added in 0.2.0.
    pub fn add_extgstate(&mut self, gs: Object) -> Result<Name> {
        self.add_resource("ExtGState", "Gs", gs)
    }

    /// Registers the font `font` under `/Font` with an unused name (`Ft1`, …), or returns the
    /// name it already has there.
    ///
    /// # Errors
    /// As [`add_xobject`](Self::add_xobject).
    ///
    /// Added in 0.2.0.
    pub fn add_font(&mut self, font: ObjRef) -> Result<Name> {
        self.add_resource("Font", "Ft", Object::Ref(font))
    }

    fn add_resource(&mut self, category: &'static str, prefix: &str, value: Object) -> Result<Name> {
        let doc = self.cos.document();
        let mut entries = match self.resources.get(category) {
            Some(sub) => match &*doc.deref(sub)? {
                Object::Dict(d) => d.clone(),
                _ => Dict::new(),
            },
            None => Dict::new(),
        };
        if let Some((name, _)) = entries.iter().find(|(_, v)| **v == value) {
            return Ok(name.clone());
        }
        if self.content_names.is_none() {
            self.content_names = Some(self.content_names()?);
        }
        let doc = self.cos.document();
        let mut used: Vec<Name> = self.content_names.iter().flatten().cloned().collect();
        for key in CATEGORIES {
            if let Some(sub) = self.resources.get(key)
                && let Object::Dict(d) = &*doc.deref(sub)?
            {
                used.extend(d.iter().map(|(name, _)| name.clone()));
            }
        }
        // One of the first `used.len() + 1` numbers is free.
        let name = (1..=used.len() + 1)
            .map(|n| Name::from(format!("{prefix}{n}").as_str()))
            .find(|name| !used.contains(name))
            .unwrap_or_else(|| Name::from(prefix));
        entries.insert(name.clone(), value);
        if let Some(names) = &mut self.content_names {
            names.insert(name.clone());
        }
        self.resources.insert(category, Object::Dict(entries));
        self.dict.insert("Resources", Object::Dict(self.resources.clone()));
        self.write_page()?;
        Ok(name)
    }

    /// Every `/Contents` entry decoded; a null or missing stream is empty. The streams share one
    /// [`Limits::max_decoded_total`] budget, as in `Document::page_content`.
    fn decoded_contents(&self) -> Result<Vec<Vec<u8>>> {
        let doc = self.cos.document();
        let mut total = 0usize;
        let mut out = Vec::with_capacity(self.contents.len());
        for element in &self.contents {
            let bytes = match &*doc.deref(element)? {
                Object::Stream(s) => doc.decode_stream(s)?,
                Object::Null => Vec::new(),
                _ => {
                    return Err(Error::Type {
                        expected: "stream",
                        context: "/Contents",
                    });
                }
            };
            total = total.saturating_add(bytes.len());
            if total > self.limits.max_decoded_total() {
                return Err(Error::LimitExceeded {
                    limit: Limit::DecodedTotal,
                    at: None,
                });
            }
            out.push(bytes);
        }
        Ok(out)
    }

    /// A new stream object with `old`'s dictionary and `decoded` as its content: compressed
    /// again with Flate unless `old` was stored unfiltered.
    fn copy_stream(&mut self, old: ObjRef, decoded: &[u8]) -> Result<ObjRef> {
        let mut dict = match self.cos.get(old)?.as_deref() {
            Some(Object::Stream(s)) => s.dict.clone(),
            _ => Dict::new(),
        };
        let filtered = dict.contains_key("Filter");
        for key in ["Length", "Filter", "DecodeParms", "DL"] {
            dict.remove(key);
        }
        let stream = if filtered {
            Stream::flate(dict, decoded)
        } else {
            Stream::new(dict, decoded.to_vec())
        };
        self.cos.create(Object::Stream(stream))
    }

    fn new_stream(&mut self, content: &[u8]) -> Result<ObjRef> {
        // A sentinel `q` after the content must be read as its own operator: content that ends
        // inside a string, array, dictionary or inline image would swallow it.
        let (balance, last) = scan::scan(&[content, b"q"], &self.limits)?;
        let closed = balance.unmatched.is_empty() && balance.depth == 1 && !balance.in_text && balance.marked == 0;
        if !closed || last != Some(1) {
            return Err(Error::InvalidStructure {
                kind: "content stream",
                message: "added content must end at a token boundary, close every q, text object and marked-content sequence it opens and restore no state it didn't save",
            });
        }
        // A newline ends a trailing comment, so the next stream's first token is its own.
        let mut bytes = content.to_vec();
        bytes.push(b'\n');
        self.cos.create(Object::Stream(Stream::flate(Dict::new(), &bytes)))
    }

    /// Whether the page's content is wrapped now ([`Balance::wrapped`]).
    fn is_wrapped(&self) -> Result<bool> {
        let streams = self.decoded_contents()?;
        let views: Vec<&[u8]> = streams.iter().map(Vec::as_slice).collect();
        Ok(scan_balance(&views, &self.limits)?.wrapped)
    }

    /// Every name the content uses as an operand (including inside arrays and dictionaries,
    /// such as a `BDC` property list).
    fn content_names(&self) -> Result<BTreeSet<Name>> {
        fn collect(object: &Object, names: &mut BTreeSet<Name>) {
            match object {
                Object::Name(name) => {
                    names.insert(name.clone());
                }
                Object::Array(items) => items.iter().for_each(|item| collect(item, names)),
                Object::Dict(dict) => dict.iter().for_each(|(_, value)| collect(value, names)),
                _ => {}
            }
        }
        let mut names = BTreeSet::new();
        for stream in self.decoded_contents()? {
            for item in ContentIter::new(&stream).with_limits(&self.limits) {
                if let Lexed::Op(op) = item? {
                    op.operands.iter().for_each(|operand| collect(operand, &mut names));
                }
            }
        }
        Ok(names)
    }

    fn write_page(&mut self) -> Result<()> {
        self.dict.insert("Contents", Object::Array(self.contents.clone()));
        self.cos.set(self.page, Object::Dict(self.dict.clone()))
    }
}
