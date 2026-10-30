//! hayro's half: render one page at a DPI to an opaque RGB bitmap, with the settings of the
//! hayro-parity spike (`spikes/hayro-parity/src/render.rs`): the page's render dimensions
//! (`CropBox` with `/Rotate`), `floor(points × dpi / 72)` pixels per axis, a white background,
//! default settings (embedded standard 14 fonts and CMaps, no system fonts).

use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_syntax::Pdf;
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::{PixmapSettings, RenderCache, RenderSettings};

use crate::image::Rgb;

/// A loaded document.
pub struct Doc {
    pdf: Pdf,
    settings: InterpreterSettings,
}

impl Doc {
    pub fn load(bytes: Vec<u8>) -> Result<Self> {
        let pdf = Pdf::new(Arc::new(bytes)).map_err(|e| anyhow!("hayro cannot load the file: {e:?}"))?;
        Ok(Self {
            pdf,
            settings: InterpreterSettings::default(),
        })
    }

    pub fn page_count(&self) -> usize {
        self.pdf.pages().len()
    }

    /// Page `index` at `dpi`.
    pub fn render(&self, index: usize, dpi: f32) -> Result<Rgb> {
        let page = self.pdf.pages().get(index).context("no such page")?;
        let scale = dpi / 72.0;
        let cache = RenderCache::new();
        let pixmap = hayro::render(
            page,
            &cache,
            &self.settings,
            &RenderSettings::default(),
            &PixmapSettings {
                x_scale: scale,
                y_scale: scale,
                bg_color: WHITE,
            },
        );
        let mut data = Vec::new();
        for px in pixmap.data_as_u8_slice().as_chunks::<4>().0 {
            // Premultiplied over white: c + (255 - a).
            let [r, g, b, a] = *px;
            data.extend([r, g, b].map(|c| c.saturating_add(255 - a)));
        }
        Rgb::new(u32::from(pixmap.width()), u32::from(pixmap.height()), data)
    }
}
