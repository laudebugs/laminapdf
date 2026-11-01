//! Resource limits applied while reading untrusted input ([ADR 0003] §3).
//!
//! Every number read from a file is attacker-controlled, so everything that can grow with the
//! input (nesting, decoded bytes, reference chains, …) is bounded by a [`Limits`] value.
//! **Hard** limits fail the call that hits them with [`Error::LimitExceeded`]; **soft** limits
//! drop the item over the limit and carry on with a warning (the reader in `laminapdf-cos`
//! raises it).
//!
//! [ADR 0003]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0003-reader-tolerance-and-limits.md
//! [`Error::LimitExceeded`]: crate::Error::LimitExceeded

use core::fmt;

/// Which hard limit was exceeded (the `limit` of [`Error::LimitExceeded`](crate::Error::LimitExceeded)).
///
/// Soft limits never produce an error, so they have no variant here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Limit {
    /// [`Limits::max_nesting_depth`].
    NestingDepth,
    /// [`Limits::max_stream_decoded`].
    StreamDecoded,
    /// [`Limits::max_decoded_total`].
    DecodedTotal,
    /// [`Limits::max_filter_chain`].
    FilterChain,
    /// [`Limits::max_resolve_depth`].
    ResolveDepth,
    /// [`Limits::max_page_tree_depth`]. The only reader of the page tree today is
    /// `Document::pages`, one of ADR 0003's tolerant paths: it skips a subtree past this
    /// limit with the `page-tree-node-skipped` warning (an `Error::Strict` in strict mode)
    /// instead of returning this error (ADR 0003 §3). Typed views (Round B2c) that read the
    /// tree outside that walk return it.
    PageTreeDepth,
    /// [`Limits::max_tree_depth`].
    TreeDepth,
    /// [`Limits::max_string_length`].
    StringLength,
    /// [`Limits::max_content_operators`].
    ContentOperators,
    /// [`Limits::max_xobject_depth`].
    XObjectDepth,
    /// [`Limits::max_image_pixels`].
    ImagePixels,
    /// [`Limits::max_read_window`].
    ReadWindow,
}

impl Limit {
    /// Every hard limit, in declaration order.
    pub const ALL: [Self; 12] = [
        Self::NestingDepth,
        Self::StreamDecoded,
        Self::DecodedTotal,
        Self::FilterChain,
        Self::ResolveDepth,
        Self::PageTreeDepth,
        Self::TreeDepth,
        Self::StringLength,
        Self::ContentOperators,
        Self::XObjectDepth,
        Self::ImagePixels,
        Self::ReadWindow,
    ];

    /// The name of the [`Limits`] field, e.g. `"max_nesting_depth"`. Stable: part of the
    /// public API in Rust, WASM and TypeScript (listed in `fixtures/codes.json`).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::NestingDepth => "max_nesting_depth",
            Self::StreamDecoded => "max_stream_decoded",
            Self::DecodedTotal => "max_decoded_total",
            Self::FilterChain => "max_filter_chain",
            Self::ResolveDepth => "max_resolve_depth",
            Self::PageTreeDepth => "max_page_tree_depth",
            Self::TreeDepth => "max_tree_depth",
            Self::StringLength => "max_string_length",
            Self::ContentOperators => "max_content_operators",
            Self::XObjectDepth => "max_xobject_depth",
            Self::ImagePixels => "max_image_pixels",
            Self::ReadWindow => "max_read_window",
        }
    }

    /// The configured value of this limit in `limits`.
    #[must_use]
    pub const fn value_in(self, limits: &Limits) -> u64 {
        // `usize` is at most 64 bits on every supported target, so the casts are lossless.
        match self {
            Self::NestingDepth => limits.max_nesting_depth as u64,
            Self::StreamDecoded => limits.max_stream_decoded as u64,
            Self::DecodedTotal => limits.max_decoded_total as u64,
            Self::FilterChain => limits.max_filter_chain as u64,
            Self::ResolveDepth => limits.max_resolve_depth as u64,
            Self::PageTreeDepth => limits.max_page_tree_depth as u64,
            Self::TreeDepth => limits.max_tree_depth as u64,
            Self::StringLength => limits.max_string_length as u64,
            Self::ContentOperators => limits.max_content_operators,
            Self::XObjectDepth => limits.max_xobject_depth as u64,
            Self::ImagePixels => limits.max_image_pixels,
            Self::ReadWindow => limits.max_read_window as u64,
        }
    }
}

