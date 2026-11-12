//! The content-stream operators of ISO 32000-2 Annex A (Table A.1) and their operand
//! signatures.

use laminapdf_syntax::Object;

/// The kind of one operand in an operator's [`Signature`].
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Operand {
    /// An integer or real number.
    Number,
    /// A name.
    Name,
    /// A literal or hex string.
    String,
    /// An array.
    Array,
    /// A dictionary or a name (a property list given inline or by its resource name,
    /// §14.6.2).
    NameOrDict,
}

impl Operand {
    fn accepts(self, object: &Object) -> bool {
        match self {
            Self::Number => matches!(object, Object::Integer(_) | Object::Real(_)),
            Self::Name => matches!(object, Object::Name(_)),
            Self::String => matches!(object, Object::String(_)),
            Self::Array => matches!(object, Object::Array(_)),
            Self::NameOrDict => matches!(object, Object::Name(_) | Object::Dict(_)),
        }
    }
}

/// The operands an operator takes (ISO 32000-2 Annex A and the tables it refers to).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Signature {
    /// Exactly these operands, in order.
    Fixed(&'static [Operand]),
    /// One or more numbers: the colour components of `SC`/`sc` (§8.6.8, Table 73).
    Color,
    /// One or more numbers, optionally followed by a pattern name, or the name alone:
    /// `SCN`/`scn` (§8.6.8).
    ColorOrPattern,
}

impl Signature {
    /// The fewest operands the operator accepts.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn min(&self) -> usize {
        match self {
            Self::Fixed(kinds) => kinds.len(),
            Self::Color | Self::ColorOrPattern => 1,
        }
    }

    /// The most operands the operator accepts (`None`: no fixed maximum).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn max(&self) -> Option<usize> {
        match self {
            Self::Fixed(kinds) => Some(kinds.len()),
            Self::Color | Self::ColorOrPattern => None,
        }
    }

    /// The operands the operator uses out of `operands` (everything read since the previous
    /// operator), or `None` when they don't fit: too few, or of the wrong type.
    ///
    /// Operands are taken from the end, as a PostScript interpreter pops them, so extra
    /// operands in front are left out ([`Checked::extra`]).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn check<'o>(&self, operands: &'o [Object]) -> Option<Checked<'o>> {
        let used = match self {
            Self::Fixed(kinds) => {
                let start = operands.len().checked_sub(kinds.len())?;
                let used = operands.get(start..)?;
                if !kinds.iter().zip(used).all(|(kind, operand)| kind.accepts(operand)) {
                    return None;
                }
                used
            }
            Self::Color => number_suffix(operands),
            Self::ColorOrPattern => match operands.split_last() {
                Some((Object::Name(_), rest)) => {
                    let numbers = number_suffix(rest).len();
                    operands.get(rest.len() - numbers..)?
                }
                _ => number_suffix(operands),
            },
        };
        if used.is_empty() && self.min() > 0 {
            return None;
        }
        Some(Checked {
            operands: used,
            extra: used.len() < operands.len(),
        })
    }
}

/// The longest run of numbers at the end of `operands`.
fn number_suffix(operands: &[Object]) -> &[Object] {
    let numbers = operands.iter().rev().take_while(|o| Operand::Number.accepts(o)).count();
    operands.get(operands.len() - numbers..).unwrap_or_default()
}

/// The result of [`Signature::check`]: the operands an operator uses.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Checked<'o> {
    /// The operands the operator uses, in order.
    pub operands: &'o [Object],
    /// Whether operands were left over in front of them (`content-bad-operands`).
    pub extra: bool,
}

use Operand::{Array as A, Name as N, NameOrDict as ND, Number as F, String as S};

