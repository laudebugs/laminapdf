//! Dates and times as `AFDate_FormatEx` and `AFTime_Format` read and write them: `util.scand`,
//! its fallback that takes the numbers in the text in the pattern's order, and `util.printd`.
//! pdf.js's `util.js` is the reading followed, with two changes so that the same input always
//! gives the same output: a date whose year isn't given takes 2000, not the current year, and a
//! pattern that prints the year then leaves the text as typed; and the fallback reads `h` hours
//! too (pdf.js's skips them, so `14:05` under `h:MM tt` reads as 12:14).

/// Month names, as `printd` writes them and `scand` reads them.
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// Day names, Sunday first.
const DAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

/// `printd`'s and `scand`'s pattern elements, longest first at each letter.
const ELEMENTS: [&str; 20] = [
    "mmmm", "mmm", "mm", "m", "dddd", "ddd", "dd", "d", "yyyy", "yy", "HH", "H", "hh", "h", "MM", "M", "ss", "s", "tt",
    "t",
];

/// A local date and time, normalised as JavaScript's `new Date(y, m, d, h, mi, s)` normalises
/// it (the 32nd of a month is the next month's 1st).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Civil {
    year: i64,
    /// 0 for January.
    month: i64,
    day: i64,
    hours: i64,
    minutes: i64,
    seconds: i64,
}

impl Civil {
    fn new(year: i64, month: i64, day: i64, hours: i64, minutes: i64, seconds: i64) -> Self {
        // JavaScript takes years 0 to 99 as 1900 to 1999.
        let year = if (0..100).contains(&year) { year + 1900 } else { year };
        let total = (days_from_civil(year + month.div_euclid(12), month.rem_euclid(12) + 1, 1) + day - 1) * 86_400
            + hours * 3600
            + minutes * 60
            + seconds;
        let (days, secs) = (total.div_euclid(86_400), total.rem_euclid(86_400));
        let (y, m, d) = civil_from_days(days);
        Self {
            year: y,
            month: m - 1,
            day: d,
            hours: secs / 3600,
            minutes: secs % 3600 / 60,
            seconds: secs % 60,
        }
    }

    /// 0 for Sunday.
    fn weekday(self) -> usize {
        let days = days_from_civil(self.year, self.month + 1, self.day);
        usize::try_from((days + 4).rem_euclid(7)).unwrap_or(0)
    }
}

/// Days from 1970-01-01 to the proleptic Gregorian date `y-m-d` (`m` from 1).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The date `days` after 1970-01-01, as (year, month from 1, day).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// The element of [`ELEMENTS`] that `pattern` starts with.
fn element(pattern: &str) -> Option<&'static str> {
    ELEMENTS.iter().copied().find(|e| pattern.starts_with(e))
}

/// `date` written by `pattern`, as `util.printd` writes it; `\x` writes `x`.
#[must_use]
pub(crate) fn printd(pattern: &str, date: Civil) -> String {
    let month = MONTHS
        .get(usize::try_from(date.month).unwrap_or(0))
        .copied()
        .unwrap_or("");
    let day = DAYS.get(date.weekday()).copied().unwrap_or("");
    let hour12 = 1 + (date.hours + 11) % 12;
    let mut out = String::new();
    let mut rest = pattern;
    while let Some(c) = rest.chars().next() {
        if let Some(e) = element(rest) {
            let written = match e {
                "mmmm" => month.to_owned(),
                "mmm" => month.chars().take(3).collect(),
                "mm" => format!("{:02}", date.month + 1),
                "m" => (date.month + 1).to_string(),
                "dddd" => day.to_owned(),
                "ddd" => day.chars().take(3).collect(),
                "dd" => format!("{:02}", date.day),
                "d" => date.day.to_string(),
                "yyyy" => format!("{:04}", date.year),
                "yy" => format!("{:02}", date.year.rem_euclid(100)),
                "HH" => format!("{:02}", date.hours),
                "H" => date.hours.to_string(),
                "hh" => format!("{hour12:02}"),
                "h" => hour12.to_string(),
                "MM" => format!("{:02}", date.minutes),
                "M" => date.minutes.to_string(),
                "ss" => format!("{:02}", date.seconds),
                "s" => date.seconds.to_string(),
                "tt" => (if date.hours < 12 { "am" } else { "pm" }).to_owned(),
                _ => (if date.hours < 12 { "a" } else { "p" }).to_owned(),
            };
            out.push_str(&written);
            rest = rest.get(e.len()..).unwrap_or("");
        } else if c == '\\' {
            let mut chars = rest.chars();
            chars.next();
            if let Some(escaped) = chars.next() {
                out.push(escaped);
            }
            rest = chars.as_str();
        } else {
            out.push(c);
            rest = rest.get(c.len_utf8()..).unwrap_or("");
        }
    }
    out
}

