//! Tables written by `tools/std14-gen` (`cargo run -p std14-gen`) from the Adobe Core 14 AFM
//! files, ISO 32000-2 Annex D and the Adobe Glyph List in `tools/std14-gen/data/` (sources,
//! licences and SHA-256 in its README). Never edit the `*.rs` files next to this one by hand:
//! the generator's `committed_tables_are_current` test fails when they differ from a
//! regeneration. This file is hand-written.

#[rustfmt::skip]
pub(crate) mod agl;
#[rustfmt::skip]
pub(crate) mod chars;
#[rustfmt::skip]
pub(crate) mod encodings;
#[rustfmt::skip]
pub(crate) mod metrics;

/// An undefined code in an encoding table.
pub(crate) const NONE: u16 = u16::MAX;

/// What the AFM file of one standard 14 font says (Adobe's AFM specification, version 4.1):
/// the header values in glyph space units (thousandths of an em; y grows upwards) and the
/// advance width of every glyph.
///
/// Read it with [`Standard14::metrics`](crate::Standard14::metrics).
pub struct AfmMetrics {
    /// `FontName`, the `/BaseFont` (`Helvetica-Bold`).
    pub font_name: &'static str,
    /// `FamilyName` (`Helvetica`).
    pub family_name: &'static str,
    /// `Weight` (`Medium`, `Roman`, `Bold`).
    pub weight: &'static str,
    /// `ItalicAngle` in degrees, counter-clockwise from vertical (negative for a font that
    /// leans right).
    pub italic_angle: f32,
    /// `IsFixedPitch`.
    pub is_fixed_pitch: bool,
    /// `FontBBox` `[x_min, y_min, x_max, y_max]`.
    pub bbox: [i16; 4],
    /// `CapHeight` (the symbolic fonts have none).
    pub cap_height: Option<i16>,
    /// `XHeight` (the symbolic fonts have none).
    pub x_height: Option<i16>,
    /// `Ascender` (the symbolic fonts have none).
    pub ascender: Option<i16>,
    /// `Descender`, negative (the symbolic fonts have none).
    pub descender: Option<i16>,
    /// `StdHW`, the dominant horizontal stem width.
    pub std_hw: u16,
    /// `StdVW`, the dominant vertical stem width (written as `/StemV`).
    pub std_vw: u16,
    /// The glyph names, sorted.
    pub(crate) names: &'static [&'static str],
    /// The advance width (`WX`) of each glyph in `names`.
    pub(crate) widths: &'static [u16],
}

impl std::fmt::Debug for AfmMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AfmMetrics")
            .field("font_name", &self.font_name)
            .field("glyphs", &self.names.len())
            .finish_non_exhaustive()
    }
}
