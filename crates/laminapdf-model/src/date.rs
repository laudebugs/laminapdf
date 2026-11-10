//! PDF dates (ISO 32000-2 §7.9.4): `D:YYYYMMDDHHmmSSOHH'mm`.

use std::fmt::{self, Write as _};
use std::str::FromStr;

/// A date as written in a PDF (§7.9.4), e.g. `D:20261028143000+01'00`.
///
/// Every field after the year is optional and defaults as the standard says (month and day
/// to 1, the time to 0); the UTC offset is `None` when the date doesn't give one ("the
/// relationship to UT is unknown").
///
/// Parsing is tolerant where real files commonly stray: the `D:` prefix may be missing,
/// surrounding whitespace is ignored, the apostrophes of the offset are optional, a trailing
/// apostrophe is accepted, and `Z` may be followed by `00'00'`. Out-of-range fields (month
/// 13, February 30, hour 24, …) are rejected.
///
/// Dates order by the instant they denote (local time minus the UTC offset); a date without
/// an offset is compared as if it were UTC. Two dates naming the same instant with different
/// offsets order by offset (unknown first), so the order agrees with `==`.
///
/// ```
/// use laminapdf_model::PdfDate;
///
/// let date: PdfDate = "D:20261028143000+01'00'".parse()?;
/// assert_eq!((date.year(), date.month(), date.day()), (2026, 10, 28));
/// assert_eq!(date.utc_offset_minutes(), Some(60));
/// assert_eq!(date.to_string(), "D:20261028143000+01'00'");
/// # Ok::<(), laminapdf_model::InvalidDate>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PdfDate {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    offset: Option<i16>,
}

/// The string is not a PDF date.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("not a PDF date")]
#[non_exhaustive]
pub struct InvalidDate;

impl PdfDate {
    /// The year (0–9999).
    #[must_use]
    pub const fn year(&self) -> u16 {
        self.year
    }

    /// The month (1–12).
    #[must_use]
    pub const fn month(&self) -> u8 {
        self.month
    }

    /// The day of the month (1–31).
    #[must_use]
    pub const fn day(&self) -> u8 {
        self.day
    }

    /// The hour (0–23).
    #[must_use]
    pub const fn hour(&self) -> u8 {
        self.hour
    }

    /// The minute (0–59).
    #[must_use]
    pub const fn minute(&self) -> u8 {
        self.minute
    }

    /// The second (0–59).
    #[must_use]
    pub const fn second(&self) -> u8 {
        self.second
    }

    /// The offset from UTC in minutes (`Some(0)` for `Z`), or `None` when unknown.
    #[must_use]
    pub const fn utc_offset_minutes(&self) -> Option<i16> {
        self.offset
    }
}

impl PdfDate {
    /// Seconds since 0000-03-01 UTC (proleptic Gregorian), treating an unknown offset as
    /// UTC. The instant and the offset together determine every field.
    fn instant(&self) -> i64 {
        // Howard Hinnant's days-from-civil, with the year starting in March.
        let (y, m) = if self.month <= 2 {
            (i64::from(self.year) - 1, i64::from(self.month) + 9)
        } else {
            (i64::from(self.year), i64::from(self.month) - 3)
        };
        let era = y.div_euclid(400);
        let year_of_era = y.rem_euclid(400);
        let day_of_year = (153 * m + 2) / 5 + i64::from(self.day) - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
        let days = era * 146_097 + day_of_era;
        let local = days * 86_400 + i64::from(self.hour) * 3_600 + i64::from(self.minute) * 60 + i64::from(self.second);
        local - i64::from(self.offset.unwrap_or(0)) * 60
    }
}

impl Ord for PdfDate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.instant(), self.offset).cmp(&(other.instant(), other.offset))
    }
}

impl PartialOrd for PdfDate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

const fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        4 | 6 | 9 | 11 => 30,
        2 if (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400) => 29,
        2 => 28,
        _ => 31,
    }
}

