//! The keys a `FreeText` annotation keeps its text style in (ISO 32000-2 §12.5.6.6, §12.7.4.3,
//! §12.7.4.4): `/DA`, the default appearance (`/Helv 12 Tf 0 g`); `/DS`, the default style as
//! CSS (`font: Helvetica 12pt; text-align:left; color:#000000`); and `/RC`, the rich text as
//! XHTML. Lamina PDF writes `/RC` only for bold and italic and reads it wider: `b`, `i`, `span`
//! and `p`, with `font-weight`, `font-style`, `font-size`, `color`, `font-family` and
//! `text-align` (E4 plan §11). Whatever it reads but can't keep marks the box simplified.

use laminapdf_cos::Dict;
use laminapdf_syntax::format_real;

use super::freetext::{AnnotFont, FreeText, RichText, StandardFamily, TextAlign, TextSpan};
use super::geom::AnnotColor;
use laminapdf_cos::{Object, PdfString};
use laminapdf_syntax::text::encode_text_string;

/// How deep `/RC`'s elements nest before the rest of the text is read flat.
const MAX_DEPTH: usize = 64;

/// Text style values read from `/DA`, `/DS` or `/RC`'s body: what the box takes when its own
/// keys don't say.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Defaults {
    pub(crate) family: Option<StandardFamily>,
    pub(crate) size: Option<f64>,
    pub(crate) color: Option<AnnotColor>,
    pub(crate) align: Option<TextAlign>,
    /// `/DA` names a font Lamina PDF can't map to a standard family.
    pub(crate) unknown_font: bool,
    /// The font resource `/DA` names (`Helv` for `/Helv 12 Tf`).
    pub(crate) name: Option<String>,
}

/// `/DA` and `/DS` for `text` drawn in `color` (black when absent).
pub(crate) fn write_da(dict: &mut Dict, text: &FreeText, color: Option<AnnotColor>) {
    let family = family_of(text.font);
    let resource = match family {
        StandardFamily::Helvetica => "Helv",
        StandardFamily::Times => "TiRo",
        StandardFamily::Courier => "Cour",
    };
    let color = color.unwrap_or(AnnotColor::Gray(0.0));
    let da = format!("/{resource} {} Tf {}", format_real(text.size), color_operator(color));
    dict.insert("DA", Object::String(PdfString::literal(da.into_bytes())));
    let ds = format!(
        "font: {} {}pt; text-align:{}; color:{}",
        css_family(family),
        format_real(text.size),
        text.align.css(),
        hex(color)
    );
    dict.insert("DS", Object::String(PdfString::literal(encode_text_string(&ds))));
}

/// The family a box's keys name: its own, or Helvetica for a font given by its bytes, which no
/// other reader can name.
const fn family_of(font: AnnotFont) -> StandardFamily {
    match font {
        AnnotFont::Standard(family) => family,
        AnnotFont::Font(_) => StandardFamily::Helvetica,
    }
}

const fn css_family(family: StandardFamily) -> &'static str {
    match family {
        StandardFamily::Helvetica => "Helvetica,sans-serif",
        StandardFamily::Times => "Times,serif",
        StandardFamily::Courier => "Courier,monospace",
    }
}

/// The operator setting `color` as the fill: `g`, `rg` or `k`.
fn color_operator(color: AnnotColor) -> String {
    let parts: Vec<String> = color.components().iter().map(|v| format_real(*v)).collect();
    let op = match parts.len() {
        1 => "g",
        3 => "rg",
        _ => "k",
    };
    format!("{} {op}", parts.join(" "))
}

/// `color` as CSS's `#rrggbb`, CMYK converted the simple way.
fn hex(color: AnnotColor) -> String {
    let (r, g, b) = match color.components().as_slice() {
        [g] => (*g, *g, *g),
        [r, g, b] => (*r, *g, *b),
        [c, m, y, k] => ((1.0 - c) * (1.0 - k), (1.0 - m) * (1.0 - k), (1.0 - y) * (1.0 - k)),
        _ => (0.0, 0.0, 0.0),
    };
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 0–1, then scaled to a byte"
    )]
    let byte = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02X}{:02X}{:02X}", byte(r), byte(g), byte(b))
}

