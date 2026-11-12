//! How an image file says it should be shown: its EXIF orientation and its resolution, read
//! from a JPEG's `JFIF` and `Exif` segments or a PNG's `pHYs` and `eXIf` chunks for images to PDF.
//! Reading is tolerant: a field that is missing or malformed is simply absent.
//!
//! EXIF is a TIFF structure (TIFF 6.0 §2): a byte-order mark, then directories of 12-byte
//! entries. Only IFD0's `Orientation` (0x0112), `XResolution` (0x011A), `YResolution` (0x011B)
//! and `ResolutionUnit` (0x0128) are read (CIPA DC-008 §4.6.4).

/// How a stored image is turned to be shown upright: EXIF's `Orientation` values 1–8.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum Orientation {
    /// 1: shown as stored.
    #[default]
    Upright,
    /// 2: mirrored left to right.
    Mirrored,
    /// 3: turned half a turn.
    UpsideDown,
    /// 4: mirrored top to bottom.
    MirroredUpsideDown,
    /// 5: mirrored along the top-left to bottom-right diagonal.
    Transposed,
    /// 6: turned a quarter turn clockwise.
    Clockwise,
    /// 7: mirrored along the other diagonal.
    Transversed,
    /// 8: turned a quarter turn anticlockwise.
    Anticlockwise,
}

impl Orientation {
    /// The orientation EXIF's value `value` names; `None` outside 1–8.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn from_exif(value: u16) -> Option<Self> {
        Some(match value {
            1 => Self::Upright,
            2 => Self::Mirrored,
            3 => Self::UpsideDown,
            4 => Self::MirroredUpsideDown,
            5 => Self::Transposed,
            6 => Self::Clockwise,
            7 => Self::Transversed,
            8 => Self::Anticlockwise,
            _ => return None,
        })
    }

    /// Whether showing the image swaps its width and height.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn swaps_sides(self) -> bool {
        matches!(
            self,
            Self::Transposed | Self::Clockwise | Self::Transversed | Self::Anticlockwise
        )
    }

    /// The matrix `[a b c d e f]` that maps a point of the stored image, `w` × `h` with y down,
    /// to where it is shown, also with y down.
    pub(crate) fn matrix(self, w: f64, h: f64) -> [f64; 6] {
        match self {
            Self::Upright => [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            Self::Mirrored => [-1.0, 0.0, 0.0, 1.0, w, 0.0],
            Self::UpsideDown => [-1.0, 0.0, 0.0, -1.0, w, h],
            Self::MirroredUpsideDown => [1.0, 0.0, 0.0, -1.0, 0.0, h],
            Self::Transposed => [0.0, 1.0, 1.0, 0.0, 0.0, 0.0],
            Self::Clockwise => [0.0, 1.0, -1.0, 0.0, h, 0.0],
            Self::Transversed => [0.0, -1.0, -1.0, 0.0, h, w],
            Self::Anticlockwise => [0.0, -1.0, 1.0, 0.0, 0.0, w],
        }
    }
}

/// What an image file says about showing it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Shown {
    pub(crate) orientation: Orientation,
    /// Horizontal and vertical resolution, in dots per metre.
    pub(crate) dots_per_metre: Option<(u32, u32)>,
}

impl Shown {
    /// `self`, with `other`'s fields where `self` has none.
    fn or(self, other: Self) -> Self {
        Self {
            orientation: if self.orientation == Orientation::Upright {
                other.orientation
            } else {
                self.orientation
            },
            dots_per_metre: self.dots_per_metre.or(other.dots_per_metre),
        }
    }
}

const METRES_PER_INCH: f64 = 0.0254;

/// Dots per metre from a density in `unit`s: 1 per inch, 2 per centimetre (JFIF's units, and
/// EXIF's `ResolutionUnit` minus one). `None` for a zero density or another unit.
fn per_metre(x: f64, y: f64, per_cm: bool) -> Option<(u32, u32)> {
    let scale = if per_cm { 100.0 } else { 1.0 / METRES_PER_INCH };
    let convert = |v: f64| {
        let dpm = (v * scale).round();
        // A density below one dot per metre, or one past u32, is noise, not a resolution.
        (dpm >= 1.0 && dpm <= f64::from(u32::MAX)).then(|| {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "range checked above"
            )]
            let dpm = dpm as u32;
            dpm
        })
    };
    Some((convert(x)?, convert(y)?))
}

