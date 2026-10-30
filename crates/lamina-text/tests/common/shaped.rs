//! Shaped text on the test page (todo B4b): paragraphs laid out with `break_lines` and written
//! with `pdf_run`'s `TJ` arrays, rises and `/ActualText` spans: what the builder (B5b) will
//! do, for the `text-shaped.pdf` golden and the extraction checks.

use lamina_text::{FontKey, HarfRust, LayoutOptions, Result, TjElement, break_lines, pdf_run};
use pdf_writer::{Name, Str, TextStr};

use super::TestPdf;

/// The left edge of the text box, in points.
const BOX_X: f32 = 72.0;

impl TestPdf {
    /// The page's resource name for `key` (`F1`, `F2`, … in order of first use).
    fn resource(&mut self, key: FontKey) -> Result<String> {
        let object = self.fonts.reference(key, &self.doc)?;
        if let Some((name, _)) = self.resources.iter().find(|(_, r)| *r == object) {
            return Ok(name.clone());
        }
        let name = format!("F{}", self.resources.len() + 1);
        self.resources.push((name.clone(), object));
        Ok(name)
    }

    /// Lays `text` out in a box `options.width` wide with the registered fonts `keys` (in
    /// order of preference) and draws its lines; returns each line's drawn text.
    pub fn paragraph(&mut self, keys: &[FontKey], text: &str, options: &LayoutOptions) -> Result<Vec<String>> {
        let fonts: Vec<_> = keys.iter().map(|&k| self.fonts.font(k).unwrap().clone()).collect();
        let lines = break_lines(&HarfRust, &fonts, text, options)?;
        let size = options.size;
        let mut drawn = Vec::new();
        for line in &lines {
            self.y -= size * 1.6;
            let y = self.y;
            self.content.begin_text();
            let mut rise = 0.0_f32;
            // Logical order, so extractors read right-to-left lines in order (B4d).
            for run in line.drawing_order().into_iter().map(|i| &line.runs[i]) {
                let key = keys[run.font];
                let name = self.resource(key)?;
                let pdf = pdf_run(&run.run, text, &mut self.fonts, key)?;
                #[expect(clippy::cast_possible_truncation, reason = "page coordinates")]
                let x = BOX_X + (line.x + run.x) as f32;
                self.content
                    .set_font(Name(name.as_bytes()), size)
                    .set_text_matrix([1.0, 0.0, 0.0, 1.0, x, y]);
                for piece in &pdf.pieces {
                    if let Some(actual) = &piece.actual_text {
                        self.content
                            .begin_marked_content_with_properties(Name(b"Span"))
                            .properties()
                            .actual_text(TextStr(actual));
                    }
                    for segment in &piece.segments {
                        #[expect(clippy::cast_possible_truncation, reason = "small values")]
                        let wanted = (segment.rise * f64::from(size) / 1000.0) as f32;
                        if wanted.to_bits() != rise.to_bits() {
                            self.content.set_rise(wanted);
                            rise = wanted;
                        }
                        let mut show = self.content.show_positioned();
                        let mut items = show.items();
                        for element in &segment.tj {
                            match element {
                                TjElement::Codes(codes) => {
                                    items.show(Str(codes));
                                }
                                #[expect(clippy::cast_possible_truncation, reason = "three decimals at most")]
                                TjElement::Adjust(amount) => {
                                    items.adjust(*amount as f32);
                                }
                            }
                        }
                    }
                    if piece.actual_text.is_some() {
                        self.content.end_marked_content();
                    }
                }
            }
            if rise != 0.0 {
                self.content.set_rise(0.0);
            }
            self.content.end_text();
            drawn.push(text[line.content.clone()].to_owned());
        }
        Ok(drawn)
    }
}
