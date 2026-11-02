//! [`Rotation`]: a rotation by a multiple of 90°, clockwise, as `/Rotate` (ISO 32000-2
//! §7.7.3.3) stores it and a viewer adds it.

use crate::Object;

/// A rotation by a multiple of 90°, clockwise when displayed: a page's `/Rotate`
/// (§7.7.3.3), or a view rotation added to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Rotation {
    /// No rotation.
    #[default]
    None,
    /// 90° clockwise.
    Clockwise90,
    /// 180°.
    Half,
    /// 270° clockwise (90° counter-clockwise).
    Clockwise270,
}

impl Rotation {
    /// The rotation in degrees: 0, 90, 180 or 270.
    #[must_use]
    pub const fn degrees(self) -> u16 {
        match self {
            Self::None => 0,
            Self::Clockwise90 => 90,
            Self::Half => 180,
            Self::Clockwise270 => 270,
        }
    }

    /// The rotation for `degrees` (any multiple of 90, negative included), else `Option::None`.
    #[must_use]
    pub const fn from_degrees(degrees: i64) -> Option<Self> {
        match degrees.rem_euclid(360) {
            0 => Some(Self::None),
            90 => Some(Self::Clockwise90),
            180 => Some(Self::Half),
            270 => Some(Self::Clockwise270),
            _ => None,
        }
    }

    /// The rotation for `degrees` written the canonical way, 0, 90, 180 or 270, else
    /// `Option::None`: the check for values callers pass in (op JSON, the bindings), where
    /// `-90` or `450` is a mistake rather than a file to tolerate.
    #[must_use]
    pub const fn from_canonical_degrees(degrees: u16) -> Option<Self> {
        match degrees {
            0 => Some(Self::None),
            90 => Some(Self::Clockwise90),
            180 => Some(Self::Half),
            270 => Some(Self::Clockwise270),
            _ => None,
        }
    }

    /// The rotation a resolved `/Rotate` value states: an integer or an integral real that is
    /// a multiple of 90 (`90` and `90.0` alike), else `Option::None`.
    #[must_use]
    pub fn from_object(value: &Object) -> Option<Self> {
        let degrees = match *value {
            Object::Integer(i) => i,
            #[expect(
                clippy::cast_possible_truncation,
                reason = "only integral values in i64 range get here"
            )]
            Object::Real(r) if r.fract() == 0.0 && r.abs() < 1e15 => r as i64,
            _ => return None,
        };
        Self::from_degrees(degrees)
    }

    /// Whether the rotation swaps width and height.
    #[must_use]
    pub const fn is_quarter_turn(self) -> bool {
        matches!(self, Self::Clockwise90 | Self::Clockwise270)
    }
}

#[cfg(test)]
mod tests {
    use super::Rotation;
    use crate::Object;

    #[test]
    fn from_degrees_should_accept_every_multiple_of_90() {
        let read: Vec<_> = [0, 90, -90, 450, 45].into_iter().map(Rotation::from_degrees).collect();
        assert_eq!(
            read,
            [
                Some(Rotation::None),
                Some(Rotation::Clockwise90),
                Some(Rotation::Clockwise270),
                Some(Rotation::Clockwise90),
                None
            ]
        );
    }

    #[test]
    fn from_object_should_read_integers_and_integral_reals_only() {
        let values = [
            Object::Integer(90),
            Object::Real(90.0),
            Object::Real(-180.0),
            Object::Real(90.5),
            Object::Real(1e300),
            Object::Name("Rotate".into()),
        ];
        let read: Vec<_> = values.iter().map(Rotation::from_object).collect();
        assert_eq!(
            read,
            [
                Some(Rotation::Clockwise90),
                Some(Rotation::Clockwise90),
                Some(Rotation::Half),
                None,
                None,
                None
            ]
        );
    }

    #[test]
    fn from_canonical_degrees_should_refuse_other_spellings() {
        let read: Vec<_> = [270, 450, 45]
            .into_iter()
            .map(Rotation::from_canonical_degrees)
            .collect();
        assert_eq!(read, [Some(Rotation::Clockwise270), None, None]);
    }

    #[test]
    fn degrees_should_round_trip_through_from_degrees() {
        let all = [
            Rotation::None,
            Rotation::Clockwise90,
            Rotation::Half,
            Rotation::Clockwise270,
        ];
        let back: Vec<_> = all
            .iter()
            .map(|r| Rotation::from_degrees(i64::from(r.degrees())))
            .collect();
        assert_eq!(back, all.map(Some));
    }
}