impl fmt::Display for Limit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Resource limits for reading untrusted PDF input ([ADR 0003] §3).
///
/// `Copy`, with private fields: read a limit with its getter and change it with the
/// matching `with_*` builder, starting from [`Limits::default`] (or [`Limits::new`] in a
/// `const` context):
///
/// ```
/// use laminapdf_syntax::Limits;
///
/// const STRICTER: Limits = Limits::new().with_max_nesting_depth(64).with_max_stream_decoded(1 << 20);
/// assert_eq!(STRICTER.max_nesting_depth(), 64);
/// assert_eq!(Limits::default().max_nesting_depth(), 256);
/// ```
///
/// | Field | Kind | Default |
/// | --- | --- | --- |
/// | [`max_nesting_depth`](Self::max_nesting_depth) | hard | 256 |
/// | [`max_stream_decoded`](Self::max_stream_decoded) | hard | 64 MiB |
/// | [`max_decoded_total`](Self::max_decoded_total) | hard | 1 GiB |
/// | [`max_filter_chain`](Self::max_filter_chain) | hard | 8 |
/// | [`max_resolve_depth`](Self::max_resolve_depth) | hard | 32 |
/// | [`max_page_tree_depth`](Self::max_page_tree_depth) | hard | 256 |
/// | [`max_tree_depth`](Self::max_tree_depth) | hard | 64 |
/// | [`max_string_length`](Self::max_string_length) | hard | 16 MiB |
/// | [`max_xref_sections`](Self::max_xref_sections) | soft | 4,096 |
/// | [`max_object_number`](Self::max_object_number) | soft | 8,388,607 |
/// | [`max_warnings`](Self::max_warnings) | soft | 1,000 |
/// | [`max_content_operators`](Self::max_content_operators) | hard | 10,000,000 |
/// | [`max_xobject_depth`](Self::max_xobject_depth) | hard | 32 |
/// | [`max_image_pixels`](Self::max_image_pixels) | hard (reserved) | 2^28 |
/// | [`max_read_window`](Self::max_read_window) | hard | 256 MiB |
///
/// Reserved limits are part of the API now so they can be configured once, but nothing
/// enforces them until the feature they bound lands (image decoding).
///
/// [ADR 0003]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0003-reader-tolerance-and-limits.md
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[expect(
    clippy::struct_field_names,
    reason = "the field names are the ADR's public limit names (`max_*`)"
)]
pub struct Limits {
    max_nesting_depth: usize,
    max_stream_decoded: usize,
    max_decoded_total: usize,
    max_filter_chain: usize,
    max_resolve_depth: usize,
    max_page_tree_depth: usize,
    max_tree_depth: usize,
    max_string_length: usize,
    max_xref_sections: usize,
    max_object_number: u32,
    max_warnings: usize,
    max_content_operators: u64,
    max_xobject_depth: usize,
    max_image_pixels: u64,
    max_read_window: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self::new()
    }
}

