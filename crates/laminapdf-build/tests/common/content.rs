//! Content-stream builder (ISO 32000-1 §8–9 operators), ported from the `pdf-basics` spike as
//! test support for the frozen `hello`/`imaging-demo` goldens (`legacy.rs`) and
//! `laminapdf-save`'s raw-writer tests. `laminapdf-build` itself writes content with
//! `pdf_writer::Content` (ADR 0005 §6); this builder stays only because those goldens need
//! its exact formatting (`%` comments, fixed decimals, operators grouped on one line).
//!
//! Operators are separated by `\n` (like Python's `"\n".join(ops)`); inside
//! [`ContentBuilder::line`] they are separated by a single space instead. No trailing
//! newline is added unless [`ContentBuilder::end_line`] is called.

use laminapdf_cos::{Name, Object, PdfString};
use laminapdf_syntax::format_real;

fn write_name(bytes: &[u8], out: &mut Vec<u8>) {
    Object::Name(Name::new(bytes)).write_to(out);
}

fn write_literal_string(bytes: &[u8], out: &mut Vec<u8>) {
    Object::String(PdfString::literal(bytes)).write_to(out);
}

/// The circle-approximation constant κ = 4(√2 − 1)/3.
pub const KAPPA: f64 = 4.0 * (std::f64::consts::SQRT_2 - 1.0) / 3.0;

/// A numeric operand: default real formatting, or a fixed number of decimals.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Num {
    /// Formatted like [`format_real`]: up to 6 decimals, trailing zeros trimmed.
    Auto(f64),
    /// Formatted with exactly this many decimals (like Python's `f"{x:.Nf}"`).
    Fixed(f64, u8),
}

/// Shorthand for [`Num::Fixed`].
#[must_use]
pub const fn fixed(value: f64, decimals: u8) -> Num {
    Num::Fixed(value, decimals)
}

impl From<f64> for Num {
    fn from(v: f64) -> Self {
        Self::Auto(v)
    }
}

impl From<i32> for Num {
    fn from(v: i32) -> Self {
        Self::Auto(f64::from(v))
    }
}

impl Num {
    fn write(self, out: &mut Vec<u8>) {
        let text = match self {
            Self::Auto(v) => format_real(v),
            Self::Fixed(v, d) => format!("{v:.prec$}", prec = usize::from(d)),
        };
        out.extend_from_slice(text.as_bytes());
    }
}

/// Builds content-stream bytes operator by operator.
#[derive(Debug, Clone, Default)]
pub struct ContentBuilder {
    buf: Vec<u8>,
    /// Separator owed before the next operator (`None` at the very start or after `end_line`).
    pending: Option<u8>,
    in_line: bool,
}

