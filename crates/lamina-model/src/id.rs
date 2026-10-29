//! [`LaminaId`]s (ADR 0005 §5): opaque, typed identifiers for pages, outline items,
//! annotations and fields, and the document-level [`IdAllocator`].

use std::fmt::{self, Write as _};
use std::str::FromStr;

use lamina_cos::{DirectPath, Error, ObjRef, PathSegment};

/// What a [`LaminaId`] identifies. Its letter starts the string form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Kind {
    /// A page (`p`).
    Page,
    /// An outline item (`o`).
    OutlineItem,
    /// An annotation (`a`; views arrive with the editor rounds).
    Annot,
    /// An interactive form field (`f`; views arrive with the editor rounds).
    Field,
}

impl Kind {
    const fn letter(self) -> char {
        match self {
            Self::Page => 'p',
            Self::OutlineItem => 'o',
            Self::Annot => 'a',
            Self::Field => 'f',
        }
    }

    const fn from_letter(c: char) -> Option<Self> {
        match c {
            'p' => Some(Self::Page),
            'o' => Some(Self::OutlineItem),
            'a' => Some(Self::Annot),
            'f' => Some(Self::Field),
            _ => None,
        }
    }
}

/// Where a [`LaminaId`]'s path starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Anchor {
    /// An indirect object.
    Object(ObjRef),
    /// The trailer (only when `/Root` itself is direct).
    Trailer,
}

/// One step of a path ID.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Segment {
    Key(Box<str>),
    Index(usize),
}

/// An opaque, typed identifier for an item of the document (ADR 0005 §5).
///
/// An item that is an indirect object is identified by its kind plus that object's number
/// and generation; the string form is `p12.0`. A direct (inline) dictionary has no object
/// identity and gets a **path ID** from its nearest indirect ancestor, e.g. `p5.0/Kids/3`,
/// which is stable for the same bytes. The string form is documented as opaque: compare IDs,
/// store them, send them, but don't derive meaning from them.
///
/// ```
/// use lamina_model::{Kind, LaminaId};
/// use lamina_cos::ObjRef;
///
/// let id = LaminaId::object(Kind::Page, ObjRef::new(12, 0));
/// assert_eq!(id.to_string(), "p12.0");
/// assert_eq!("p12.0".parse::<LaminaId>()?, id);
/// assert_eq!("p5.0/Kids/3".parse::<LaminaId>()?.to_string(), "p5.0/Kids/3");
/// # Ok::<(), lamina_model::ParseIdError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LaminaId {
    kind: Kind,
    anchor: Anchor,
    path: Box<[Segment]>,
}

impl LaminaId {
    /// The ID of the indirect object `r` as an item of `kind`.
    #[must_use]
    pub fn object(kind: Kind, r: ObjRef) -> Self {
        Self {
            kind,
            anchor: Anchor::Object(r),
            path: Box::new([]),
        }
    }

    /// The path ID of a direct dictionary at `path`.
    #[must_use]
    pub fn path(kind: Kind, path: &DirectPath) -> Self {
        let segments = path
            .segments
            .iter()
            .map(|s| match *s {
                PathSegment::Key(k) => Segment::Key(k.into()),
                PathSegment::Index(i) => Segment::Index(i),
            })
            .collect();
        Self {
            kind,
            anchor: path.anchor.map_or(Anchor::Trailer, Anchor::Object),
            path: segments,
        }
    }

    /// What this ID identifies.
    #[must_use]
    pub const fn kind(&self) -> Kind {
        self.kind
    }

    /// The indirect object this ID names, when it is an object ID (not a path ID).
    #[must_use]
    pub const fn obj_ref(&self) -> Option<ObjRef> {
        match self.anchor {
            Anchor::Object(r) if self.path.is_empty() => Some(r),
            _ => None,
        }
    }

    /// True for a path ID (a direct dictionary).
    #[must_use]
    pub const fn is_path(&self) -> bool {
        !self.path.is_empty()
    }
}

