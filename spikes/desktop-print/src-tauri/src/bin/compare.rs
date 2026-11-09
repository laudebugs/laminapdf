//! Fidelity of each printed PDF against its fixture, page by page: page count, displayed page
//! size, SSIM at 150 dpi (both rendered by PDFium at the fixture page's pixel size), and text
//! the original page lacks (a header or footer the print path added).
//!
//! ```sh
//! cargo run --release --bin compare -- <fixtures-dir> <out-dir> <fidelity.csv>
//! ```
//! Reads `<out-dir>/<label>-<path>-<fixture>.pdf` and appends one row per page. D4c can reuse it
//! as a manual check.

use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::PathBuf;

use desktop_print::pdf;
use laminapdf_test_fixtures::image::{Bitmap, ssim, write_png};

const DPI: f32 = 150.0;
/// Page sizes within this many points count as equal.
const SIZE_TOLERANCE: f32 = 2.0;

/// Words of two or more letters or digits: what a header or footer adds (a title, a URL, a
/// date, "1/50"), without the punctuation that renderers extract differently.
fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 2)
        .map(str::to_lowercase)
        .collect()
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: compare <fixtures-dir> <out-dir> <fidelity.csv>";
    let (fixtures, outs, csv) = (
        PathBuf::from(args.next().ok_or(usage)?),
        PathBuf::from(args.next().ok_or(usage)?),
        PathBuf::from(args.next().ok_or(usage)?),
    );
    let new = !csv.exists();
    let mut f = OpenOptions::new().create(true).append(true).open(&csv)?;
    if new {
        writeln!(f, "label,path,fixture,page,ref_w,ref_h,out_w,out_h,size_ok,shift_x_pt,shift_y_pt,ssim,added_text,note")?;
    }
    let mut entries: Vec<_> = std::fs::read_dir(&outs)?.filter_map(Result::ok).map(|e| e.path()).collect();
    entries.sort();
    for out in entries.into_iter().filter(|p| p.extension().is_some_and(|e| e == "pdf")) {
        let stem = out.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_owned();
        // A page-range output: <label>-<path>-<fixture>@<first>-<last>.
        let (base, first, last) = match stem.split_once('@') {
            Some((base, range)) => {
                let (a, b) = range.split_once('-').unwrap_or((range, range));
                (base.to_owned(), a.parse::<i32>().unwrap_or(1), Some(b.parse::<i32>().unwrap_or(1)))
            }
            None => (stem.clone(), 1, None),
        };
        let Some((fixture, path, label)) = pdf::FIXTURES.iter().find_map(|fx| {
            let rest = base.strip_suffix(&format!("-{fx}"))?;
            let (label, path) = rest.rsplit_once('-')?;
            Some((*fx, path.to_owned(), label.to_owned()))
        }) else {
            continue;
        };
        let reference = pdf::open(&std::fs::read(fixtures.join(format!("{fixture}.pdf")))?)?;
        let printed = match std::fs::read(&out).map_err(|e| e.to_string()).and_then(|b| pdf::open(&b)) {
            Ok(d) => d,
            Err(e) => {
                writeln!(f, "{label},{path},{fixture},,,,,,,,,,,unreadable: {}", e.replace(',', ";"))?;
                continue;
            }
        };
        let fixture_name = match last {
            Some(l) => format!("{fixture}@{first}-{l}"),
            None => fixture.to_owned(),
        };
        // The reference pages this output should hold.
        let skip = first - 1;
        let rn = match last {
            Some(l) => l - first + 1,
            None => reference.pages().len(),
        };
        let pn = printed.pages().len();
        // Every word of the original, so a page printed on the wrong sheet isn't blamed for text.
        let ref_words: Vec<String> = reference
            .pages()
            .iter()
            .flat_map(|p| words(&p.text().map(|t| t.all()).unwrap_or_default()))
            .collect();
        if rn != pn {
            writeln!(f, "{label},{path},{fixture_name},,,,,,,,,,,{pn} pages printed for {rn}")?;
        }
        for i in 0..rn.min(pn) {
            let Ok(rp) = reference.pages().get(i + skip) else {
                writeln!(f, "{label},{path},{fixture_name},,,,,,,,,,,no page {} in the original", i + skip + 1)?;
                continue;
            };
            let pp = printed.pages().get(i)?;
            let (rw, rh, ow, oh) = (rp.width().value, rp.height().value, pp.width().value, pp.height().value);
            let size_ok = (rw - ow).abs() <= SIZE_TOLERANCE && (rh - oh).abs() <= SIZE_TOLERANCE;
            let (px_w, px_h) = ((rw * DPI / 72.0).round() as i32, (rh * DPI / 72.0).round() as i32);
            let a = pdf::render_at(&rp, px_w, px_h, false)?;
            let b = pdf::render_at(&pp, px_w, px_h, false)?;
            // Both at the original's pixel size: a page on the wrong paper is stretched, and scores low.
            let (a, b) = (bitmap(a), bitmap(b));
            // The offset of the printed content (a shift is reported apart from the score), and
            // SSIM once the shift is undone.
            let (dx, dy) = offset(&a, &b);
            let score = format!("{:.4}", ssim(&a, &shifted(&b, dx, dy)));
            // LAMINA_SPIKE_DUMP=<dir>: both renders as PNGs, to look at a low score.
            if let Some(dir) = std::env::var_os("LAMINA_SPIKE_DUMP") {
                let dir = PathBuf::from(dir);
                let _ = std::fs::create_dir_all(&dir);
                write_png(&dir.join(format!("{stem}-p{}-ref.png", i + 1)), &a);
                write_png(&dir.join(format!("{stem}-p{}-out.png", i + 1)), &b);
            }
            let out_text = pp.text().map(|t| t.all()).unwrap_or_default();
            let added: Vec<String> = words(&out_text).into_iter().filter(|w| !ref_words.contains(w)).collect();
            let added = added.join(" ").replace(',', ";");
            let (sx, sy) = (dx as f32 * 72.0 / DPI, dy as f32 * 72.0 / DPI);
            writeln!(f, "{label},{path},{fixture_name},{},{rw:.1},{rh:.1},{ow:.1},{oh:.1},{size_ok},{sx:.1},{sy:.1},{score},{added},", i + skip + 1)?;
        }
        println!("compared {stem}");
    }
    Ok(())
}