/// `/DA`'s font, size and fill colour: `Tf`'s operands and the last `g`, `rg` or `k`.
pub(crate) fn parse_da(da: &str) -> Defaults {
    let mut out = Defaults::default();
    let mut operands: Vec<&str> = Vec::new();
    for token in da.split_ascii_whitespace() {
        let number = |i: usize| -> Option<f64> {
            operands
                .len()
                .checked_sub(i)
                .and_then(|at| operands.get(at))
                .and_then(|t| t.parse::<f64>().ok())
                .filter(|v| v.is_finite())
        };
        match token {
            "Tf" => {
                // Preview writes `//Helvetica`: the name's slashes, however many.
                if let Some(name) = operands
                    .iter()
                    .rev()
                    .find(|t| t.starts_with('/'))
                    .map(|t| t.trim_start_matches('/'))
                {
                    out.family = family_of_name(name);
                    out.unknown_font = out.family.is_none();
                    out.name = Some(name.to_owned());
                }
                // 0 is "auto size", which a box drawn once can't follow.
                out.size = number(1).filter(|s| *s > 0.0);
            }
            "g" | "rg" | "k" => {
                let n = match token {
                    "g" => 1,
                    "rg" => 3,
                    _ => 4,
                };
                let values: Option<Vec<f64>> = (1..=n).rev().map(number).collect();
                out.color = values.and_then(|v| AnnotColor::from_components(&v));
            }
            // Any other operator (`Tc`, `Tw`, `G`, …) ends its operands; numbers and names are
            // operands.
            _ if token.starts_with(|c: char| c.is_ascii_alphabetic() || c == '\'' || c == '"') => {}
            _ => {
                operands.push(token);
                continue;
            }
        }
        operands.clear();
    }
    out
}

/// The standard family a font name or CSS family list stands for: Helvetica and Arial, Times,
/// Courier, and CSS's generic families.
pub(crate) fn family_of_name(name: &str) -> Option<StandardFamily> {
    let first = name
        .split(',')
        .next()
        .unwrap_or_default()
        .trim()
        .trim_matches(['"', '\'']);
    let lower = first.to_ascii_lowercase();
    if lower == "helv" || lower.starts_with("helvetica") || lower.starts_with("arial") || lower == "sans-serif" {
        Some(StandardFamily::Helvetica)
    } else if lower == "tiro" || lower.starts_with("times") || lower == "serif" {
        Some(StandardFamily::Times)
    } else if lower == "cour" || lower.starts_with("courier") || lower == "monospace" {
        Some(StandardFamily::Courier)
    } else {
        None
    }
}

/// CSS declarations (`/DS`, a `style` attribute): family, size, colour and alignment, each when
/// it says one this reader knows.
pub(crate) fn parse_css(css: &str) -> Defaults {
    let mut out = Defaults::default();
    for (name, value) in declarations(css) {
        match name.as_str() {
            "font" => {
                for token in value.split_ascii_whitespace() {
                    if let Some(size) = css_size(token) {
                        out.size = Some(size);
                    } else if let Some(family) = family_of_name(token) {
                        out.family.get_or_insert(family);
                    }
                }
            }
            "font-family" => out.family = family_of_name(&value).or(out.family),
            "font-size" => out.size = css_size(&value).or(out.size),
            "color" => out.color = css_color(&value).or(out.color),
            "text-align" => {
                out.align = match value.as_str() {
                    "center" => Some(TextAlign::Center),
                    "right" | "end" => Some(TextAlign::Right),
                    "left" | "start" | "justify" => Some(TextAlign::Left),
                    _ => out.align,
                }
            }
            _ => {}
        }
    }
    out
}

/// `name: value` pairs of a declaration list, names lower-cased, values trimmed.
fn declarations(css: &str) -> impl Iterator<Item = (String, String)> + '_ {
    css.split(';').filter_map(|d| {
        let (name, value) = d.split_once(':')?;
        Some((name.trim().to_ascii_lowercase(), value.trim().to_owned()))
    })
}