/// Reads `n` ASCII digits from the front of `s`.
fn digits(s: &mut &[u8], n: usize) -> Option<u16> {
    let (head, rest) = s.split_at_checked(n)?;
    if !head.iter().all(u8::is_ascii_digit) {
        return None;
    }
    *s = rest;
    Some(head.iter().fold(0u16, |acc, d| acc * 10 + u16::from(d - b'0')))
}

/// An optional two-digit field.
enum Field {
    /// Two digits.
    Value(u8),
    /// The input ends, or the UTC offset starts, here.
    Absent,
}

/// Reads an optional two-digit field; `None` when something else is there.
fn optional_pair(s: &mut &[u8]) -> Option<Field> {
    match s.first() {
        None | Some(b'Z' | b'+' | b'-') => Some(Field::Absent),
        Some(_) => digits(s, 2).and_then(|v| u8::try_from(v).ok()).map(Field::Value),
    }
}

impl FromStr for PdfDate {
    type Err = InvalidDate;

    fn from_str(s: &str) -> Result<Self, InvalidDate> {
        let s = s.trim();
        let mut rest = s.strip_prefix("D:").unwrap_or(s).as_bytes();
        let year = digits(&mut rest, 4).ok_or(InvalidDate)?;
        let mut fields = [1u8, 1, 0, 0, 0];
        for slot in &mut fields {
            match optional_pair(&mut rest).ok_or(InvalidDate)? {
                Field::Value(v) => *slot = v,
                Field::Absent => break,
            }
        }
        let [month, day, hour, minute, second] = fields;
        let offset = parse_offset(rest)?;
        Self::checked([month, day, hour, minute, second], year, offset)
    }
}