/// What a JPEG's `APP0` (`JFIF`) and `APP1` (`Exif`) segments say: JFIF's density wins over
/// EXIF's resolution, which cameras often leave at 72 dpi.
pub(crate) fn from_jpeg_segments(jfif: Option<&[u8]>, exif: Option<&[u8]>) -> Shown {
    let jfif = jfif.and_then(|s| {
        // "JFIF\0", version (2 bytes), units, x density, y density.
        let [b'J', b'F', b'I', b'F', 0, _, _, units, x0, x1, y0, y1, ..] = *s else {
            return None;
        };
        let (x, y) = (
            f64::from(u16::from_be_bytes([x0, x1])),
            f64::from(u16::from_be_bytes([y0, y1])),
        );
        match units {
            1 => per_metre(x, y, false),
            2 => per_metre(x, y, true),
            _ => None,
        }
    });
    let exif = exif
        .and_then(|s| s.strip_prefix(b"Exif\0\0"))
        .map(from_tiff)
        .unwrap_or_default();
    Shown {
        orientation: exif.orientation,
        dots_per_metre: jfif.or(exif.dots_per_metre),
    }
}

/// What a PNG's `pHYs` and `eXIf` chunks say: `pHYs` wins for the resolution.
pub(crate) fn from_png_chunks(phys: Option<&[u8]>, exif: Option<&[u8]>) -> Shown {
    let phys = phys.and_then(|c| {
        // x pixels per unit, y pixels per unit, unit (1: the metre).
        let [x0, x1, x2, x3, y0, y1, y2, y3, 1] = *c else {
            return None;
        };
        let (x, y) = (
            u32::from_be_bytes([x0, x1, x2, x3]),
            u32::from_be_bytes([y0, y1, y2, y3]),
        );
        (x > 0 && y > 0).then_some((x, y))
    });
    Shown {
        dots_per_metre: phys,
        ..Shown::default()
    }
    .or(exif.map(from_tiff).unwrap_or_default())
}

/// A TIFF structure's byte order.
#[derive(Clone, Copy)]
struct Tiff<'a> {
    data: &'a [u8],
    big_endian: bool,
}

impl Tiff<'_> {
    fn u16(self, at: usize) -> Option<u16> {
        let b = <[u8; 2]>::try_from(self.data.get(at..at.checked_add(2)?)?).ok()?;
        Some(if self.big_endian {
            u16::from_be_bytes(b)
        } else {
            u16::from_le_bytes(b)
        })
    }

    fn u32(self, at: usize) -> Option<u32> {
        let b = <[u8; 4]>::try_from(self.data.get(at..at.checked_add(4)?)?).ok()?;
        Some(if self.big_endian {
            u32::from_be_bytes(b)
        } else {
            u32::from_le_bytes(b)
        })
    }

    /// The RATIONAL a value field at `at` points to.
    fn rational(self, at: usize) -> Option<f64> {
        let offset = usize::try_from(self.u32(at)?).ok()?;
        let (n, d) = (self.u32(offset)?, self.u32(offset.checked_add(4)?)?);
        (d != 0).then(|| f64::from(n) / f64::from(d))
    }
}

/// TIFF field types (TIFF 6.0 §2): SHORT and RATIONAL, the only ones read.
const SHORT: u16 = 3;
const RATIONAL: u16 = 5;

