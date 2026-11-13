//! E7a spike: what an editable content IR costs, and how much of a page's content survives an
//! edit byte for byte under each encoder ADR 0022 weighs.
//!
//! For every page of the PDFs named on the command line it:
//!
//! 1. reads the page's decoded content (`/Contents`, concatenated);
//! 2. tokenizes it into operators with byte spans (B9's `ContentIter`) and groups them into the
//!    objects an editor selects: text objects (`BT … ET`), paths (construction up to the
//!    painting operator), XObjects (`Do`), inline images, shadings (`sh`), with marked content
//!    nesting counted;
//! 3. re-serialises every operator canonically (full re-serialise), and checks that the result
//!    tokenizes to the same operators;
//! 4. edits one object (the first text object, else the first object) the span-preserving way:
//!    its span re-serialised canonically and wrapped in `q 1 0 0 1 10 0 cm … Q` (a move), the
//!    rest copied byte for byte, and checks the result tokenizes to what was meant.
//!
//! It prints one row per file and the totals. Nothing here is product code.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use laminapdf_content::{ContentIter, Lexed};
use laminapdf_cos::{Document, OpenOptions};
use laminapdf_syntax::Object;

/// One operator with the bytes it came from: `start` is the end of the previous operator (so
/// the span takes its operands and the white space before them), `end` the end of its keyword
/// (or of `EI` for an inline image).
struct IrOp {
    keyword: Vec<u8>,
    operands: Vec<Object>,
    inline: Option<Vec<u8>>,
    start: usize,
    end: usize,
}

/// An object an editor selects: a range of operators.
struct IrObject {
    first: usize,
    last: usize,
}

#[derive(Default)]
struct Totals {
    files: usize,
    unreadable: usize,
    pages: usize,
    bytes: usize,
    ops: usize,
    objects: usize,
    text_objects: usize,
    parse: Duration,
    group: Duration,
    encode: Duration,
    canonical_bytes: usize,
    canonical_same: usize,
    canonical_close: usize,
    edited_pages: usize,
    verbatim_bytes: usize,
    edit_same: usize,
    slowest_page: Duration,
}

const PAINT: &[&[u8]] = &[b"S", b"s", b"f", b"F", b"f*", b"B", b"B*", b"b", b"b*", b"n"];
const PATH: &[&[u8]] = &[b"m", b"l", b"c", b"v", b"y", b"h", b"re"];

fn main() {
    let paths: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    let mut t = Totals::default();
    println!(
        "| file | pages | content KB | ops | objects | parse ms | canonical / original | canonical re-reads the same | verbatim after one edit |"
    );
    println!("| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |");
    for path in paths {
        let Ok(bytes) = std::fs::read(&path) else { continue };
        t.files += 1;
        let Ok(doc) = Document::open(bytes, OpenOptions::default()) else {
            t.unreadable += 1;
            continue;
        };
        let Ok(pages) = doc.pages() else {
            t.unreadable += 1;
            continue;
        };
        let before = (
            t.pages,
            t.bytes,
            t.ops,
            t.objects,
            t.parse,
            t.canonical_bytes,
            t.canonical_same,
            t.verbatim_bytes,
            t.edited_pages,
        );
        for page in &pages {
            let Ok(content) = doc.page_content(page) else { continue };
            page_row(&content, &mut t);
        }
        let file_pages = t.pages - before.0;
        let file_bytes = t.bytes - before.1;
        let edited = t.edited_pages - before.8;
        println!(
            "| {} | {} | {:.1} | {} | {} | {:.2} | {:.3} | {}/{} | {} |",
            path.file_name().unwrap_or_default().to_string_lossy(),
            file_pages,
            file_bytes as f64 / 1024.0,
            t.ops - before.2,
            t.objects - before.3,
            (t.parse - before.4).as_secs_f64() * 1000.0,
            ratio(t.canonical_bytes - before.5, file_bytes),
            t.canonical_same - before.6,
            file_pages,
            if edited == 0 {
                "—".to_owned()
            } else {
                format!(
                    "{:.1} %",
                    100.0 * (t.verbatim_bytes - before.7) as f64 / file_bytes.max(1) as f64
                )
            },
        );
    }
    summary(&t);
}

fn ratio(a: usize, b: usize) -> f64 {
    if b == 0 { 1.0 } else { a as f64 / b as f64 }
}