impl PdfDate {
    /// The date from its fields (`[month, day, hour, minute, second]`), when they name one.
    fn checked(fields: [u8; 5], year: u16, offset: Option<i16>) -> Result<Self, InvalidDate> {
        let [month, day, hour, minute, second] = fields;
        let valid = (1..=12).contains(&month)
            && day >= 1
            && day <= days_in_month(year, month)
            && hour < 24
            && minute < 60
            && second < 60
            && offset.is_none_or(|m| m.unsigned_abs() < 24 * 60);
        if !valid {
            return Err(InvalidDate);
        }
        Ok(Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
            offset,
        })
    }

    /// The date as XMP writes it (ISO 8601, ISO 16684-1 §8.2.1.1): `2026-10-28T14:30:00+01:00`,
    /// `…Z` for UTC, and no zone when the PDF date gives none.
    ///
    /// ```
    /// use laminapdf_model::PdfDate;
    ///
    /// let date: PdfDate = "D:20261028143000+01'00".parse()?;
    /// assert_eq!(date.to_iso8601(), "2026-10-28T14:30:00+01:00");
    /// # Ok::<(), laminapdf_model::InvalidDate>(())
    /// ```
    #[must_use]
    pub fn to_iso8601(&self) -> String {
        let mut s = format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        );
        match self.offset {
            None => {}
            Some(0) => s.push('Z'),
            Some(m) => {
                let sign = if m < 0 { '-' } else { '+' };
                let m = m.unsigned_abs();
                // Writing to a String can't fail.
                let _ = write!(s, "{sign}{:02}:{:02}", m / 60, m % 60);
            }
        }
        s
    }

    /// Reads an XMP date (ISO 8601 as ISO 16684-1 §8.2.1.1 restricts it): `YYYY`, `YYYY-MM`,
    /// `YYYY-MM-DD`, or a full date with a time, `Thh:mm`, then optionally `:ss` and after the
    /// seconds a fraction (dropped: a PDF date counts whole seconds), and after the time a zone
    /// (`Z`, `+hh:mm`, `-hh:mm`) or none. A zone or fraction without a time is refused.
    /// `-00:00` reads as UTC: RFC 3339 §4.3 gives it the UTC instant with the local offset
    /// unknown, and a PDF date can only say the first. Surrounding whitespace is ignored.
    ///
    /// ```
    /// use laminapdf_model::PdfDate;
    ///
    /// let date = PdfDate::from_iso8601("2026-10-28T14:30+01:00")?;
    /// assert_eq!(date, "D:20261028143000+01'00".parse()?);
    /// # Ok::<(), laminapdf_model::InvalidDate>(())
    /// ```
    ///
    /// # Errors
    /// [`InvalidDate`] when `s` is not such a date or names no real day or time.
    pub fn from_iso8601(s: &str) -> Result<Self, InvalidDate> {
        let mut rest = s.trim().as_bytes();
        let year = digits(&mut rest, 4).ok_or(InvalidDate)?;
        let mut fields = [1u8, 1, 0, 0, 0];
        let mut parts = 0;
        // Date parts after `-`, the time after `T`, its parts after `:`.
        for (slot, separator) in fields.iter_mut().zip(*b"--T::") {
            let Some(rest_after) = rest.strip_prefix(&[separator]) else {
                break;
            };
            rest = rest_after;
            *slot = digits(&mut rest, 2)
                .and_then(|v| u8::try_from(v).ok())
                .ok_or(InvalidDate)?;
            parts += 1;
        }
        // A time gives at least hours and minutes; a zone needs a time and a fraction seconds.
        if parts == 3 || (parts < 3 && !rest.is_empty()) {
            return Err(InvalidDate);
        }
        if let Some(fraction) = rest.strip_prefix(b".") {
            if parts < 5 {
                return Err(InvalidDate);
            }
            let digits_len = fraction.iter().take_while(|b| b.is_ascii_digit()).count();
            if digits_len == 0 {
                return Err(InvalidDate);
            }
            rest = fraction.get(digits_len..).unwrap_or_default();
        }
        let offset = match rest {
            b"" => None,
            b"Z" => Some(0),
            [sign @ (b'+' | b'-'), zone @ ..] => {
                let mut zone = zone;
                let hours = digits(&mut zone, 2).ok_or(InvalidDate)?;
                zone = zone.strip_prefix(b":").ok_or(InvalidDate)?;
                let minutes = digits(&mut zone, 2).ok_or(InvalidDate)?;
                if !zone.is_empty() || hours > 23 || minutes > 59 {
                    return Err(InvalidDate);
                }
                let total = i16::try_from(hours * 60 + minutes).map_err(|_| InvalidDate)?;
                Some(if *sign == b'-' { -total } else { total })
            }
            _ => return Err(InvalidDate),
        };
        Self::checked(fields, year, offset)
    }

    /// The date as ISO 32000-1 §7.9.4's grammar writes it: `D:20261028143000+01'00`, with no
    /// apostrophe after the minutes. [`Display`](fmt::Display) keeps the trailing apostrophe of
    /// Adobe's PDF Reference, which the files Lamina PDF already builds carry.
    ///
    /// ```
    /// use laminapdf_model::PdfDate;
    ///
    /// let date: PdfDate = "D:20261028143000+01'00'".parse()?;
    /// assert_eq!(date.to_pdf_string(), "D:20261028143000+01'00");
    /// # Ok::<(), laminapdf_model::InvalidDate>(())
    /// ```
    #[must_use]
    pub fn to_pdf_string(&self) -> String {
        let mut s = self.to_string();
        if self.offset.is_some_and(|m| m != 0) {
            s.pop();
        }
        s
    }
}