/// IFD0's orientation and resolution in an EXIF (TIFF) structure.
fn from_tiff(data: &[u8]) -> Shown {
    let big_endian = match data.get(..4) {
        Some(b"MM\0*") => true,
        Some(b"II*\0") => false,
        _ => return Shown::default(),
    };
    let tiff = Tiff { data, big_endian };
    let Some(ifd) = tiff.u32(4).and_then(|o| usize::try_from(o).ok()) else {
        return Shown::default();
    };
    let count = tiff.u16(ifd).unwrap_or(0);
    let (mut orientation, mut x, mut y, mut per_cm) = (None, None, None, false);
    for i in 0..usize::from(count) {
        let Some(entry) = ifd.checked_add(2 + i * 12) else {
            break;
        };
        let (Some(tag), Some(kind)) = (tiff.u16(entry), tiff.u16(entry + 2)) else {
            break;
        };
        let value = entry + 8;
        match (tag, kind) {
            (0x0112, SHORT) => orientation = tiff.u16(value).and_then(Orientation::from_exif),
            (0x011A, RATIONAL) => x = tiff.rational(value),
            (0x011B, RATIONAL) => y = tiff.rational(value),
            (0x0128, SHORT) => per_cm = tiff.u16(value) == Some(3),
            _ => {}
        }
    }
    Shown {
        orientation: orientation.unwrap_or_default(),
        dots_per_metre: x.zip(y).and_then(|(x, y)| per_metre(x, y, per_cm)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A little- or big-endian EXIF block with IFD0's `Orientation` and, optionally, a resolution.
    fn exif(big_endian: bool, orientation: u16, dpi: Option<u32>) -> Vec<u8> {
        let u16b = |v: u16| if big_endian { v.to_be_bytes() } else { v.to_le_bytes() };
        let u32b = |v: u32| if big_endian { v.to_be_bytes() } else { v.to_le_bytes() };
        let mut out = if big_endian {
            b"MM\0*".to_vec()
        } else {
            b"II*\0".to_vec()
        };
        out.extend(u32b(8));
        let entries: u16 = if dpi.is_some() { 3 } else { 1 };
        out.extend(u16b(entries));
        let rationals = 8 + 2 + u32::from(entries) * 12 + 4;
        let mut entry = |tag: u16, kind: u16, value: [u8; 4]| {
            out.extend(u16b(tag));
            out.extend(u16b(kind));
            out.extend(u32b(1));
            out.extend(value);
        };
        let mut short = [0; 4];
        short[..2].copy_from_slice(&u16b(orientation));
        entry(0x0112, SHORT, short);
        if dpi.is_some() {
            entry(0x011A, RATIONAL, u32b(rationals));
            entry(0x011B, RATIONAL, u32b(rationals + 8));
        }
        out.extend(u32b(0));
        if let Some(dpi) = dpi {
            for _ in 0..2 {
                out.extend(u32b(dpi));
                out.extend(u32b(1));
            }
        }
        out
    }

    #[test]
    fn every_byte_order_reads_the_orientation_and_the_resolution() {
        for big_endian in [false, true] {
            let shown = from_tiff(&exif(big_endian, 6, Some(300)));
            assert_eq!(shown.orientation, Orientation::Clockwise);
            assert_eq!(shown.dots_per_metre, Some((11_811, 11_811)));
        }
    }

    #[test]
    fn a_value_outside_one_to_eight_reads_as_upright() {
        assert_eq!(from_tiff(&exif(false, 9, None)).orientation, Orientation::Upright);
    }

    #[test]
    fn truncated_or_foreign_data_reads_as_nothing() {
        let full = exif(true, 8, Some(72));
        for cut in 0..full.len() {
            let _ = from_tiff(&full[..cut]);
        }
        assert_eq!(from_tiff(b"not tiff"), Shown::default());
    }

    #[test]
    fn jfif_density_wins_over_the_exif_resolution() {
        let mut app1 = b"Exif\0\0".to_vec();
        app1.extend(exif(false, 3, Some(72)));
        let jfif = *b"JFIF\0\x01\x02\x01\x01\x2c\x01\x2c";
        let shown = from_jpeg_segments(Some(&jfif), Some(&app1));
        assert_eq!(shown.orientation, Orientation::UpsideDown);
        assert_eq!(shown.dots_per_metre, Some((11_811, 11_811)));
        let aspect_only = *b"JFIF\0\x01\x02\x00\x00\x01\x00\x01";
        assert_eq!(
            from_jpeg_segments(Some(&aspect_only), Some(&app1)).dots_per_metre,
            Some((2835, 2835))
        );
    }

    #[test]
    fn png_phys_in_metres_wins_over_exif() {
        let phys = [0, 0, 0x0B, 0x13, 0, 0, 0x0B, 0x13, 1];
        let shown = from_png_chunks(Some(&phys), Some(&exif(false, 8, Some(300))));
        assert_eq!(shown.orientation, Orientation::Anticlockwise);
        assert_eq!(shown.dots_per_metre, Some((2835, 2835)));
        assert_eq!(
            from_png_chunks(Some(&[0, 0, 0, 1, 0, 0, 0, 1, 0]), None).dots_per_metre,
            None
        );
    }

    #[test]
    fn each_orientation_maps_the_stored_corners_onto_the_shown_rectangle() {
        let (width, height) = (4.0, 2.0);
        let apply = |m: [f64; 6], (x, y): (f64, f64)| (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]);
        for value in 1..=8 {
            let orientation = Orientation::from_exif(value).unwrap();
            let m = orientation.matrix(width, height);
            let (shown_w, shown_h) = if orientation.swaps_sides() {
                (height, width)
            } else {
                (width, height)
            };
            for corner in [(0.0, 0.0), (width, 0.0), (0.0, height), (width, height)] {
                let (u, v) = apply(m, corner);
                assert!(
                    (0.0..=shown_w).contains(&u) && (0.0..=shown_h).contains(&v),
                    "{value}: {u},{v}"
                );
            }
        }
        // Orientation 6: the stored top-left corner is shown at the top right.
        assert_eq!(
            apply(Orientation::Clockwise.matrix(width, height), (0.0, 0.0)),
            (height, 0.0)
        );
    }
}
