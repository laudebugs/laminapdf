//! The PDF/A output intent (ISO 19005-2 §6.2.3; ISO 32000-1 §14.11.5): `/OutputIntents` with
//! one `GTS_PDFA1` intent whose `/DestOutputProfile` is a compact sRGB ICC profile, so device
//! RGB and gray colour have a defined meaning.
//!
//! The profile is `sRGB-v4.icc` from Clinton Ingram's Compact ICC Profiles (CC0-1.0; 480 bytes;
//! provenance in `data/README.md`): small enough to sit in every build, the WASM modules
//! included.

use laminapdf_cos::{CosMut, Dict, Name, Object, PdfString, Stream};

use crate::error::Result;

/// The ICC profile bytes (ICC v4.2, RGB, display class).
pub(crate) const SRGB_ICC: &[u8] = include_bytes!("../../data/sRGB-v4.icc");

/// The output condition the profile describes (the ICC registry's name for sRGB).
const CONDITION: &str = "sRGB IEC61966-2.1";

/// Writes the profile and returns the catalog's `/OutputIntents` array.
pub(crate) fn output_intents(cos: &mut CosMut<'_>) -> Result<Object> {
    let mut profile = Dict::new();
    profile.insert("N", 3);
    let profile = cos.create(Object::Stream(Stream::flate(profile, SRGB_ICC)))?;
    let mut intent = Dict::new();
    intent.insert("Type", Name::from("OutputIntent"));
    intent.insert("S", Name::from("GTS_PDFA1"));
    intent.insert(
        "OutputConditionIdentifier",
        PdfString::literal(CONDITION.as_bytes().to_vec()),
    );
    intent.insert("Info", PdfString::literal(CONDITION.as_bytes().to_vec()));
    intent.insert("DestOutputProfile", profile);
    Ok(Object::Array(vec![Object::Dict(intent)]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_profile_is_a_valid_rgb_profile() {
        let p = crate::IccProfile::new(SRGB_ICC.to_vec()).unwrap();
        assert_eq!((p.components(), SRGB_ICC.len()), (3, 480));
    }
}