/// What `scand` has read so far.
#[derive(Debug, Clone, Copy)]
struct Read {
    year: i64,
    month: i64,
    day: i64,
    hours: i64,
    minutes: i64,
    seconds: i64,
    am: Option<bool>,
}

/// A pattern element or a literal character.
#[derive(Debug, Clone, Copy)]
enum Part {
    Element(&'static str),
    Literal(char),
}

fn parts(pattern: &str) -> Vec<Part> {
    let mut out = Vec::new();
    let mut rest = pattern;
    while let Some(c) = rest.chars().next() {
        if let Some(e) = element(rest) {
            out.push(Part::Element(e));
            rest = rest.get(e.len()..).unwrap_or("");
        } else {
            out.push(Part::Literal(c));
            rest = rest.get(c.len_utf8()..).unwrap_or("");
        }
    }
    out
}

/// `text` read whole by `pattern`, as `util.scand` reads it (its regular expression, as an
/// explicit search); `None` when it doesn't match.
fn scand_exact(pattern: &str, text: &str) -> Option<Civil> {
    let parts = parts(pattern);
    let mut read = Read {
        year: 2000,
        month: 0,
        day: 1,
        hours: 0,
        minutes: 0,
        seconds: 0,
        am: None,
    };
    let chars: Vec<char> = text.chars().collect();
    let mut failed = std::collections::BTreeSet::new();
    if !matches(&parts, &chars, 0, 0, &mut read, &mut failed) {
        return None;
    }
    if let Some(am) = read.am {
        read.hours = read.hours % 12 + if am { 0 } else { 12 };
    }
    Some(Civil::new(
        read.year,
        read.month,
        read.day,
        read.hours,
        read.minutes,
        read.seconds,
    ))
}

/// Whether `parts[p..]` matches `chars[c..]` whole, recording what it reads. `failed` holds the
/// positions already known not to match, so the search takes time in proportion to the
/// pattern times the text.
fn matches(
    parts: &[Part],
    chars: &[char],
    p: usize,
    c: usize,
    read: &mut Read,
    failed: &mut std::collections::BTreeSet<(usize, usize)>,
) -> bool {
    let Some(part) = parts.get(p) else {
        return c == chars.len();
    };
    if failed.contains(&(p, c)) {
        return false;
    }
    let rest: String = chars.get(c..).unwrap_or_default().iter().collect();
    for (len, apply) in candidates(*part, &rest) {
        let saved = *read;
        apply(read);
        if matches(parts, chars, p + 1, c + len, read, failed) {
            return true;
        }
        *read = saved;
    }
    failed.insert((p, c));
    false
}

/// A way to read a part at the start of `rest`: the characters it takes and what it records.
type Candidate = (usize, Box<dyn Fn(&mut Read)>);

fn candidates(part: Part, rest: &str) -> Vec<Candidate> {
    let digits = |min: usize, max: usize| -> Vec<(usize, i64)> {
        let run = rest.chars().take_while(char::is_ascii_digit).take(max).count();
        (min..=run.min(max))
            .rev()
            .filter_map(|n| rest.get(..n)?.parse().ok().map(|v| (n, v)))
            .collect()
    };
    let named = |names: &[&str], len: Option<usize>| -> Vec<(usize, i64)> {
        names
            .iter()
            .enumerate()
            .filter_map(|(i, name)| {
                let name: String = len.map_or_else(|| (*name).to_owned(), |l| name.chars().take(l).collect());
                rest.starts_with(&name)
                    .then(|| (name.len(), i64::try_from(i).unwrap_or(0)))
            })
            .collect()
    };
    let set = |values: Vec<(usize, i64)>, f: fn(&mut Read, i64)| -> Vec<Candidate> {
        values
            .into_iter()
            .map(|(len, v)| (len, Box::new(move |r: &mut Read| f(r, v)) as Box<dyn Fn(&mut Read)>))
            .collect()
    };
    match part {
        Part::Literal(ch) => {
            if rest.starts_with(ch) {
                vec![(ch.len_utf8(), Box::new(|_: &mut Read| {}))]
            } else {
                Vec::new()
            }
        }
        Part::Element(e) => match e {
            "mmmm" => set(named(&MONTHS, None), |r, v| r.month = v),
            "mmm" => set(named(&MONTHS, Some(3)), |r, v| r.month = v),
            "mm" => set(digits(2, 2), |r, v| r.month = v - 1),
            "m" => set(digits(1, 2), |r, v| r.month = v - 1),
            // pdf.js records a day name's index as the day of the month.
            "dddd" => set(named(&DAYS, None), |r, v| r.day = v),
            "ddd" => set(named(&DAYS, Some(3)), |r, v| r.day = v),
            "dd" => set(digits(2, 2), |r, v| r.day = v),
            "d" => set(digits(1, 2), |r, v| r.day = v),
            "yyyy" => set(digits(4, 4), |r, v| r.year = v),
            "yy" => set(digits(2, 2), |r, v| r.year = 2000 + v),
            "HH" | "hh" => set(digits(2, 2), |r, v| r.hours = v),
            "H" | "h" => set(digits(1, 2), |r, v| r.hours = v),
            "MM" => set(digits(2, 2), |r, v| r.minutes = v),
            "M" => set(digits(1, 2), |r, v| r.minutes = v),
            "ss" => set(digits(2, 2), |r, v| r.seconds = v),
            "s" => set(digits(1, 2), |r, v| r.seconds = v),
            "tt" => {
                let mut c = rest.chars();
                match (c.next(), c.next()) {
                    (Some(a @ ('a' | 'A' | 'p' | 'P')), Some('m' | 'M')) => {
                        let am = a == 'a' || a == 'A';
                        vec![(2, Box::new(move |r: &mut Read| r.am = Some(am)))]
                    }
                    _ => Vec::new(),
                }
            }
            _ => match rest.chars().next() {
                Some(a @ ('a' | 'A' | 'p' | 'P')) => {
                    let am = a == 'a' || a == 'A';
                    vec![(1, Box::new(move |r: &mut Read| r.am = Some(am)))]
                }
                _ => Vec::new(),
            },
        },
    }
}

/// pdf.js's fallback when `scand` doesn't match: the text's numbers, in order, taken as the
/// pattern's day, month, year, hour, minute and second runs, in the order the pattern has them.
fn guess(pattern: &str, text: &str) -> Option<Civil> {
    let mut fields = Vec::new();
    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        if matches!(c, 'd' | 'm' | 'y' | 'H' | 'h' | 'M' | 's') {
            while chars.peek() == Some(&c) {
                chars.next();
            }
            fields.push(c);
        }
    }
    let mut year = None;
    let (mut month, mut day, mut hours, mut minutes, mut seconds) = (0, 1, 12, 0, 0);
    let mut read = 0;
    let numbers = text
        .split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .map(|s| s.parse::<i64>().unwrap_or(i64::MAX));
    for (field, n) in fields.iter().zip(numbers) {
        let ok = match field {
            'd' => (1..=31).contains(&n).then(|| day = n),
            'm' => (1..=12).contains(&n).then(|| month = n - 1),
            'y' => {
                year = Some(match n {
                    0..50 => n + 2000,
                    50..100 => n + 1900,
                    _ => n,
                });
                Some(())
            }
            'H' | 'h' => (0..=23).contains(&n).then(|| hours = n),
            'M' => (0..=59).contains(&n).then(|| minutes = n),
            _ => (0..=59).contains(&n).then(|| seconds = n),
        };
        ok?;
        read += 1;
    }
    if read == 0 {
        return None;
    }
    let year = match year {
        Some(y) => y,
        // A year the text doesn't give: one the output would show can't be known.
        None if pattern.contains('y') || pattern.contains("ddd") => return None,
        None => 2000,
    };
    Some(Civil::new(year, month, day, hours, minutes, seconds))
}