/// Ink per row and per column (sum of darkness), the profiles the offset search compares.
#[expect(clippy::needless_range_loop, reason = "x and y index the pixels and both profiles")]
fn profiles(b: &Bitmap) -> (Vec<u64>, Vec<u64>) {
    let (w, h) = (b.width as usize, b.height as usize);
    let (mut rows, mut cols) = (vec![0u64; h], vec![0u64; w]);
    for y in 0..h {
        for x in 0..w {
            let i = (y * w + x) * 4;
            let ink = 765 - (u64::from(b.rgba[i]) + u64::from(b.rgba[i + 1]) + u64::from(b.rgba[i + 2]));
            rows[y] += ink;
            cols[x] += ink;
        }
    }
    (rows, cols)
}

/// The search range: ±120 px is ±57.6 pt at 150 dpi.
const MAX_SHIFT: i32 = 120;

/// The shift (in pixels, printed minus original) that best lines the profiles up; on a tie
/// (blank or flat profiles) the smallest shift wins, so an empty page reads 0, not the edge.
fn best_shift(a: &[u64], b: &[u64]) -> i32 {
    let n = a.len() as i32;
    let cost = |s: i32| (MAX_SHIFT..n - MAX_SHIFT).map(|i| a[i as usize].abs_diff(b[(i + s) as usize])).sum::<u64>();
    let mut shifts: Vec<i32> = (-MAX_SHIFT..=MAX_SHIFT).collect();
    shifts.sort_by_key(|s| s.abs());
    let mut best = (0, u64::MAX);
    for s in shifts {
        let c = cost(s);
        if c < best.1 {
            best = (s, c);
        }
    }
    best.0
}

fn offset(a: &Bitmap, b: &Bitmap) -> (i32, i32) {
    let ((ar, ac), (br, bc)) = (profiles(a), profiles(b));
    (best_shift(&ac, &bc), best_shift(&ar, &br))
}

/// `b` moved by (−dx, −dy), white where nothing moves in.
fn shifted(b: &Bitmap, dx: i32, dy: i32) -> Bitmap {
    let (w, h) = (b.width as i32, b.height as i32);
    let mut rgba = vec![255u8; b.rgba.len()];
    for y in 0..h {
        let sy = y + dy;
        if sy < 0 || sy >= h {
            continue;
        }
        for x in 0..w {
            let sx = x + dx;
            if sx < 0 || sx >= w {
                continue;
            }
            let (d, s) = (((y * w + x) * 4) as usize, ((sy * w + sx) * 4) as usize);
            rgba[d..d + 4].copy_from_slice(&b.rgba[s..s + 4]);
        }
    }
    Bitmap { width: b.width, height: b.height, rgba }
}

fn bitmap(r: pdf::Raster) -> Bitmap {
    Bitmap { width: r.width, height: r.height, rgba: r.rgba }
}