macro_rules! operators {
    ($( $(#[$doc:meta])* $variant:ident = $keyword:literal, $signature:expr; )*) => {
        /// A content-stream operator: one of the 73 of ISO 32000-2 Annex A (Table A.1), or
        /// an [`Unknown`](Operator::Unknown) keyword.
        ///
        /// `Copy`; an unknown operator borrows its keyword from the content bytes.
        ///
        /// Added in 0.2.0.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[non_exhaustive]
        pub enum Operator<'a> {
            $( $(#[$doc])* $variant, )*
            /// Any other keyword (skipped with `content-unknown-operator` unless inside a
            /// `BX`/`EX` compatibility section, §7.8.2).
            Unknown(&'a [u8]),
        }

        impl<'a> Operator<'a> {
            /// Every Annex A operator, in the order of Table A.1's categories.
            ///
            /// Added in 0.2.0.
            pub const ALL: &'static [Operator<'static>] = &[ $( Operator::$variant, )* ];

            /// The operator a keyword names ([`Operator::Unknown`] for any other keyword).
            ///
            /// Added in 0.2.0.
            #[must_use]
            pub fn from_keyword(keyword: &'a [u8]) -> Self {
                match keyword {
                    $( $keyword => Self::$variant, )*
                    other => Self::Unknown(other),
                }
            }

            /// The operator's keyword, e.g. `b"Tj"`.
            ///
            /// Added in 0.2.0.
            #[must_use]
            pub const fn keyword(&self) -> &'a [u8] {
                match self {
                    $( Self::$variant => $keyword, )*
                    Self::Unknown(keyword) => keyword,
                }
            }

            /// The operands the operator takes (none for [`Operator::Unknown`], whose
            /// operands are discarded).
            ///
            /// Added in 0.2.0.
            #[must_use]
            pub const fn signature(&self) -> Signature {
                match self {
                    $( Self::$variant => $signature, )*
                    Self::Unknown(_) => Signature::Fixed(&[]),
                }
            }
        }
    };
}

operators! {
    // General graphics state (§8.4.4, Table 56).
    /// `w`: line width.
    LineWidth = b"w", Signature::Fixed(&[F]);
    /// `J`: line cap style.
    LineCap = b"J", Signature::Fixed(&[F]);
    /// `j`: line join style.
    LineJoin = b"j", Signature::Fixed(&[F]);
    /// `M`: miter limit.
    MiterLimit = b"M", Signature::Fixed(&[F]);
    /// `d`: dash array and phase.
    Dash = b"d", Signature::Fixed(&[A, F]);
    /// `ri`: rendering intent.
    RenderingIntent = b"ri", Signature::Fixed(&[N]);
    /// `i`: flatness tolerance.
    Flatness = b"i", Signature::Fixed(&[F]);
    /// `gs`: apply a graphics state parameter dictionary (`/ExtGState` resource).
    ExtGState = b"gs", Signature::Fixed(&[N]);
    // Special graphics state (§8.4.4).
    /// `q`: save the graphics state.
    Save = b"q", Signature::Fixed(&[]);
    /// `Q`: restore the graphics state.
    Restore = b"Q", Signature::Fixed(&[]);
    /// `cm`: concatenate a matrix to the CTM.
    Transform = b"cm", Signature::Fixed(&[F, F, F, F, F, F]);
    // Path construction (§8.5.2, Table 58).
    /// `m`: begin a subpath.
    MoveTo = b"m", Signature::Fixed(&[F, F]);
    /// `l`: straight line segment.
    LineTo = b"l", Signature::Fixed(&[F, F]);
    /// `c`: cubic Bézier curve.
    CurveTo = b"c", Signature::Fixed(&[F, F, F, F, F, F]);
    /// `v`: cubic Bézier curve, first control point at the current point.
    CurveToV = b"v", Signature::Fixed(&[F, F, F, F]);
    /// `y`: cubic Bézier curve, second control point at the end point.
    CurveToY = b"y", Signature::Fixed(&[F, F, F, F]);
    /// `h`: close the subpath.
    ClosePath = b"h", Signature::Fixed(&[]);
    /// `re`: rectangle.
    Rectangle = b"re", Signature::Fixed(&[F, F, F, F]);
    // Path painting (§8.5.3, Table 59).
    /// `S`: stroke.
    Stroke = b"S", Signature::Fixed(&[]);
    /// `s`: close and stroke.
    CloseStroke = b"s", Signature::Fixed(&[]);
    /// `f`: fill, nonzero winding rule.
    Fill = b"f", Signature::Fixed(&[]);
    /// `F`: fill, nonzero winding rule (obsolete spelling of `f`).
    FillObsolete = b"F", Signature::Fixed(&[]);
    /// `f*`: fill, even-odd rule.
    FillEvenOdd = b"f*", Signature::Fixed(&[]);
    /// `B`: fill (nonzero) and stroke.
    FillStroke = b"B", Signature::Fixed(&[]);
    /// `B*`: fill (even-odd) and stroke.
    FillStrokeEvenOdd = b"B*", Signature::Fixed(&[]);
    /// `b`: close, fill (nonzero) and stroke.
    CloseFillStroke = b"b", Signature::Fixed(&[]);
    /// `b*`: close, fill (even-odd) and stroke.
    CloseFillStrokeEvenOdd = b"b*", Signature::Fixed(&[]);
    /// `n`: end the path without painting it.
    EndPath = b"n", Signature::Fixed(&[]);
    // Clipping paths (§8.5.4, Table 60).
    /// `W`: intersect the clip with the path, nonzero rule.
    Clip = b"W", Signature::Fixed(&[]);
    /// `W*`: intersect the clip with the path, even-odd rule.
    ClipEvenOdd = b"W*", Signature::Fixed(&[]);
    // Text objects (§9.4.1, Table 105).
    /// `BT`: begin a text object.
    BeginText = b"BT", Signature::Fixed(&[]);
    /// `ET`: end a text object.
    EndText = b"ET", Signature::Fixed(&[]);
    // Text state (§9.3.1, Table 103).
    /// `Tc`: character spacing.
    CharSpacing = b"Tc", Signature::Fixed(&[F]);
    /// `Tw`: word spacing.
    WordSpacing = b"Tw", Signature::Fixed(&[F]);
    /// `Tz`: horizontal scaling (percent).
    HorizontalScaling = b"Tz", Signature::Fixed(&[F]);
    /// `TL`: leading.
    Leading = b"TL", Signature::Fixed(&[F]);
    /// `Tf`: font and size.
    Font = b"Tf", Signature::Fixed(&[N, F]);
    /// `Tr`: text rendering mode.
    RenderMode = b"Tr", Signature::Fixed(&[F]);
    /// `Ts`: text rise.
    Rise = b"Ts", Signature::Fixed(&[F]);
    // Text positioning (§9.4.2, Table 106).
    /// `Td`: move to the start of the next line, offset from the current one.
    MoveText = b"Td", Signature::Fixed(&[F, F]);
    /// `TD`: as `Td`, also setting the leading to `-ty`.
    MoveTextSetLeading = b"TD", Signature::Fixed(&[F, F]);
    /// `Tm`: set the text matrix and text line matrix.
    TextMatrix = b"Tm", Signature::Fixed(&[F, F, F, F, F, F]);
    /// `T*`: move to the start of the next line.
    NextLine = b"T*", Signature::Fixed(&[]);
    // Text showing (§9.4.3, Table 107).
    /// `Tj`: show a string.
    ShowText = b"Tj", Signature::Fixed(&[S]);
    /// `TJ`: show strings with individual position adjustments.
    ShowTextAdjusted = b"TJ", Signature::Fixed(&[A]);
    /// `'`: move to the next line and show a string.
    NextLineShowText = b"'", Signature::Fixed(&[S]);
    /// `"`: set word and character spacing, move to the next line and show a string.
    SpacingNextLineShowText = b"\"", Signature::Fixed(&[F, F, S]);
    // Type 3 fonts (§9.6.4, Table 111).
    /// `d0`: glyph width (coloured glyph).
    GlyphWidth = b"d0", Signature::Fixed(&[F, F]);
    /// `d1`: glyph width and bounding box (uncoloured glyph).
    GlyphWidthBBox = b"d1", Signature::Fixed(&[F, F, F, F, F, F]);
    // Colour (§8.6.8, Table 73).
    /// `CS`: stroking colour space.
    StrokeColorSpace = b"CS", Signature::Fixed(&[N]);
    /// `cs`: nonstroking colour space.
    FillColorSpace = b"cs", Signature::Fixed(&[N]);
    /// `SC`: stroking colour.
    StrokeColor = b"SC", Signature::Color;
    /// `SCN`: stroking colour, including patterns, Separation, DeviceN and ICCBased.
    StrokeColorN = b"SCN", Signature::ColorOrPattern;
    /// `sc`: nonstroking colour.
    FillColor = b"sc", Signature::Color;
    /// `scn`: nonstroking colour, including patterns, Separation, DeviceN and ICCBased.
    FillColorN = b"scn", Signature::ColorOrPattern;
    /// `G`: DeviceGray stroking colour.
    StrokeGray = b"G", Signature::Fixed(&[F]);
    /// `g`: DeviceGray nonstroking colour.
    FillGray = b"g", Signature::Fixed(&[F]);
    /// `RG`: DeviceRGB stroking colour.
    StrokeRgb = b"RG", Signature::Fixed(&[F, F, F]);
    /// `rg`: DeviceRGB nonstroking colour.
    FillRgb = b"rg", Signature::Fixed(&[F, F, F]);
    /// `K`: DeviceCMYK stroking colour.
    StrokeCmyk = b"K", Signature::Fixed(&[F, F, F, F]);
    /// `k`: DeviceCMYK nonstroking colour.
    FillCmyk = b"k", Signature::Fixed(&[F, F, F, F]);
    // Shading (§8.7.4.2, Table 76).
    /// `sh`: paint a shading (`/Shading` resource) over the clip.
    Shading = b"sh", Signature::Fixed(&[N]);
    // Inline images (§8.9.7, Table 90).
    /// `BI`: begin an inline image. [`ContentIter`](crate::ContentIter) reads the whole
    /// `BI … ID … EI` sequence as this one operator, with its [`InlineImage`](crate::InlineImage).
    BeginInlineImage = b"BI", Signature::Fixed(&[]);
    /// `ID`: begin inline image data (only meaningful inside `BI … EI`).
    InlineImageData = b"ID", Signature::Fixed(&[]);
    /// `EI`: end an inline image (only meaningful inside `BI … EI`).
    EndInlineImage = b"EI", Signature::Fixed(&[]);
    // XObjects (§8.8, Table 86).
    /// `Do`: paint an XObject (`/XObject` resource).
    PaintXObject = b"Do", Signature::Fixed(&[N]);
    // Marked content (§14.6, Table 352).
    /// `MP`: marked-content point.
    MarkPoint = b"MP", Signature::Fixed(&[N]);
    /// `DP`: marked-content point with a property list.
    MarkPointProperties = b"DP", Signature::Fixed(&[N, ND]);
    /// `BMC`: begin a marked-content sequence.
    BeginMarked = b"BMC", Signature::Fixed(&[N]);
    /// `BDC`: begin a marked-content sequence with a property list.
    BeginMarkedProperties = b"BDC", Signature::Fixed(&[N, ND]);
    /// `EMC`: end a marked-content sequence.
    EndMarked = b"EMC", Signature::Fixed(&[]);
    // Compatibility (§7.8.2, Table 33).
    /// `BX`: begin a compatibility section (unknown operators are ignored silently).
    BeginCompatibility = b"BX", Signature::Fixed(&[]);
    /// `EX`: end a compatibility section.
    EndCompatibility = b"EX", Signature::Fixed(&[]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annex_a_defines_73_operators() {
        assert_eq!(Operator::ALL.len(), 73);
    }

    #[test]
    fn every_keyword_round_trips() {
        for op in Operator::ALL {
            assert_eq!(Operator::from_keyword(op.keyword()), *op);
        }
    }

    #[test]
    fn an_unknown_keyword_keeps_its_bytes() {
        assert_eq!(Operator::from_keyword(b"Xy").keyword(), b"Xy");
    }

    #[test]
    fn fixed_signatures_take_operands_from_the_end() {
        let operands = [Object::Integer(9), Object::Integer(1), Object::Integer(2)];
        let checked = Operator::MoveTo.signature().check(&operands);
        assert_eq!(
            checked,
            Some(Checked {
                operands: operands.get(1..).unwrap_or_default(),
                extra: true
            })
        );
    }

    #[test]
    fn a_wrong_operand_type_fails_the_check() {
        let operands = [Object::name("a"), Object::Integer(2)];
        assert_eq!(Operator::MoveTo.signature().check(&operands), None);
    }

    #[test]
    fn scn_accepts_a_pattern_name_after_components() {
        let operands = [Object::Real(0.5), Object::name("P1")];
        let checked = Operator::FillColorN.signature().check(&operands);
        assert_eq!(checked.map(|c| (c.operands.len(), c.extra)), Some((2, false)));
    }

    #[test]
    fn sc_rejects_a_name() {
        assert_eq!(Operator::FillColor.signature().check(&[Object::name("P1")]), None);
    }
}