fn page_row(content: &[u8], t: &mut Totals) {
    t.pages += 1;
    t.bytes += content.len();
    let started = Instant::now();
    let ops = parse(content);
    let parsed = started.elapsed();
    let objects = group(&ops);
    let grouped = started.elapsed() - parsed;
    t.parse += parsed;
    t.group += grouped;
    t.ops += ops.len();
    t.objects += objects.len();
    t.text_objects += objects.iter().filter(|o| ops[o.first].keyword == b"BT").count();
    t.slowest_page = t.slowest_page.max(parsed + grouped);

    // Full re-serialise.
    let encoding = Instant::now();
    let canonical = encode(&ops);
    t.encode += encoding.elapsed();
    t.canonical_bytes += canonical.len();
    let reparsed = parse(&canonical);
    if same_ops(&reparsed, &ops) {
        t.canonical_same += 1;
        t.canonical_close += 1;
    } else if close_ops(&reparsed, &ops) {
        t.canonical_close += 1;
    } else if !close_ops(&reparsed, &ops) && std::env::var_os("SPIKE_DEBUG").is_some() {
        let i = reparsed
            .iter()
            .zip(&ops)
            .position(|(x, y)| x.keyword != y.keyword || x.operands != y.operands || x.inline != y.inline)
            .unwrap_or(reparsed.len().min(ops.len()));
        let show = |o: Option<&IrOp>| {
            o.map(|o| {
                format!(
                    "{} {:?} inline={}",
                    String::from_utf8_lossy(&o.keyword),
                    o.operands,
                    o.inline.as_ref().map_or(0, Vec::len)
                )
            })
            .unwrap_or_default()
        };
        eprintln!(
            "canonical differs at op {i} of {}/{}: was [{}] now [{}]",
            ops.len(),
            reparsed.len(),
            show(ops.get(i)),
            show(reparsed.get(i))
        );
    }

    // Span-preserving: one object moved, everything else copied byte for byte.
    let Some(target) = objects
        .iter()
        .find(|o| ops[o.first].keyword == b"BT")
        .or_else(|| objects.first())
    else {
        return;
    };
    let (from, to) = (ops[target.first].start, ops[target.last].end);
    let mut edited = content[..from].to_vec();
    edited.extend_from_slice(b"\nq 1 0 0 1 10 0 cm\n");
    edited.extend_from_slice(&encode(&ops[target.first..=target.last]));
    edited.extend_from_slice(b"Q\n");
    edited.extend_from_slice(&content[to..]);
    t.edited_pages += 1;
    t.verbatim_bytes += content.len() - (to - from);
    let reread = parse(&edited);
    // What the edit meant: the operators before, q, cm, the object's, Q, the operators after.
    let expected_len = ops.len() + 3;
    let body_same = reread.len() == expected_len
        && close_ops(&reread[..target.first], &ops[..target.first])
        && close_ops(
            &reread[target.first + 2..target.last + 3],
            &ops[target.first..=target.last],
        )
        && close_ops(&reread[target.last + 4..], &ops[target.last + 1..]);
    if body_same {
        t.edit_same += 1;
    } else if std::env::var_os("SPIKE_DEBUG").is_some() {
        let kw = |o: &[IrOp]| {
            o.iter()
                .take(6)
                .map(|o| String::from_utf8_lossy(&o.keyword).into_owned())
                .collect::<Vec<_>>()
                .join(" ")
        };
        eprintln!(
            "edit differs: ops {} -> {} (expected {}), object {}..={} [{}]; around: [{}] reread [{}]",
            ops.len(),
            reread.len(),
            expected_len,
            target.first,
            target.last,
            kw(&ops[target.first..=target.last]),
            String::from_utf8_lossy(&content[from..to.min(from + 80)]).replace('\n', " "),
            kw(&reread[target.first.saturating_sub(0)..]),
        );
    }
}

fn parse(content: &[u8]) -> Vec<IrOp> {
    let mut ops = Vec::new();
    let mut previous_end = 0;
    for lexed in ContentIter::new(content) {
        let Ok(Lexed::Op(op)) = lexed else { continue };
        let keyword = op.operator.keyword().to_vec();
        let (end, inline) = match &op.inline_image {
            Some(image) => {
                // BI's offset through the end of the EI after the data.
                let after = image.data_offset + image.data.len();
                let ei = content[after..]
                    .windows(2)
                    .position(|w| w == b"EI")
                    .map_or(content.len(), |p| after + p + 2);
                let start = content[..op.offset + 1]
                    .windows(2)
                    .rposition(|w| w == b"BI")
                    .unwrap_or(op.offset);
                (ei, Some(content[start..ei].to_vec()))
            }
            None => (op.offset + keyword.len(), None),
        };
        ops.push(IrOp {
            keyword,
            operands: op.operands,
            inline,
            start: previous_end,
            end,
        });
        previous_end = end;
    }
    ops
}

