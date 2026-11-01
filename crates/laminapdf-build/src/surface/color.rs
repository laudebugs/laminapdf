//! Colour (ISO 32000-2 §8.6): device colours and ICC-based colour spaces with caller-supplied
//! profiles.

use std::sync::Arc;

use sha2::{Digest, Sha256};

use crate::error::{Error, Result};

/// A colour in a device colour space or an ICC-based one. Components are in `[0, 1]`
/// (constructors clamp them; NaN reads as 0).
///
/// ```
/// use laminapdf_build::Color;
///
/// assert_eq!(Color::hex("#3366cc")?, Color::rgb(0.2, 0.4, 0.8));
/// assert_eq!(Color::hex("#fff")?, Color::rgb(1.0, 1.0, 1.0));
/// # Ok::<(), laminapdf_build::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Color {
    /// `DeviceGray`: 0 is black, 1 white.
    Gray(f64),
    /// `DeviceRGB`.
    Rgb(f64, f64, f64),
    /// `DeviceCMYK`.
    Cmyk(f64, f64, f64, f64),
    /// An `ICCBased` colour space (§8.6.5.5) and its components.
    Icc(IccProfile, Vec<f64>),
}

/// A component clamped to `[0, 1]`, NaN as 0.
fn unit(v: f64) -> f64 {
    if v.is_nan() { 0.0 } else { v.clamp(0.0, 1.0) }
}

impl Color {
    /// Black in `DeviceGray`.
    pub const BLACK: Self = Self::Gray(0.0);

    /// A `DeviceGray` level.
    #[must_use]
    pub fn gray(level: f64) -> Self {
        Self::Gray(unit(level))
    }

    /// A `DeviceRGB` colour.
    #[must_use]
    pub fn rgb(r: f64, g: f64, b: f64) -> Self {
        Self::Rgb(unit(r), unit(g), unit(b))
    }

    /// A `DeviceRGB` colour from 8-bit components.
    #[must_use]
    pub fn rgb8(r: u8, g: u8, b: u8) -> Self {
        let c = |v: u8| f64::from(v) / 255.0;
        Self::Rgb(c(r), c(g), c(b))
    }

    /// A `DeviceCMYK` colour.
    #[must_use]
    pub fn cmyk(c: f64, m: f64, y: f64, k: f64) -> Self {
        Self::Cmyk(unit(c), unit(m), unit(y), unit(k))
    }

    /// A colour in the ICC profile's colour space: one component per channel of the profile
    /// (`/N`).
    ///
    /// # Errors
    /// [`Error::ColorComponents`] when `components` has another length.
    pub fn icc(profile: &IccProfile, components: &[f64]) -> Result<Self> {
        if components.len() != profile.components() {
            return Err(Error::ColorComponents {
                expected: profile.components(),
                actual: components.len(),
            });
        }
        Ok(Self::Icc(
            profile.clone(),
            components.iter().copied().map(unit).collect(),
        ))
    }

    /// `#rrggbb` or `#rgb` (hexadecimal, either case) as a `DeviceRGB` colour.
    ///
    /// # Errors
    /// [`Error::InvalidColor`] for anything else.
    pub fn hex(text: &str) -> Result<Self> {
        let invalid = || Error::InvalidColor { text: text.to_owned() };
        let digits = text.strip_prefix('#').ok_or_else(invalid)?;
        if !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(invalid());
        }
        let nibble = |i: usize| -> Result<u8> {
            let s = digits.get(i..=i).ok_or_else(invalid)?;
            u8::from_str_radix(s, 16).map_err(|_| invalid())
        };
        match digits.len() {
            3 => Ok(Self::rgb8(nibble(0)? * 17, nibble(1)? * 17, nibble(2)? * 17)),
            6 => Ok(Self::rgb8(
                nibble(0)? * 16 + nibble(1)?,
                nibble(2)? * 16 + nibble(3)?,
                nibble(4)? * 16 + nibble(5)?,
            )),
            _ => Err(invalid()),
        }
    }
}

/// An ICC profile (ICC.1:2010; ISO 32000-2 §8.6.5.5) given by the caller, checked when it is
/// made: its header's size, `acsp` signature, profile class and data colour space, which gives
/// `/N` (Gray 1, RGB 3, CMYK 4; Lab profiles, whose components need a `/Range` outside
/// `[0, 1]`, are refused). Cheap to clone; the builder writes each distinct
/// profile once per document.
#[derive(Clone, PartialEq, Eq)]
pub struct IccProfile {
    inner: Arc<IccData>,
}

#[derive(PartialEq, Eq)]
struct IccData {
    bytes: Vec<u8>,
    space: IccSpace,
    hash: [u8; 32],
}

/// The data colour space of a profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IccSpace {
    Gray,
    Rgb,
    Cmyk,
}

impl std::fmt::Debug for IccProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IccProfile")
            .field("bytes", &self.inner.bytes.len())
            .field("space", &self.inner.space)
            .finish()
    }
}