impl fmt::Display for LaminaId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind.letter())?;
        match self.anchor {
            Anchor::Object(r) => write!(f, "{}.{}", r.num, r.gen_nr)?,
            Anchor::Trailer => f.write_str("t")?,
        }
        for segment in &self.path {
            match segment {
                Segment::Key(k) => write!(f, "/{}", escape_key(k))?,
                Segment::Index(i) => write!(f, "/{i}")?,
            }
        }
        Ok(())
    }
}

/// Why a string is not a [`LaminaId`] (or not one of the expected kind).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ParseIdError {
    /// The string does not have the ID syntax.
    #[error("not a lamina ID: {0:?}")]
    Syntax(String),
    /// The ID is valid but of another kind than the typed ID asked for.
    #[error("expected a {expected:?} ID, found a {found:?} ID")]
    WrongKind {
        /// The kind the typed ID requires.
        expected: Kind,
        /// The kind the string has.
        found: Kind,
    },
}

/// A canonical unsigned decimal: digits only, no sign, no leading zero (except `0`).
fn canonical_number<T: FromStr>(s: &str) -> Option<T> {
    let canonical = !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) && (s == "0" || !s.starts_with('0'));
    canonical.then(|| s.parse().ok()).flatten()
}

/// Writes a dictionary key so it reads back as the same key and never as an index: every
/// byte outside `[A-Za-z0-9]` and a leading digit become `#XX` (as in PDF names), and the
/// empty key is a lone `#`.
fn escape_key(key: &str) -> String {
    if key.is_empty() {
        return "#".to_owned();
    }
    let mut out = String::with_capacity(key.len());
    for (i, b) in key.bytes().enumerate() {
        if b.is_ascii_alphabetic() || (b.is_ascii_digit() && i > 0) {
            out.push(char::from(b));
        } else {
            // Writing to a `String` cannot fail.
            let _ = write!(out, "#{b:02X}");
        }
    }
    out
}

/// The inverse of [`escape_key`]; `None` for anything [`escape_key`] can't have written.
fn unescape_key(part: &str) -> Option<String> {
    if part == "#" {
        return Some(String::new());
    }
    let bytes = part.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while let Some(&b) = bytes.get(i) {
        if b == b'#' {
            let hex = part.get(i + 1..i + 3)?;
            if !hex.bytes().all(|h| h.is_ascii_digit() || (b'A'..=b'F').contains(&h)) {
                return None;
            }
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else if b.is_ascii_alphanumeric() && !(i == 0 && b.is_ascii_digit()) {
            out.push(b);
            i += 1;
        } else {
            return None;
        }
    }
    let key = String::from_utf8(out).ok()?;
    // Only the canonical spelling parses, so a string form maps to exactly one ID.
    (escape_key(&key) == part).then_some(key)
}

impl FromStr for LaminaId {
    type Err = ParseIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || ParseIdError::Syntax(s.to_owned());
        let mut chars = s.chars();
        let kind = chars.next().and_then(Kind::from_letter).ok_or_else(err)?;
        let mut parts = chars.as_str().split('/');
        let anchor = match parts.next().ok_or_else(err)? {
            "t" => Anchor::Trailer,
            head => {
                let (num, gen_nr) = head.split_once('.').ok_or_else(err)?;
                let num = canonical_number(num).ok_or_else(err)?;
                let gen_nr = canonical_number(gen_nr).ok_or_else(err)?;
                Anchor::Object(ObjRef::new(num, gen_nr))
            }
        };
        let path = parts
            .map(|part| {
                if let Some(i) = canonical_number(part) {
                    Some(Segment::Index(i))
                } else {
                    unescape_key(part).map(|k| Segment::Key(k.into()))
                }
            })
            .collect::<Option<Box<[Segment]>>>()
            .ok_or_else(err)?;
        Ok(Self { kind, anchor, path })
    }
}