macro_rules! limit_accessors {
    ($( $(#[$doc:meta])* $field:ident, $with:ident: $ty:ty; )*) => {
        $(
            $(#[$doc])*
            #[must_use]
            pub const fn $field(&self) -> $ty {
                self.$field
            }

            #[doc = concat!("Sets [`", stringify!($field), "`](Self::", stringify!($field), ").")]
            #[must_use]
            pub const fn $with(mut self, value: $ty) -> Self {
                self.$field = value;
                self
            }
        )*
    };
}

impl Limits {
    /// The defaults from ADR 0003 (the same as [`Limits::default`], usable in `const`).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            max_nesting_depth: 256,
            max_stream_decoded: 64 * 1024 * 1024,
            max_decoded_total: 1024 * 1024 * 1024,
            max_filter_chain: 8,
            max_resolve_depth: 32,
            max_page_tree_depth: 256,
            max_tree_depth: 64,
            max_string_length: 16 * 1024 * 1024,
            max_xref_sections: 4096,
            // ISO 32000-1 Annex C: the largest object number a conforming reader must support.
            max_object_number: 8_388_607,
            max_warnings: 1000,
            max_content_operators: 10_000_000,
            max_xobject_depth: 32,
            max_image_pixels: 1 << 28,
            max_read_window: 256 * 1024 * 1024,
        }
    }

    /// No limits at all, for trusted input only (lamina's own output in tests).
    ///
    /// Nesting is parsed recursively, so hostile input with unlimited nesting depth can
    /// exhaust the stack: never use this for files from outside.
    #[must_use]
    pub const fn unlimited() -> Self {
        Self {
            max_nesting_depth: usize::MAX,
            max_stream_decoded: usize::MAX,
            max_decoded_total: usize::MAX,
            max_filter_chain: usize::MAX,
            max_resolve_depth: usize::MAX,
            max_page_tree_depth: usize::MAX,
            max_tree_depth: usize::MAX,
            max_string_length: usize::MAX,
            max_xref_sections: usize::MAX,
            max_object_number: u32::MAX,
            max_warnings: usize::MAX,
            max_content_operators: u64::MAX,
            max_xobject_depth: usize::MAX,
            max_image_pixels: u64::MAX,
            max_read_window: usize::MAX,
        }
    }

    limit_accessors! {
        /// Hard: array/dictionary nesting depth accepted by the parser, and the graphics states
        /// one content stream may save with `q` in `laminapdf-content` (default 256; ADR 0023 §4).
        max_nesting_depth, with_max_nesting_depth: usize;
        /// Hard: bytes one stream may decode to, per filter stage (default 64 MiB).
        max_stream_decoded, with_max_stream_decoded: usize;
        /// Hard: bytes decoded by one top-level call (`open`, one page's content, one
        /// `resolve` or `decode_stream`), across all streams and filter stages (default 1 GiB).
        max_decoded_total, with_max_decoded_total: usize;
        /// Hard: filters in one stream's `/Filter` chain (default 8).
        max_filter_chain, with_max_filter_chain: usize;
        /// Hard: indirect objects in flight at once while resolving a reference chain (default 32).
        max_resolve_depth, with_max_resolve_depth: usize;
        /// Hard: `/Kids` levels below the page-tree root (default 256). Hit inside the
        /// tolerant page walk, the subtree is skipped with a `page-tree-node-skipped` warning,
        /// as ADR 0003 §3 prescribes for hard limits on tolerant paths.
        max_page_tree_depth, with_max_page_tree_depth: usize;
        /// Hard: depth of name trees, number trees and outlines (default 64; enforced by the
        /// typed views).
        max_tree_depth, with_max_tree_depth: usize;
        /// Hard: bytes in one string or name token (default 16 MiB).
        max_string_length, with_max_string_length: usize;
        /// Soft: `/Prev` hops in the xref chain; past it the stored xref is abandoned and
        /// rebuilt by scanning (default 4,096).
        max_xref_sections, with_max_xref_sections: usize;
        /// Soft: highest object number kept; higher entries are dropped with a warning
        /// (default 8,388,607, ISO 32000-1 Annex C).
        max_object_number, with_max_object_number: u32;
        /// Soft: warnings kept per document; later ones are dropped after one
        /// `warnings-truncated` warning (default 1,000).
        max_warnings, with_max_warnings: usize;
        /// Hard: operators read by one content-stream interpretation (`laminapdf-content`), the
        /// page's and every Form XObject's each time it is drawn (default 10,000,000; ADR 0023).
        max_content_operators, with_max_content_operators: u64;
        /// Hard: Form `XObject`s drawn at once by `laminapdf-content`; patterns and Type 3 glyphs
        /// too once they are interpreted (default 32; ADR 0023).
        max_xobject_depth, with_max_xobject_depth: usize;
        /// Hard, reserved: pixels in one decoded image (default 2^28).
        max_image_pixels, with_max_image_pixels: u64;
        /// Hard: bytes one parse may read from a source without an in-memory slice
        /// (`laminapdf-cos` reads such sources in windows; default 256 MiB). It bounds how far a
        /// window grows for one object (its head, a stream's data, the search for a missing
        /// `endstream`) and the backward walk over an object header found by repair. Past
        /// it, the object fails with `LimitExceeded` where an in-memory parse would have read
        /// on; a tolerant path (repair, `/Length` recovery, the xref entry check) treats the
        /// item as unreadable instead. In-memory input is never limited by it.
        max_read_window, with_max_read_window: usize;
    }
}

/// The decoding budget of one top-level call: [`Limits::max_decoded_total`] bytes, spent by
/// every filter stage of every stream the call decodes.
///
/// Create one per call ([`DecodeBudget::new`]) and pass it to every
/// [`filters::decode`](crate::filters::decode) the call makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodeBudget {
    remaining: usize,
}

impl DecodeBudget {
    /// A fresh budget of `limits.max_decoded_total()` bytes.
    #[must_use]
    pub const fn new(limits: &Limits) -> Self {
        Self {
            remaining: limits.max_decoded_total,
        }
    }

    /// Bytes that may still be decoded.
    #[must_use]
    pub const fn remaining(&self) -> usize {
        self.remaining
    }

    /// Records `bytes` of decoded output.
    pub(crate) const fn spend(&mut self, bytes: usize) {
        self.remaining = self.remaining.saturating_sub(bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_adr_table() {
        let l = Limits::default();
        assert_eq!(
            [
                l.max_nesting_depth(),
                l.max_stream_decoded(),
                l.max_decoded_total(),
                l.max_filter_chain(),
                l.max_resolve_depth(),
                l.max_page_tree_depth(),
                l.max_tree_depth(),
                l.max_string_length(),
                l.max_xref_sections(),
                l.max_warnings(),
                l.max_xobject_depth(),
                l.max_read_window(),
            ],
            [
                256,
                64 << 20,
                1 << 30,
                8,
                32,
                256,
                64,
                16 << 20,
                4096,
                1000,
                32,
                256 << 20
            ]
        );
    }

    #[test]
    fn defaults_for_wide_limits_match_the_adr_table() {
        let l = Limits::default();
        assert_eq!(
            (l.max_object_number(), l.max_content_operators(), l.max_image_pixels()),
            (8_388_607, 10_000_000, 1 << 28)
        );
    }

    #[test]
    fn with_builder_changes_only_its_field() {
        let l = Limits::default().with_max_filter_chain(2);
        assert_eq!(
            l,
            Limits {
                max_filter_chain: 2,
                ..Limits::default()
            }
        );
    }

    #[test]
    fn limit_value_in_reads_the_matching_field() {
        let l = Limits::default().with_max_string_length(7);
        assert_eq!(Limit::StringLength.value_in(&l), 7);
    }

    #[test]
    fn limit_names_are_distinct() {
        let mut names: Vec<_> = Limit::ALL.iter().map(|l| l.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), Limit::ALL.len());
    }
}
