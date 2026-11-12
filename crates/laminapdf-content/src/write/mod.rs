//! The append API: adds content to a page without reading what its existing content
//! does. [`PageContentWriter`] wraps a page's content in `q … Q`, adds streams before or after
//! it and registers resources under unused names, all through a [`CosMut`], so the caller's
//! transaction undoes it through the journal (ADR 0006).
//!
//! The only change it makes to existing content is blanking the `Q` tokens that
//! [`scan_balance`] finds at depth 0 (a renderer ignores them, but inside a wrap they would
//! restore the wrap's own state). Every other stream is left as it is, and a stream it changes is
//! written as a new object, so a stream other pages share stays theirs.

mod scan;

pub use scan::{Balance, StreamOffset, scan_balance};

use laminapdf_cos::{CosMut, Error, ObjRef, Page, Result};
use laminapdf_syntax::{Dict, Limits, Name, Object, Stream};

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
        /// The unmatched `Q` tokens that were blanked, by their place in the old `/Contents`.
        blanked: Vec<StreamOffset>,
        /// Whether the closing stream starts with `ET` because the content ended in a text
        /// object.
        ended_text: bool,
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
        let resources = page.resources_dict().cloned().unwrap_or_default();
        let limits = *doc.limits();
        Ok(Self {
            cos,
            page: r,
            dict,
            contents,
            resources,
            limits,
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
    /// text object). Unmatched `Q`s are blanked first, each in a new copy of its stream; no
    /// other stream changes. A page already wrapped is left alone ([`Wrapped::Already`]).
    ///
    /// # Errors
    /// [`Error::InvalidStructure`] when the content ends inside a string, array, dictionary or
    /// inline image, so a closing stream could not be read as `Q`s; [`Error::Type`] for a
    /// `/Contents` entry that isn't a stream; errors decoding the streams or writing.
    ///
    /// Added in 0.2.0.
    pub fn wrap_existing(&mut self) -> Result<Wrapped> {
        let decoded = self.decoded_contents()?;
        let views: Vec<&[u8]> = decoded.iter().map(Vec::as_slice).collect();
        let balance = scan_balance(&views, &self.limits)?;
        if balance.wrapped {
            return Ok(Wrapped::Already);
        }
        let mut streams = decoded.clone();
        for (i, stream) in streams.iter_mut().enumerate() {
            let offsets = balance.unmatched.iter().filter(|at| at.stream == i).map(|at| at.offset);
            *stream = scan::blank(stream, offsets);
        }
        let closing = balance.depth + 1;
        let mut close = Vec::with_capacity(4 + 2 * closing);
        close.push(b'\n');
        if balance.in_text {
            close.extend_from_slice(b"ET\n");
        }
        for _ in 0..closing {
            close.extend_from_slice(b"Q\n");
        }
        let open = b"q\n".to_vec();

        let mut check: Vec<&[u8]> = vec![&open];
        check.extend(streams.iter().map(Vec::as_slice));
        check.push(&close);
        if !scan_balance(&check, &self.limits)?.wrapped {
            return Err(Error::InvalidStructure {
                kind: "content stream",
                message: "the page's content ends inside a string, array, dictionary or inline image, so nothing can follow it",
            });
        }

        let mut changed: Vec<usize> = balance.unmatched.iter().map(|at| at.stream).collect();
        changed.dedup();
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
            blanked: balance.unmatched,
            ended_text: balance.in_text,
        })
    }

    /// Adds `content` as a new Flate-compressed stream before every other stream of the page.
    ///
    /// # Errors
    /// [`Error::InvalidStructure`] when `content` has an unmatched `Q`, leaves a `q` open or
    /// ends inside a text object; errors writing.
    ///
    /// Added in 0.2.0.
    pub fn prepend(&mut self, content: &[u8]) -> Result<ObjRef> {
        let r = self.new_stream(content)?;
        self.contents.insert(0, Object::Ref(r));
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
        let mut used = Vec::new();
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
        self.resources.insert(category, Object::Dict(entries));
        self.dict.insert("Resources", Object::Dict(self.resources.clone()));
        self.write_page()?;
        Ok(name)
    }

    /// Every `/Contents` entry decoded; a null or missing stream is empty.
    fn decoded_contents(&self) -> Result<Vec<Vec<u8>>> {
        let doc = self.cos.document();
        self.contents
            .iter()
            .map(|element| match &*doc.deref(element)? {
                Object::Stream(s) => doc.decode_stream(s),
                Object::Null => Ok(Vec::new()),
                _ => Err(Error::Type {
                    expected: "stream",
                    context: "/Contents",
                }),
            })
            .collect()
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
        let balance = scan_balance(&[content], &self.limits)?;
        if !balance.unmatched.is_empty() || balance.depth > 0 || balance.in_text {
            return Err(Error::InvalidStructure {
                kind: "content stream",
                message: "added content must close every q and text object it opens and restore no state it didn't save",
            });
        }
        self.cos.create(Object::Stream(Stream::flate(Dict::new(), content)))
    }

    fn write_page(&mut self) -> Result<()> {
        self.dict.insert("Contents", Object::Array(self.contents.clone()));
        self.cos.set(self.page, Object::Dict(self.dict.clone()))
    }
}