macro_rules! typed_id {
    ($(#[$doc:meta])* $name:ident, $kind:expr) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(LaminaId);

        impl $name {
            /// The kind every ID of this type has.
            pub const KIND: Kind = $kind;

            /// The ID of the indirect object `r`.
            #[must_use]
            pub fn object(r: ObjRef) -> Self {
                Self(LaminaId::object(Self::KIND, r))
            }

            /// The path ID of a direct dictionary at `path`.
            #[must_use]
            pub fn path(path: &DirectPath) -> Self {
                Self(LaminaId::path(Self::KIND, path))
            }

            /// The untyped ID.
            #[must_use]
            pub const fn as_lamina_id(&self) -> &LaminaId {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl FromStr for $name {
            type Err = ParseIdError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::try_from(s.parse::<LaminaId>()?)
            }
        }

        impl TryFrom<LaminaId> for $name {
            type Error = ParseIdError;

            fn try_from(id: LaminaId) -> Result<Self, Self::Error> {
                if id.kind == Self::KIND {
                    Ok(Self(id))
                } else {
                    Err(ParseIdError::WrongKind {
                        expected: Self::KIND,
                        found: id.kind,
                    })
                }
            }
        }

        impl From<$name> for LaminaId {
            fn from(id: $name) -> Self {
                id.0
            }
        }
    };
}

typed_id!(
    /// The [`LaminaId`] of a page. Kept when the page moves (page ops never replace the page
    /// object, ADR 0005 §5).
    PageId,
    Kind::Page
);
typed_id!(
    /// The [`LaminaId`] of an outline item.
    OutlineItemId,
    Kind::OutlineItem
);
typed_id!(
    /// The [`LaminaId`] of an annotation (views arrive with the editor rounds).
    AnnotId,
    Kind::Annot
);
typed_id!(
    /// The [`LaminaId`] of an interactive form field (views arrive with the editor rounds).
    FieldId,
    Kind::Field
);

/// The document-level ID allocator (ADR 0005 §5): a borrowed view of the COS overlay's
/// number allocator, returned by [`Document::ids`](crate::Document::ids).
///
/// Existing items need no allocation: their ID is derived from their object (or path) when
/// a view first sees them, so the same bytes always give the same IDs and nothing is
/// recorded per walk. New objects get theirs when the overlay allocates a number for them
/// ([`lamina_cos::Document::reserve_number`], also used by [`lamina_cos::CosMut::create`]):
/// numbers start above both the trailer's `/Size` and the highest object number in any
/// xref section (as the incremental writer does, so a dangling reference below `/Size`
/// never resolves to a new object), stay within
/// [`Limits::max_object_number`](lamina_cos::Limits::max_object_number), and are never
/// reused in a session, so neither are IDs.
#[derive(Debug, Clone, Copy)]
pub struct IdAllocator<'d> {
    cos: &'d lamina_cos::Document,
}

impl<'d> IdAllocator<'d> {
    /// The allocator of `cos`.
    #[must_use]
    pub const fn new(cos: &'d lamina_cos::Document) -> Self {
        Self { cos }
    }

    /// Whether this document handed out `id`'s object number (existing items' IDs are
    /// derived, not allocated, so this never depends on which views were read).
    #[must_use]
    pub fn is_allocated(&self, id: &LaminaId) -> bool {
        id.obj_ref()
            .is_some_and(|r| r.gen_nr == 0 && self.cos.is_reserved_number(r.num))
    }

    /// Allocates a fresh object number (generation 0) and its ID as an item of `kind`. The
    /// object itself is created by a later [`lamina_cos::CosMut::set`].
    ///
    /// # Errors
    /// [`Error::InvalidStructure`] once the next number would exceed the document's
    /// [`Limits::max_object_number`](lamina_cos::Limits::max_object_number).
    pub fn allocate(&self, kind: Kind) -> Result<(ObjRef, LaminaId), Error> {
        let r = self.cos.reserve_number()?;
        Ok((r, LaminaId::object(kind, r)))
    }
}

#[cfg(test)]
mod tests {
    use super::{escape_key, unescape_key};

    #[test]
    fn keys_with_any_characters_round_trip_and_never_read_as_indexes() {
        for key in ["Kids", "My-Key", "A_B", "3D", "0", "", "Ünï", "a/b#c"] {
            let escaped = escape_key(key);
            assert_eq!(unescape_key(&escaped).as_deref(), Some(key), "{key:?} -> {escaped:?}");
            assert!(escaped.parse::<usize>().is_err(), "{key:?} must not look like an index");
        }
    }

    #[test]
    fn non_canonical_key_spellings_are_rejected() {
        for part in ["#4B", "my-key", "#zz", "#4"] {
            assert_eq!(unescape_key(part), None, "{part:?}");
        }
    }

    use super::*;

    fn round_trip(s: &str) -> String {
        s.parse::<LaminaId>().unwrap().to_string()
    }

    #[test]
    fn object_ids_round_trip_through_their_string_form() {
        for s in ["p12.0", "o1.3", "a0.0", "f4294967295.65535"] {
            assert_eq!(round_trip(s), s);
        }
    }

    #[test]
    fn path_ids_round_trip_through_their_string_form() {
        for s in [
            "p5.0/Kids/3",
            "p1.0/Pages/Kids/0/Kids/12",
            "pt/Root/Pages/Kids/0",
            "o9.0/First/Next",
        ] {
            assert_eq!(round_trip(s), s);
        }
    }

    #[test]
    fn parse_rejects_malformed_strings() {
        for s in [
            "",
            "p",
            "x1.0",
            "p1",
            "p1.",
            "p.0",
            "p01.0",
            "p1.00",
            "p-1.0",
            "p1.0/",
            "p1.0//Kids",
            "p1.0/Ki ds",
            "p1.0/3x",
            "p4294967296.0",
            "p1.65536",
            "p1.0/07",
        ] {
            assert!(s.parse::<LaminaId>().is_err(), "{s:?}");
        }
    }

    #[test]
    fn path_id_from_a_direct_path_names_anchor_and_steps() {
        let path = DirectPath::new(
            Some(ObjRef::new(5, 0)),
            vec![PathSegment::Key("Kids"), PathSegment::Index(3)],
        );
        assert_eq!(PageId::path(&path).to_string(), "p5.0/Kids/3");
    }

    #[test]
    fn obj_ref_is_only_set_for_object_ids() {
        let object: LaminaId = "p3.1".parse().unwrap();
        let path: LaminaId = "p3.1/Kids/0".parse().unwrap();
        assert_eq!((object.obj_ref(), path.obj_ref()), (Some(ObjRef::new(3, 1)), None));
    }

    #[test]
    fn typed_ids_reject_another_kind() {
        assert_eq!(
            "o3.0".parse::<PageId>(),
            Err(ParseIdError::WrongKind {
                expected: Kind::Page,
                found: Kind::OutlineItem
            })
        );
    }

    #[test]
    fn typed_ids_parse_their_own_kind() {
        let id: OutlineItemId = "o3.0".parse().unwrap();
        assert_eq!(id, OutlineItemId::object(ObjRef::new(3, 0)));
    }

    #[test]
    fn ids_of_different_kinds_for_one_object_differ() {
        let r = ObjRef::new(7, 0);
        assert_ne!(LaminaId::object(Kind::Page, r), LaminaId::object(Kind::Annot, r));
    }

    #[test]
    fn a_trailer_rooted_empty_path_round_trips() {
        let id = LaminaId::path(Kind::Page, &DirectPath::new(None, Vec::new()));
        assert_eq!(id.to_string().parse::<LaminaId>().unwrap(), id);
    }
}