/// A size in points from `12pt`, `16px` (at 96 to the inch) or a bare number.
fn css_size(value: &str) -> Option<f64> {
    let value = value.trim();
    let (number, scale) = if let Some(v) = value.strip_suffix("pt") {
        (v, 1.0)
    } else if let Some(v) = value.strip_suffix("px") {
        (v, 0.75)
    } else {
        (value, 1.0)
    };
    let size = number.trim().parse::<f64>().ok()? * scale;
    (size.is_finite() && size > 0.0).then_some(size)
}

/// A colour from `#rgb`, `#rrggbb` or `rgb(r, g, b)` (0–255 or percentages).
fn css_color(value: &str) -> Option<AnnotColor> {
    let value = value.trim();
    if let Some(hex) = value.strip_prefix('#') {
        let digits: Vec<u8> = hex
            .chars()
            .map(|c| c.to_digit(16).and_then(|d| u8::try_from(d).ok()))
            .collect::<Option<_>>()?;
        let bytes = match digits.as_slice() {
            [r, g, b] => [r * 17, g * 17, b * 17],
            [r1, r2, g1, g2, b1, b2] => [r1 * 16 + r2, g1 * 16 + g2, b1 * 16 + b2],
            _ => return None,
        };
        let [r, g, b] = bytes.map(|v| f64::from(v) / 255.0);
        return Some(AnnotColor::Rgb(r, g, b));
    }
    let inner = value.strip_prefix("rgb(")?.strip_suffix(')')?;
    let parts: Vec<f64> = inner
        .split(',')
        .map(|p| {
            let p = p.trim();
            match p.strip_suffix('%') {
                Some(pct) => pct.trim().parse::<f64>().ok().map(|v| v / 100.0),
                None => p.parse::<f64>().ok().map(|v| v / 255.0),
            }
        })
        .collect::<Option<_>>()?;
    match parts.as_slice() {
        [r, g, b] if [r, g, b].iter().all(|v| v.is_finite()) => {
            Some(AnnotColor::Rgb(r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0)))
        }
        _ => None,
    }
}

/// `/RC` for `text` drawn in `color`: an XHTML body styled with the box's font, size, colour and
/// alignment, a `p` per paragraph and a `span` per bold or italic span.
pub(crate) fn write_rc(text: &FreeText, color: Option<AnnotColor>) -> String {
    let family = family_of(text.font);
    let style = format!(
        "font-family:{};font-size:{}pt;text-align:{};color:{}",
        css_family(family),
        format_real(text.size),
        text.align.css(),
        hex(color.unwrap_or(AnnotColor::Gray(0.0)))
    );
    write_body(&text.text, Some(&style))
}

/// The XHTML document of `text`: a body (with `style` when given), a `p` per paragraph and a
/// `span` per bold or italic span.
pub(crate) fn write_body(text: &RichText, style: Option<&str>) -> String {
    let mut out = String::from("<?xml version=\"1.0\"?><body xmlns=\"http://www.w3.org/1999/xhtml\"");
    if let Some(style) = style {
        out.push_str(" style=\"");
        out.push_str(style);
        out.push('"');
    }
    out.push('>');
    for paragraph in &text.paragraphs {
        out.push_str("<p>");
        for span in paragraph {
            let style = match (span.bold, span.italic) {
                (false, false) => None,
                (true, false) => Some("font-weight:bold"),
                (false, true) => Some("font-style:italic"),
                (true, true) => Some("font-weight:bold;font-style:italic"),
            };
            match style {
                Some(style) => {
                    out.push_str("<span style=\"");
                    out.push_str(style);
                    out.push_str("\">");
                    escape_into(&mut out, &span.text);
                    out.push_str("</span>");
                }
                None => escape_into(&mut out, &span.text),
            }
        }
        out.push_str("</p>");
    }
    out.push_str("</body>");
    out
}

/// `text` with XML's five special characters escaped.
fn escape_into(out: &mut String, text: &str) {
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
}

/// What reading `/RC` gives: the text, the body's style (the box's defaults) and whether it held
/// something the subset can't keep.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ReadRc {
    pub(crate) text: RichText,
    pub(crate) body: Defaults,
    pub(crate) simplified: bool,
}

