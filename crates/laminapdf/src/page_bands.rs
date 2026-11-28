//! The text a page already shows where a header or footer goes (E6 plan §2.3): the runs in the top
//! and bottom tenth of the page, the bands running headers and footers use. A page-marks dialog
//! checks what it is about to add against them and warns of text it would run into.

use laminapdf_model::RectPt;

use crate::document::Document;
use crate::error::Result;

/// The share of the page's height at its top and at its bottom that a running header or footer
/// uses: the bands [`Document::page_bands`] reads.
const BAND: f64 = 0.1;

/// One run of text in a band of a page.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct BandRun {
    /// The run's text.
    pub text: String,
    /// The run's box in page space (ADR 0027): points from the page's top-left corner as shown.
    pub bounds: RectPt,
    /// Whether the file marks the run as an artifact, as the headers and footers that Lamina PDF
    /// and other tools add are in a tagged file.
    pub artifact: bool,
}

/// The text in the top and bottom bands of a page, as [`Document::page_bands`] reads it.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct PageBands {
    /// The runs whose middle is in the top tenth of the page, in the order the page draws them.
    pub header: Vec<BandRun>,
    /// The runs whose middle is in the bottom tenth of the page.
    pub footer: Vec<BandRun>,
}

impl Document {
    /// The text page `page` (0-based) shows in its top and bottom tenths, where running headers
    /// and footers sit (feature `extract`): what a header, a footer or a Bates number added there
    /// would run into. Hidden text (an OCR layer) is left out.
    ///
    /// ```
    /// # use laminapdf::{Document, OpenOptions};
    /// # let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/content/add/footer.pdf"))?;
    /// let doc = Document::open_bytes(bytes, OpenOptions::default())?;
    /// let bands = doc.page_bands(0)?;
    /// assert!(bands.footer.iter().any(|run| run.text.contains("Page 3 of 24")));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// `page-index-out-of-range`; the content limits, `strict`, `data-pending` and the other
    /// reading errors.
    ///
    /// Added in 0.7.0.
    pub fn page_bands(&self, page: u32) -> Result<PageBands> {
        let text = self.page_text(page, &crate::extract::TextOptions::default())?;
        let height = text.frame.height().get();
        let mut bands = PageBands::default();
        for run in text.runs.iter().filter(|r| !r.invisible && !r.text.trim().is_empty()) {
            let middle = f64::midpoint(run.bounds.y0.get(), run.bounds.y1.get());
            let found = BandRun {
                text: run.text.clone(),
                bounds: run.bounds,
                artifact: run.artifact,
            };
            if middle <= height * BAND {
                bands.header.push(found);
            } else if middle >= height * (1.0 - BAND) {
                bands.footer.push(found);
            }
        }
        Ok(bands)
    }
}
