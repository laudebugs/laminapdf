//! The resolutions every check of this tool renders at, defined once for the render goldens
//! and the annotation parity suite (`cargo xtask annots-parity` asks `annots-parity settings`
//! for them rather than keeping a copy).

/// Resolution of the committed goldens: small PNGs for the repository.
pub const GOLDEN_DPI: u32 = 72;
/// Resolution of the engine-against-PDFium parity checks.
pub const PARITY_DPI: u32 = 150;

/// `dpi` as a renderer's scale input.
pub fn as_scale(dpi: u32) -> f32 {
    f32::from(u16::try_from(dpi).unwrap_or(u16::MAX))
}