/// The style in force at a point in `/RC`: what spans keep, and what the box keeps once.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Style {
    bold: bool,
    italic: bool,
    family: Option<StandardFamily>,
    size: Option<f64>,
    color: Option<AnnotColor>,
    align: Option<TextAlign>,
}

/// The one value `values` share, and whether they disagree.
fn uniform<T: PartialEq + Copy>(values: impl Iterator<Item = Option<T>>) -> (Option<T>, bool) {
    let mut first: Option<Option<T>> = None;
    let mut differ = false;
    for v in values {
        match first {
            None => first = Some(v),
            Some(f) => differ |= f != v,
        }
    }
    (first.flatten(), differ)
}

/// Sets `style`'s weight and slant from `css`; true when `css` asks for something else the subset
/// can't draw (a stretch, decoration, raise or letter spacing).
fn emphasis(css: &str, style: &mut Style) -> bool {
    let mut simplified = false;
    for (prop, value) in declarations(css) {
        match prop.as_str() {
            "font-weight" => {
                style.bold = value == "bold" || value == "bolder" || value.parse::<u32>().is_ok_and(|w| w >= 600);
            }
            "font-style" => style.italic = value == "italic" || value == "oblique",
            "font-stretch" | "text-decoration" | "vertical-align" | "letter-spacing"
                if !value.is_empty() && !value.eq_ignore_ascii_case("normal") && value != "none" =>
            {
                simplified = true;
            }
            _ => {}
        }
    }
    simplified
}

/// Reads `/RC`'s XHTML, tolerating what Acrobat and others write: any prolog, namespaces and
/// attributes, unknown elements (read as spans), missing end tags and unescaped text. A `p` or
/// `br` starts a paragraph. It never fails: text it can't parse is read as text.
pub(crate) fn read_rc(rc: &str) -> ReadRc {
    let mut out = ReadRc::default();
    let mut stack: Vec<(String, Style)> = Vec::new();
    let mut paragraph: Vec<TextSpan> = Vec::new();
    // The style of every stretch of text that draws something.
    let mut seen: Vec<Style> = Vec::new();
    let mut in_body = false;
    let mut rest = rc;
    while !rest.is_empty() {
        let style = stack.last().map(|(_, s)| *s).unwrap_or_default();
        let Some(open) = rest.find('<') else {
            push_text(&mut paragraph, &mut seen, &unescape(rest), style);
            break;
        };
        let (text, tail) = rest.split_at(open);
        push_text(&mut paragraph, &mut seen, &unescape(text), style);
        let Some(close) = tail.find('>') else {
            push_text(&mut paragraph, &mut seen, &unescape(tail), style);
            break;
        };
        let (tag, after) = tail.split_at(close + 1);
        rest = after;
        let inner = tag.trim_start_matches('<').trim_end_matches('>');
        if inner.starts_with('?') || inner.starts_with('!') {
            continue;
        }
        if let Some(name) = inner.strip_prefix('/') {
            let name = local_name(name);
            if let Some(at) = stack.iter().rposition(|(n, _)| *n == name) {
                stack.truncate(at);
            }
            if name == "p" {
                end_paragraph(&mut out.text, &mut paragraph);
            }
            continue;
        }
        let self_closing = inner.ends_with('/');
        let inner = inner.trim_end_matches('/');
        let name = local_name(inner.split_ascii_whitespace().next().unwrap_or_default());
        let css = attribute(inner, "style").unwrap_or_default();
        let declared = parse_css(&css);
        let mut next = Style {
            family: declared.family.or(style.family),
            size: declared.size.or(style.size),
            color: declared.color.or(style.color),
            align: declared.align.or(style.align),
            ..style
        };
        out.simplified |= emphasis(&css, &mut next);
        match name.as_str() {
            "body" => in_body = true,
            "b" | "strong" => next.bold = true,
            "i" | "em" => next.italic = true,
            "p" | "div" => {
                // Text before the paragraph, outside any `p`, is a paragraph of its own; the
                // white space between paragraphs is the XHTML's layout.
                if has_text(&paragraph) {
                    end_paragraph(&mut out.text, &mut paragraph);
                } else {
                    paragraph.clear();
                }
            }
            "br" => {
                end_paragraph(&mut out.text, &mut paragraph);
                continue;
            }
            "span" | "html" | "head" => {}
            _ => out.simplified |= in_body,
        }
        if !self_closing && stack.len() < MAX_DEPTH {
            stack.push((name, next));
        }
    }
    if has_text(&paragraph) {
        end_paragraph(&mut out.text, &mut paragraph);
    }
    if out.text.paragraphs.is_empty() {
        out.text.paragraphs.push(Vec::new());
    }
    // A size, colour, font or alignment all the text shares is the box's, however the XHTML
    // nests it; one that changes inside the text is what the subset can't keep.
    let (family, f) = uniform(seen.iter().map(|s| s.family));
    let (size, z) = uniform(seen.iter().map(|s| s.size));
    let (color, c) = uniform(seen.iter().map(|s| s.color));
    let (align, a) = uniform(seen.iter().map(|s| s.align));
    out.simplified |= f || z || c || a;
    out.body = Defaults {
        family,
        size,
        color,
        align,
        unknown_font: false,
        name: None,
    };
    out
}