impl ContentBuilder {
    /// An empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The bytes built so far.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf
    }

    /// Consumes the builder, returning its bytes.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }

    fn begin_item(&mut self) {
        if let Some(sep) = self.pending.take() {
            self.buf.push(sep);
        }
    }

    fn end_item(&mut self) {
        self.pending = Some(if self.in_line { b' ' } else { b'\n' });
    }

    fn op(&mut self, operands: &[Num], operator: &str) -> &mut Self {
        self.begin_item();
        for n in operands {
            n.write(&mut self.buf);
            self.buf.push(b' ');
        }
        self.buf.extend_from_slice(operator.as_bytes());
        self.end_item();
        self
    }

    fn name_op(&mut self, name: &str, operator: &str) -> &mut Self {
        self.begin_item();
        write_name(name.as_bytes(), &mut self.buf);
        self.buf.push(b' ');
        self.buf.extend_from_slice(operator.as_bytes());
        self.end_item();
        self
    }

    /// Emits the operators added by `f` on one line, separated by spaces.
    pub fn line(&mut self, f: impl FnOnce(&mut Self)) -> &mut Self {
        let outer = self.in_line;
        self.begin_item();
        self.in_line = true;
        f(self);
        self.in_line = outer;
        self.end_item();
        self
    }

    /// Terminates the current line with `\n` (e.g. for a trailing newline at the end).
    pub fn end_line(&mut self) -> &mut Self {
        self.pending = None;
        self.buf.push(b'\n');
        self
    }

    /// Appends pre-formatted operator text as one item.
    pub fn raw(&mut self, text: &str) -> &mut Self {
        self.begin_item();
        self.buf.extend_from_slice(text.as_bytes());
        self.end_item();
        self
    }

    /// `% text` comment. A comment always ends its line.
    pub fn comment(&mut self, text: &str) -> &mut Self {
        self.begin_item();
        self.buf.push(b'%');
        self.buf.push(b' ');
        self.buf.extend_from_slice(text.as_bytes());
        self.pending = Some(b'\n');
        self
    }

    /// `q` — save graphics state.
    #[doc(alias = "q")]
    pub fn save_state(&mut self) -> &mut Self {
        self.op(&[], "q")
    }

    /// `Q` — restore graphics state.
    #[doc(alias = "Q")]
    pub fn restore_state(&mut self) -> &mut Self {
        self.op(&[], "Q")
    }

    /// `a b c d e f cm` — concatenate matrix.
    #[doc(alias = "cm")]
    #[expect(
        clippy::many_single_char_names,
        reason = "a–f are the matrix operand names of ISO 32000 §8.3.4"
    )]
    pub fn transform(
        &mut self,
        a: impl Into<Num>,
        b: impl Into<Num>,
        c: impl Into<Num>,
        d: impl Into<Num>,
        e: impl Into<Num>,
        f: impl Into<Num>,
    ) -> &mut Self {
        self.op(&[a.into(), b.into(), c.into(), d.into(), e.into(), f.into()], "cm")
    }

    /// `w` — line width.
    #[doc(alias = "w")]
    pub fn line_width(&mut self, w: impl Into<Num>) -> &mut Self {
        self.op(&[w.into()], "w")
    }

    /// `J` — line cap (0 butt, 1 round, 2 square).
    #[doc(alias = "J")]
    pub fn line_cap(&mut self, cap: i32) -> &mut Self {
        self.op(&[cap.into()], "J")
    }

    /// `j` — line join (0 miter, 1 round, 2 bevel).
    #[doc(alias = "j")]
    pub fn line_join(&mut self, join: i32) -> &mut Self {
        self.op(&[join.into()], "j")
    }

    /// `[a b …] phase d` — dash pattern.
    #[doc(alias = "d")]
    pub fn dash(&mut self, pattern: &[f64], phase: impl Into<Num>) -> &mut Self {
        self.begin_item();
        self.buf.push(b'[');
        for (i, &v) in pattern.iter().enumerate() {
            if i > 0 {
                self.buf.push(b' ');
            }
            Num::Auto(v).write(&mut self.buf);
        }
        self.buf.extend_from_slice(b"] ");
        phase.into().write(&mut self.buf);
        self.buf.extend_from_slice(b" d");
        self.end_item();
        self
    }

    /// `r g b RG` — stroking colour, `DeviceRGB`.
    #[doc(alias = "RG")]
    pub fn stroke_rgb(&mut self, r: impl Into<Num>, g: impl Into<Num>, b: impl Into<Num>) -> &mut Self {
        self.op(&[r.into(), g.into(), b.into()], "RG")
    }

    /// `r g b rg` — non-stroking colour, `DeviceRGB`.
    #[doc(alias = "rg")]
    pub fn fill_rgb(&mut self, r: impl Into<Num>, g: impl Into<Num>, b: impl Into<Num>) -> &mut Self {
        self.op(&[r.into(), g.into(), b.into()], "rg")
    }

    /// `x y m` — begin subpath.
    #[doc(alias = "m")]
    pub fn move_to(&mut self, x: impl Into<Num>, y: impl Into<Num>) -> &mut Self {
        self.op(&[x.into(), y.into()], "m")
    }

    /// `x y l` — line segment.
    #[doc(alias = "l")]
    pub fn line_to(&mut self, x: impl Into<Num>, y: impl Into<Num>) -> &mut Self {
        self.op(&[x.into(), y.into()], "l")
    }

    /// `x1 y1 x2 y2 x3 y3 c` — cubic Bézier.
    #[doc(alias = "c")]
    pub fn curve_to(
        &mut self,
        x1: impl Into<Num>,
        y1: impl Into<Num>,
        x2: impl Into<Num>,
        y2: impl Into<Num>,
        x3: impl Into<Num>,
        y3: impl Into<Num>,
    ) -> &mut Self {
        self.op(&[x1.into(), y1.into(), x2.into(), y2.into(), x3.into(), y3.into()], "c")
    }

    /// `h` — close subpath.
    #[doc(alias = "h")]
    pub fn close_path(&mut self) -> &mut Self {
        self.op(&[], "h")
    }

    /// `x y w h re` — rectangle.
    #[doc(alias = "re")]
    pub fn rect(&mut self, x: impl Into<Num>, y: impl Into<Num>, w: impl Into<Num>, h: impl Into<Num>) -> &mut Self {
        self.op(&[x.into(), y.into(), w.into(), h.into()], "re")
    }

    /// `S` — stroke.
    #[doc(alias = "S")]
    pub fn stroke(&mut self) -> &mut Self {
        self.op(&[], "S")
    }

    /// `f` — fill (non-zero winding).
    #[doc(alias = "f")]
    pub fn fill(&mut self) -> &mut Self {
        self.op(&[], "f")
    }

    /// `B` — fill and stroke.
    #[doc(alias = "B")]
    pub fn fill_stroke(&mut self) -> &mut Self {
        self.op(&[], "B")
    }

    /// `W` — clip (non-zero winding), takes effect at the next painting operator.
    #[doc(alias = "W")]
    pub fn clip(&mut self) -> &mut Self {
        self.op(&[], "W")
    }

    /// `n` — end path without painting.
    #[doc(alias = "n")]
    pub fn end_path(&mut self) -> &mut Self {
        self.op(&[], "n")
    }

    /// `/Name sh` — paint shading.
    #[doc(alias = "sh")]
    pub fn shade(&mut self, name: &str) -> &mut Self {
        self.name_op(name, "sh")
    }

    /// `/Name gs` — set graphics state parameters.
    #[doc(alias = "gs")]
    pub fn ext_gstate(&mut self, name: &str) -> &mut Self {
        self.name_op(name, "gs")
    }

    /// `BT` — begin text object.
    #[doc(alias = "BT")]
    pub fn begin_text(&mut self) -> &mut Self {
        self.op(&[], "BT")
    }

    /// `ET` — end text object.
    #[doc(alias = "ET")]
    pub fn end_text(&mut self) -> &mut Self {
        self.op(&[], "ET")
    }

    /// `/Font size Tf`.
    #[doc(alias = "Tf")]
    pub fn font(&mut self, name: &str, size: impl Into<Num>) -> &mut Self {
        self.begin_item();
        write_name(name.as_bytes(), &mut self.buf);
        self.buf.push(b' ');
        size.into().write(&mut self.buf);
        self.buf.extend_from_slice(b" Tf");
        self.end_item();
        self
    }

    /// `tx ty Td` — move text position.
    #[doc(alias = "Td")]
    pub fn text_move(&mut self, tx: impl Into<Num>, ty: impl Into<Num>) -> &mut Self {
        self.op(&[tx.into(), ty.into()], "Td")
    }

    /// `leading TL`.
    #[doc(alias = "TL")]
    pub fn leading(&mut self, leading: impl Into<Num>) -> &mut Self {
        self.op(&[leading.into()], "TL")
    }

    fn string_op(&mut self, text: &[u8], operator: &str) -> &mut Self {
        self.begin_item();
        write_literal_string(text, &mut self.buf);
        self.buf.push(b' ');
        self.buf.extend_from_slice(operator.as_bytes());
        self.end_item();
        self
    }

    /// `(text) Tj` — show a string (bytes are written as a literal string).
    #[doc(alias = "Tj")]
    pub fn show_text(&mut self, text: impl AsRef<[u8]>) -> &mut Self {
        self.string_op(text.as_ref(), "Tj")
    }

    /// `(text) '` — move to the next line and show a string.
    pub fn next_line_show(&mut self, text: impl AsRef<[u8]>) -> &mut Self {
        self.string_op(text.as_ref(), "'")
    }

    /// A circle as four Bézier arcs (`m`, four `c`, `h`), all operands with 3 decimals,
    /// matching `make_imaging_demo.py`.
    pub fn circle(&mut self, cx: f64, cy: f64, r: f64) -> &mut Self {
        let k = KAPPA * r;
        let p = |v: f64| fixed(v, 3);
        self.move_to(p(cx + r), p(cy))
            .curve_to(p(cx + r), p(cy + k), p(cx + k), p(cy + r), p(cx), p(cy + r))
            .curve_to(p(cx - k), p(cy + r), p(cx - r), p(cy + k), p(cx - r), p(cy))
            .curve_to(p(cx - r), p(cy - k), p(cx - k), p(cy - r), p(cx), p(cy - r))
            .curve_to(p(cx + k), p(cy - r), p(cx + r), p(cy - k), p(cx + r), p(cy))
            .close_path()
    }
}