fn group(ops: &[IrOp]) -> Vec<IrObject> {
    let mut objects = Vec::new();
    let mut i = 0;
    while i < ops.len() {
        let k = ops[i].keyword.as_slice();
        let end = if k == b"BT" {
            (i..ops.len())
                .find(|&j| ops[j].keyword == b"ET")
                .unwrap_or(ops.len() - 1)
        } else if PATH.contains(&k) {
            (i..ops.len())
                .find(|&j| PAINT.contains(&ops[j].keyword.as_slice()))
                .unwrap_or(ops.len() - 1)
        } else if k == b"Do" || k == b"BI" || k == b"sh" {
            i
        } else {
            i += 1;
            continue;
        };
        objects.push(IrObject { first: i, last: end });
        i = end + 1;
    }
    objects
}

fn encode(ops: &[IrOp]) -> Vec<u8> {
    let mut out = Vec::new();
    for op in ops {
        if let Some(raw) = &op.inline {
            out.extend_from_slice(raw);
            out.push(b'\n');
            continue;
        }
        for operand in &op.operands {
            operand.write_to(&mut out);
            out.push(b' ');
        }
        out.extend_from_slice(&op.keyword);
        out.push(b'\n');
    }
    out
}

fn same_ops(a: &[IrOp], b: &[IrOp]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(x, y)| x.keyword == y.keyword && x.operands == y.operands && x.inline == y.inline)
}

fn number(o: &Object) -> Option<f64> {
    match o {
        Object::Integer(i) => Some(*i as f64),
        Object::Real(r) => Some(*r),
        _ => None,
    }
}

/// Equal but for numbers, which may differ by 1e-6 and switch between integer and real.
fn close_ops(a: &[IrOp], b: &[IrOp]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(x, y)| {
            x.keyword == y.keyword
                && x.inline == y.inline
                && x.operands.len() == y.operands.len()
                && x.operands
                    .iter()
                    .zip(&y.operands)
                    .all(|(p, q)| match (number(p), number(q)) {
                        (Some(p), Some(q)) => (p - q).abs() <= 1e-6 * p.abs().max(1.0),
                        _ => close_object(p, q),
                    })
        })
}

fn close_object(p: &Object, q: &Object) -> bool {
    match (p, q) {
        (Object::Array(p), Object::Array(q)) => {
            p.len() == q.len()
                && p.iter().zip(q).all(|(p, q)| match (number(p), number(q)) {
                    (Some(p), Some(q)) => (p - q).abs() <= 1e-6 * p.abs().max(1.0),
                    _ => close_object(p, q),
                })
        }
        _ => p == q,
    }
}

fn summary(t: &Totals) {
    let ms = |d: Duration| d.as_secs_f64() * 1000.0;
    let mb = t.bytes as f64 / (1024.0 * 1024.0);
    println!();
    println!(
        "files {} (unreadable {}), pages {}, content {:.2} MB, operators {}, objects {} ({} text objects)",
        t.files, t.unreadable, t.pages, mb, t.ops, t.objects, t.text_objects
    );
    println!(
        "parse {:.1} ms ({:.1} MB/s), group {:.1} ms, canonical encode {:.1} ms; slowest page {:.2} ms",
        ms(t.parse),
        mb / t.parse.as_secs_f64().max(1e-9),
        ms(t.group),
        ms(t.encode),
        ms(t.slowest_page)
    );
    println!(
        "full re-serialise: {:.3} of the original size; {}/{} pages re-read to identical operators, {}/{} equal within 1e-6",
        ratio(t.canonical_bytes, t.bytes),
        t.canonical_same,
        t.pages,
        t.canonical_close,
        t.pages
    );
    println!(
        "span-preserving, one object moved: {:.2} % of the bytes copied verbatim; {}/{} edited pages re-read as meant (numbers within 1e-6)",
        100.0 * t.verbatim_bytes as f64 / t.bytes.max(1) as f64,
        t.edit_same,
        t.edited_pages
    );
}