impl IccProfile {
    /// Checks and keeps `bytes`.
    ///
    /// # Errors
    /// [`Error::InvalidIccProfile`] for bytes shorter than a header and tag count, a declared
    /// size that doesn't fit, no `acsp` signature, a device-link, abstract or named-colour
    /// profile, or a data colour space other than Gray, RGB or CMYK.
    pub fn new(bytes: Vec<u8>) -> Result<Self> {
        let invalid = |reason| Error::InvalidIccProfile { reason };
        let field = |at: usize| -> Option<[u8; 4]> { bytes.get(at..at + 4)?.try_into().ok() };
        if bytes.len() < 132 {
            return Err(invalid("shorter than the 128-byte header and the tag count"));
        }
        let declared = field(0).map_or(0, u32::from_be_bytes);
        if usize::try_from(declared).map_or(true, |d| d < 132 || d > bytes.len()) {
            return Err(invalid("the header's profile size does not match the bytes"));
        }
        if field(36) != Some(*b"acsp") {
            return Err(invalid("no 'acsp' profile file signature"));
        }
        if matches!(field(12).as_ref(), Some(b"link" | b"abst" | b"nmcl")) {
            return Err(invalid("a device-link, abstract or named-colour profile"));
        }
        let space = match field(16).as_ref() {
            Some(b"GRAY") => IccSpace::Gray,
            Some(b"RGB ") => IccSpace::Rgb,
            Some(b"CMYK") => IccSpace::Cmyk,
            // Lab profiles take components outside [0, 1] and need a /Range: not supported.
            _ => return Err(invalid("the data colour space is not Gray, RGB or CMYK")),
        };
        let hash = Sha256::digest(&bytes).into();
        Ok(Self {
            inner: Arc::new(IccData { bytes, space, hash }),
        })
    }

    /// The number of colour components (`/N`).
    #[must_use]
    pub fn components(&self) -> usize {
        match self.inner.space {
            IccSpace::Gray => 1,
            IccSpace::Rgb => 3,
            IccSpace::Cmyk => 4,
        }
    }

    /// The profile bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.inner.bytes
    }

    pub(crate) fn space(&self) -> IccSpace {
        self.inner.space
    }

    pub(crate) fn hash(&self) -> [u8; 32] {
        self.inner.hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A header-only profile (`size`, `class`, `space`), enough for the checks.
    pub(crate) fn header(class: [u8; 4], space: [u8; 4]) -> Vec<u8> {
        let mut b = vec![0; 132];
        b[..4].copy_from_slice(&132u32.to_be_bytes());
        b[12..16].copy_from_slice(&class);
        b[16..20].copy_from_slice(&space);
        b[36..40].copy_from_slice(b"acsp");
        b
    }

    mod hex {
        use super::*;

        #[test]
        fn parses_six_digits_in_either_case() {
            assert_eq!(Color::hex("#FF8000").unwrap(), Color::rgb8(255, 128, 0));
        }

        #[test]
        fn rejects_a_missing_hash_sign() {
            assert_eq!(Color::hex("ff8000").unwrap_err().code(), "invalid-color");
        }

        #[test]
        fn rejects_non_hex_digits_and_other_lengths() {
            for text in ["#ggg", "#12345", "#", "#ff80001", "#é12"] {
                assert!(Color::hex(text).is_err(), "{text}");
            }
        }
    }

    #[test]
    fn device_components_are_clamped_and_nan_is_zero() {
        assert_eq!(Color::rgb(-1.0, 2.0, f64::NAN), Color::Rgb(0.0, 1.0, 0.0));
    }

    mod icc {
        use super::*;

        #[test]
        fn takes_n_from_the_data_colour_space() {
            let n = |space: &[u8; 4]| IccProfile::new(header(*b"mntr", *space)).unwrap().components();
            assert_eq!([n(b"GRAY"), n(b"RGB "), n(b"CMYK")], [1, 3, 4]);
        }

        #[test]
        fn rejects_a_profile_without_the_signature() {
            let mut b = header(*b"mntr", *b"RGB ");
            b[36] = b'x';
            assert!(matches!(IccProfile::new(b), Err(Error::InvalidIccProfile { .. })));
        }

        #[test]
        fn rejects_a_declared_size_past_the_end() {
            let mut b = header(*b"mntr", *b"RGB ");
            b[..4].copy_from_slice(&500u32.to_be_bytes());
            assert!(IccProfile::new(b).is_err());
        }

        #[test]
        fn rejects_device_link_profiles_and_other_spaces() {
            assert!(IccProfile::new(header(*b"link", *b"RGB ")).is_err());
            assert!(IccProfile::new(header(*b"mntr", *b"XYZ ")).is_err());
            assert!(IccProfile::new(header(*b"spac", *b"Lab ")).is_err());
        }

        #[test]
        fn a_colour_needs_n_components() {
            let p = IccProfile::new(header(*b"prtr", *b"CMYK")).unwrap();
            assert_eq!(
                Color::icc(&p, &[0.1, 0.2, 0.3]).unwrap_err(),
                Error::ColorComponents { expected: 4, actual: 3 }
            );
        }
    }
}
