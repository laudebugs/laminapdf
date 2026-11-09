//! Tile sources and the tile cache. The cache is there so the benchmark measures the
//! transport, not the renderer: a prime pass fills it, and render time is reported apart.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

use laminapdf_render::{
    ByteSource, CancelToken, PixelRect, RenderBackend, RenderDocument, RenderOpenOptions, RenderRequest,
};
use laminapdf_render_pdfium::PdfiumBackend;
use serde::{Deserialize, Serialize};

/// One tile: a page, a scale in thousandths of a pixel per point, the tile's top-left pixel
/// and its nominal size (edge tiles are smaller).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TileRequest {
    pub doc: u32,
    pub page: u32,
    pub scale_milli: u32,
    pub x: u32,
    pub y: u32,
    pub size: u16,
}

impl TileRequest {
    fn scale(&self) -> f32 {
        self.scale_milli as f32 / 1000.0
    }
}

/// Straight-alpha RGBA8, rows packed at `width × 4`.
pub struct Tile {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Tile {
    /// FNV-1a over the first row: the webview recomputes it, so a transport that corrupts
    /// bytes can't win.
    pub fn first_row_sum(&self) -> u32 {
        let row = self.rgba.get(..self.width as usize * 4).unwrap_or(&self.rgba);
        row.iter()
            .fold(0x811c_9dc5_u32, |h, &b| (h ^ u32::from(b)).wrapping_mul(0x0100_0193))
    }
}

/// A document the benchmark scrolls through.
#[derive(Serialize)]
pub struct DocInfo {
    pub name: String,
    /// Displayed page sizes in points.
    pub pages: Vec<[f32; 2]>,
}

trait TileSource: Send + Sync {
    fn render(&self, req: &TileRequest) -> Result<Tile, String>;
}

/// The page's pixel size at `scale`, as `TilePlan` computes it.
fn page_px(points: f32, scale: f32) -> u32 {
    ((points * scale).floor() as u32).max(1)
}

fn clip(req: &TileRequest, page: [f32; 2]) -> Result<PixelRect, String> {
    let (pw, ph) = (page_px(page[0], req.scale()), page_px(page[1], req.scale()));
    if req.x >= pw || req.y >= ph {
        return Err(format!("tile {req:?} is outside the {pw}×{ph} page"));
    }
    let size = u32::from(req.size);
    Ok(PixelRect { x: req.x, y: req.y, width: size.min(pw - req.x), height: size.min(ph - req.y) })
}

/// Gradient plus noise in 4×4 blocks: compresses, but not trivially.
struct Synthetic {
    pages: Vec<[f32; 2]>,
}

impl TileSource for Synthetic {
    fn render(&self, req: &TileRequest) -> Result<Tile, String> {
        let page = *self.pages.get(req.page as usize).ok_or("no such page")?;
        let rect = clip(req, page)?;
        let (pw, ph) = (page_px(page[0], req.scale()), page_px(page[1], req.scale()));
        let mut rgba = Vec::with_capacity(rect.width as usize * rect.height as usize * 4);
        for y in rect.y..rect.y + rect.height {
            for x in rect.x..rect.x + rect.width {
                let mut n = (x / 4).wrapping_mul(0x9e37_79b9) ^ (y / 4).wrapping_mul(0x85eb_ca6b) ^ req.page;
                n ^= n >> 15;
                n = n.wrapping_mul(0x2c1b_3c6d);
                let noise = (n >> 28) as u8; // 0..16
                rgba.extend_from_slice(&[
                    ((x * 239) / pw) as u8 + noise,
                    ((y * 239) / ph) as u8 + noise,
                    (req.page * 37 % 200) as u8 + noise,
                    255,
                ]);
            }
        }
        Ok(Tile { width: rect.width, height: rect.height, rgba })
    }
}

/// A fixture rendered by `laminapdf-render-pdfium` on white.
struct Pdf {
    doc: Mutex<Box<dyn RenderDocument>>,
    pages: Vec<[f32; 2]>,
}

impl Pdf {
    fn open(backend: &PdfiumBackend, path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let source: Arc<dyn ByteSource> = Arc::new(bytes);
        let doc = backend.open(source, &RenderOpenOptions::new()).map_err(|e| e.to_string())?;
        let pages = (0..doc.page_count())
            .map(|i| doc.page(i).map(|p| [p.width, p.height]))
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?;
        Ok(Self { doc: Mutex::new(doc), pages })
    }
}

impl TileSource for Pdf {
    fn render(&self, req: &TileRequest) -> Result<Tile, String> {
        let page = *self.pages.get(req.page as usize).ok_or("no such page")?;
        let rect = clip(req, page)?;
        let request = RenderRequest::page(req.page, req.scale()).with_clip(rect);
        let mut rgba = vec![0; rect.width as usize * rect.height as usize * 4];
        let doc = self.doc.lock().unwrap_or_else(PoisonError::into_inner);
        doc.render(&request, &mut rgba, &CancelToken::new()).map_err(|e| e.to_string())?;
        Ok(Tile { width: rect.width, height: rect.height, rgba })
    }
}

#[derive(Default)]
struct Cache {
    tiles: BTreeMap<TileRequest, (Arc<Tile>, u64)>,
    bytes: usize,
    tick: u64,
}

/// Render and encode time, summed since the last reset.
#[derive(Default, Clone, Copy, Serialize)]
pub struct Timings {
    pub renders: u64,
    pub render_ns: u64,
    pub encodes: u64,
    pub encode_ns: u64,
}

/// Every document, behind one LRU tile cache.
pub struct Tiles {
    sources: Vec<(DocInfo, Box<dyn TileSource>)>,
    cache: Mutex<Cache>,
    budget: usize,
    pub timings: Mutex<Timings>,
}

/// The real-page fixtures: text-heavy, image-heavy, vector-heavy.
pub const FIXTURES: [&str; 3] = [
    "fixtures/golden/examples/flowing_report.pdf",
    "fixtures/corpus/imaging-demo.pdf",
    "fixtures/golden/examples/images_and_vectors.pdf",
];

impl Tiles {
    /// Doc 0 is synthetic (40 A4 pages); docs 1–3 are [`FIXTURES`] under `repo`.
    pub fn open(repo: &Path) -> Result<Self, String> {
        let mut sources: Vec<(DocInfo, Box<dyn TileSource>)> = Vec::new();
        let pages = vec![[595.0, 842.0]; 40];
        sources.push((
            DocInfo { name: "synthetic".into(), pages: pages.clone() },
            Box::new(Synthetic { pages }),
        ));
        let backend = PdfiumBackend::from_env().map_err(|e| format!("{e} (set LAMINA_PDF_PDFIUM_DIR)"))?;
        for rel in FIXTURES {
            let pdf = Pdf::open(&backend, &repo.join(rel))?;
            let name = rel.rsplit('/').next().unwrap_or(rel).to_owned();
            sources.push((DocInfo { name, pages: pdf.pages.clone() }, Box::new(pdf)));
        }
        Ok(Self { sources, cache: Mutex::default(), budget: 768 << 20, timings: Mutex::default() })
    }

