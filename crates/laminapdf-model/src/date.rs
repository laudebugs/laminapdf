//! PDF dates (ISO 32000-2 §7.9.4): `D:YYYYMMDDHHmmSSOHH'mm`.

use std::fmt;
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
        let valid = (1..=12).contains(&month)
            && day >= 1
            && day <= days_in_month(year, month)
            && hour < 24
            && minute < 60
            && second < 60;
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
    fn leap_years_follow_the_gregorian_rule() {
        assert!(parse("D:20000229").is_some() && parse("D:19000229").is_none() && parse("D:20240229").is_some());
    }
}