/// `Z`, `+HH`, `+HH'mm`, `+HH'mm'` or `+HHmm` (and `-`), or nothing.
fn parse_offset(mut rest: &[u8]) -> Result<Option<i16>, InvalidDate> {
    let Some((&sign, tail)) = rest.split_first() else {
        return Ok(None);
    };
    rest = tail;
    let sign: i16 = match sign {
        b'+' => 1,
        b'-' => -1,
        b'Z' => 0,
        _ => return Err(InvalidDate),
    };
    if sign == 0 && !matches!(rest, b"" | b"'" | b"00'00'" | b"00'00" | b"00") {
        return Err(InvalidDate);
    }
    if sign == 0 {
        return Ok(Some(0));
    }
    let hours = digits(&mut rest, 2).ok_or(InvalidDate)?;
    rest = rest.strip_prefix(b"'").unwrap_or(rest);
    let minutes = if rest.is_empty() {
        0
    } else {
        digits(&mut rest, 2).ok_or(InvalidDate)?
    };
    rest = rest.strip_prefix(b"'").unwrap_or(rest);
    if !rest.is_empty() || hours > 23 || minutes > 59 {
        return Err(InvalidDate);
    }
    let total = i16::try_from(hours * 60 + minutes).map_err(|_| InvalidDate)?;
    Ok(Some(sign * total))
}