    pub fn docs(&self) -> Vec<&DocInfo> {
        self.sources.iter().map(|(info, _)| info).collect()
    }

    /// The tile, from the cache or rendered now.
    pub fn get(&self, req: &TileRequest) -> Result<Arc<Tile>, String> {
        {
            let mut cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
            cache.tick += 1;
            let tick = cache.tick;
            if let Some((tile, used)) = cache.tiles.get_mut(req) {
                *used = tick;
                return Ok(Arc::clone(tile));
            }
        }
        let (_, source) = self.sources.get(req.doc as usize).ok_or("no such document")?;
        let start = Instant::now();
        let tile = Arc::new(source.render(req)?);
        self.record(start, false);
        let mut cache = self.cache.lock().unwrap_or_else(PoisonError::into_inner);
        cache.bytes += tile.rgba.len();
        let tick = cache.tick;
        // Two misses for one tile can race; the second replaces the first.
        if let Some((old, _)) = cache.tiles.insert(*req, (Arc::clone(&tile), tick)) {
            cache.bytes -= old.rgba.len();
        }
        while cache.bytes > self.budget {
            let Some(oldest) = cache.tiles.iter().min_by_key(|(_, (_, used))| *used).map(|(k, _)| *k) else {
                break;
            };
            if let Some((evicted, _)) = cache.tiles.remove(&oldest) {
                cache.bytes -= evicted.rgba.len();
            }
        }
        Ok(tile)
    }

    /// Adds the time since `start` to the render or encode total.
    pub fn record(&self, start: Instant, encode: bool) {
        let ns = u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX);
        let mut t = self.timings.lock().unwrap_or_else(PoisonError::into_inner);
        if encode {
            t.encodes += 1;
            t.encode_ns += ns;
        } else {
            t.renders += 1;
            t.render_ns += ns;
        }
    }
}