/// Text read at the current style, merged into the last span when that has the same style;
/// its style is kept in `seen` when it draws something. Line breaks inside the XHTML's source
/// are white space, as XHTML reads them.
fn push_text(paragraph: &mut Vec<TextSpan>, seen: &mut Vec<Style>, text: &str, style: Style) {
    let text = text.replace(['\r', '\n'], " ");
    if text.is_empty() {
        return;
    }
    if !text.trim().is_empty() {
        seen.push(style);
    }
    if let Some(last) = paragraph.last_mut()
        && last.bold == style.bold
        && last.italic == style.italic
    {
        last.text.push_str(&text);
        return;
    }
    paragraph.push(TextSpan::plain(text).with_bold(style.bold).with_italic(style.italic));
}

/// Whether `paragraph` holds more than white space.
fn has_text(paragraph: &[TextSpan]) -> bool {
    paragraph.iter().any(|s| !s.text.trim().is_empty())
}

/// Ends the paragraph being read, dropping the white space the XHTML's layout put around it.
fn end_paragraph(text: &mut RichText, paragraph: &mut Vec<TextSpan>) {
    let mut spans = std::mem::take(paragraph);
    if spans.iter().all(|s| s.text.trim().is_empty()) {
        spans.clear();
    }
    text.paragraphs.push(spans);
}

/// An element or attribute name without its namespace prefix, lower-cased.
fn local_name(name: &str) -> String {
    name.rsplit(':').next().unwrap_or_default().to_ascii_lowercase()
}

/// The value of attribute `name` in a start tag's inside, quoted with `"` or `'`.
fn attribute(tag: &str, name: &str) -> Option<String> {
    let mut rest = tag.split_once(char::is_whitespace)?.1;
    loop {
        let (key, after) = rest.split_once('=')?;
        let after = after.trim_start();
        let quote = after.chars().next().filter(|c| *c == '"' || *c == '\'')?;
        let value_and_rest = after.get(1..)?;
        let end = value_and_rest.find(quote)?;
        if local_name(key.trim()) == name {
            return value_and_rest.get(..end).map(unescape);
        }
        rest = value_and_rest.get(end + 1..)?;
    }
}

/// `text` with XML's character references read: the five named ones and numeric ones. Anything
/// else is kept as written.
fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        let (before, tail) = rest.split_at(at);
        out.push_str(before);
        let reference = tail.find(';').and_then(|end| tail.get(1..end).map(|r| (r, end)));
        let decoded = reference.and_then(|(r, end)| {
            let ch = match r {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                _ => r
                    .strip_prefix("#x")
                    .or_else(|| r.strip_prefix("#X"))
                    .and_then(|h| u32::from_str_radix(h, 16).ok())
                    .or_else(|| r.strip_prefix('#').and_then(|d| d.parse().ok()))
                    .and_then(char::from_u32),
            };
            ch.map(|c| (c, end))
        });
        if let Some((ch, end)) = decoded {
            out.push(ch);
            rest = tail.get(end + 1..).unwrap_or_default();
        } else {
            out.push('&');
            rest = tail.get(1..).unwrap_or_default();
        }
    }
    out.push_str(rest);
    out
}