/// An ISO 8601 date, `2026-11-18`, with an optional `T14:05` or `T14:05:30` (the forms of
/// JavaScript's `Date.parse` that forms meet), read as local time.
fn iso(text: &str) -> Option<Civil> {
    let (date, time) = text.split_once('T').map_or((text, None), |(d, t)| (d, Some(t)));
    let mut parts = date.splitn(3, '-').map(|p| p.parse::<i64>().ok());
    let (Some(Some(year)), Some(Some(month)), Some(Some(day))) = (parts.next(), parts.next(), parts.next()) else {
        return None;
    };
    if date.len() != 10 || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let (mut hours, mut minutes, mut seconds) = (0, 0, 0);
    if let Some(time) = time {
        let mut clock = time.split(':').map(|p| p.parse::<i64>().ok());
        hours = clock.next()??;
        minutes = clock.next()??;
        seconds = clock.next().unwrap_or(Some(0))?;
    }
    Some(Civil::new(year, month - 1, day, hours, minutes, seconds))
}

/// The date `text` holds under `pattern`: `scand`, then the numbers in order, then ISO 8601.
#[must_use]
pub(crate) fn parse(pattern: &str, text: &str) -> Option<Civil> {
    scand_exact(pattern, text)
        .or_else(|| guess(pattern, text))
        .or_else(|| iso(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn show(pattern: &str, text: &str) -> Option<String> {
        parse(pattern, text).map(|d| printd(pattern, d))
    }

    #[test]
    fn pdf_js_reads_and_writes_these_dates_so() {
        assert_eq!(
            [
                show("mmmm d, yyyy", "11/18/2026"),
                show("dd/mm/yy", "3/7/2026"),
                show("m/d/yy", "7/4/1976"),
                show("mmmm d, yyyy", "1/2/2027"),
                show("mmm d, yyyy", "Nov 18, 2026"),
            ],
            [
                Some("November 18, 2026".into()),
                Some("03/07/26".into()),
                Some("7/4/76".into()),
                Some("January 2, 2027".into()),
                Some("Nov 18, 2026".into()),
            ]
        );
    }

    #[test]
    fn a_twelve_hour_time_reads_its_hours() {
        assert_eq!(
            [
                show("h:MM tt", "14:05"),
                show("h:MM tt", "2:05 pm"),
                show("HH:MM", "9:30")
            ],
            [Some("2:05 pm".into()), Some("2:05 pm".into()), Some("09:30".into())]
        );
    }

    #[test]
    fn a_date_without_its_year_is_not_shown_when_the_pattern_prints_one() {
        assert_eq!([show("m/d/yy", "7/4"), show("m/d", "7/4")], [None, Some("7/4".into())]);
    }

    #[test]
    fn overflowing_days_roll_into_the_next_month_and_iso_dates_read() {
        assert_eq!(
            [show("yyyy-mm-dd", "2026-02-31"), show("d mmmm yyyy", "2026-11-18")],
            [Some("2026-03-03".into()), Some("18 November 2026".into())]
        );
    }

    #[test]
    fn weekdays_and_escapes_print() {
        let date = Civil::new(2026, 10, 18, 0, 0, 0);
        assert_eq!(printd("dddd \\mmm", date), "Wednesday m11");
    }
}