impl fmt::Display for PdfDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "D:{:04}{:02}{:02}{:02}{:02}{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )?;
        match self.offset {
            None => Ok(()),
            Some(0) => f.write_str("Z"),
            Some(m) => {
                let sign = if m < 0 { '-' } else { '+' };
                let m = m.unsigned_abs();
                write!(f, "{sign}{:02}'{:02}'", m / 60, m % 60)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Option<PdfDate> {
        s.parse().ok()
    }

    #[test]
    fn a_full_date_reads_every_field() {
        let d = parse("D:19981223195200-08'00'").unwrap();
        assert_eq!(
            (
                d.year(),
                d.month(),
                d.day(),
                d.hour(),
                d.minute(),
                d.second(),
                d.utc_offset_minutes()
            ),
            (1998, 12, 23, 19, 52, 0, Some(-480))
        );
    }

    #[test]
    fn a_year_alone_defaults_the_rest() {
        assert_eq!(parse("D:2026").unwrap().to_string(), "D:20260101000000");
    }

    #[test]
    fn the_prefix_and_apostrophes_are_optional() {
        assert_eq!(parse("20261028143000+0130").unwrap().utc_offset_minutes(), Some(90));
    }

    #[test]
    fn z_may_carry_a_zero_offset() {
        assert_eq!(parse("D:20261028143000Z00'00'").unwrap().utc_offset_minutes(), Some(0));
    }

    #[test]
    fn hours_only_offset_is_accepted() {
        assert_eq!(parse("D:20261028143000+05").unwrap().utc_offset_minutes(), Some(300));
    }

    #[test]
    fn display_round_trips() {
        for s in [
            "D:20261028143000+01'00'",
            "D:20000229000000Z",
            "D:20261028143000-03'30'",
        ] {
            assert_eq!(parse(s).unwrap().to_string(), s);
        }
    }

    #[test]
    fn out_of_range_fields_are_rejected() {
        for s in [
            "D:20261328",
            "D:20230229",
            "D:2026102824",
            "D:202610281260",
            "D:20261028125960",
            "D:20261028125900+24'00'",
            "D:20261000",
        ] {
            assert_eq!(parse(s), None, "{s}");
        }
    }

    #[test]
    fn garbage_is_rejected() {
        for s in [
            "",
            "D:",
            "D:20x6",
            "yesterday",
            "D:2026102814300",
            "D:20261028143000+01'00'x",
            "D:2026Zjunk",
        ] {
            assert_eq!(parse(s), None, "{s}");
        }
    }

    #[test]
    fn dates_order_by_instant_not_by_local_fields() {
        // 12:00+02:00 is 10:00 UTC, before 11:00 UTC.
        assert!(parse("D:20261028120000+02'00'").unwrap() < parse("D:20261028110000Z").unwrap());
    }

    #[test]
    fn the_same_instant_in_two_offsets_orders_by_offset_and_is_not_equal() {
        let utc = parse("D:20261028100000Z").unwrap();
        let cet = parse("D:20261028110000+01'00'").unwrap();
        assert_eq!((utc.cmp(&cet), utc == cet), (std::cmp::Ordering::Less, false));
    }

    #[test]
    fn instants_count_days_across_months_and_leap_years() {
        let a = parse("D:20240228Z").unwrap();
        let b = parse("D:20240301Z").unwrap();
        assert_eq!(b.instant() - a.instant(), 2 * 86_400);
    }

    #[test]
    fn the_legacy_trailing_apostrophe_names_the_same_instant() {
        let strict = parse("D:20261028143000+01'00").unwrap();
        let legacy = parse("D:20261028143000+01'00'").unwrap();
        assert_eq!(
            (strict == legacy, strict.cmp(&legacy)),
            (true, std::cmp::Ordering::Equal)
        );
    }

    #[test]
    fn the_strict_form_has_no_apostrophe_after_the_minutes() {
        let written: Vec<_> = [
            "D:20261028143000+01'00'",
            "D:20261028143000Z",
            "D:20261028143000-03'30",
            "D:2026",
        ]
        .into_iter()
        .map(|s| parse(s).unwrap().to_pdf_string())
        .collect();
        assert_eq!(
            written,
            [
                "D:20261028143000+01'00",
                "D:20261028143000Z",
                "D:20261028143000-03'30",
                "D:20260101000000"
            ]
        );
    }

    #[test]
    fn iso_dates_carry_their_zone() {
        let d = |s: &str| parse(s).unwrap().to_iso8601();
        assert_eq!(
            [d("D:20261029120000Z"), d("D:20261029120000-05'30'"), d("D:2026")],
            [
                "2026-10-29T12:00:00Z",
                "2026-10-29T12:00:00-05:30",
                "2026-01-01T00:00:00"
            ]
        );
    }

    #[test]
    fn iso_dates_may_be_truncated_or_carry_a_fraction() {
        let iso = |s: &str| PdfDate::from_iso8601(s).ok().map(|d| d.to_string());
        assert_eq!(
            [
                iso("2026"),
                iso("2026-10"),
                iso("2026-10-28"),
                iso("2026-10-28T14:30Z"),
                iso("2026-10-28T14:30:15.250+01:00"),
                iso(" 2026-10-28T14:30:15-08:00 "),
            ],
            [
                Some("D:20260101000000".to_owned()),
                Some("D:20261001000000".to_owned()),
                Some("D:20261028000000".to_owned()),
                Some("D:20261028143000Z".to_owned()),
                Some("D:20261028143015+01'00'".to_owned()),
                Some("D:20261028143015-08'00'".to_owned()),
            ]
        );
    }

    #[test]
    fn iso_dates_round_trip_through_pdf_dates() {
        for s in [
            "2026-10-28T14:30:00+01:00",
            "2000-02-29T00:00:00Z",
            "1999-12-31T23:59:59",
        ] {
            assert_eq!(PdfDate::from_iso8601(s).unwrap().to_iso8601(), s);
        }
    }

    #[test]
    fn minus_zero_reads_as_utc() {
        let d = PdfDate::from_iso8601("2026-10-28T14:30-00:00").unwrap();
        assert_eq!(d.utc_offset_minutes(), Some(0));
    }

    #[test]
    fn malformed_iso_dates_are_rejected() {
        for s in [
            "",
            "26-10-28",
            "2026-13-01",
            "2026-02-30",
            "2026-10-28T24:00Z",
            "2026-10-28T14",
            "2026-10-28T14:30+0100",
            "2026-10-28T14:30:00.Z",
            "2026-10-28T14:30:00+01:00x",
            "2026/10/28",
            "2026-10-28Z",
            "2026-10Z",
            "2026-10.5",
            "2026-10-28+01:00",
            "2026-10-28T14:30.5Z",
        ] {
            assert!(PdfDate::from_iso8601(s).is_err(), "{s}");
        }
    }

    #[test]
    fn leap_years_follow_the_gregorian_rule() {
        assert!(parse("D:20000229").is_some() && parse("D:19000229").is_none() && parse("D:20240229").is_some());
    }
}
