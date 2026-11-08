//! A [`LayoutModel`] for `laminapdf-extract` ([ADR 0030] §6): Docling's Heron document-layout
//! detector (RT-DETRv2) in its official ONNX export, run by ONNX Runtime.
//!
//! Nothing here downloads anything. The app supplies the weights (fetch them with
//! `cargo xtask models fetch docling-layout-heron`, or ship them beside the app) and the path of
//! the ONNX Runtime library (`libonnxruntime.so`, `.dylib` or `onnxruntime.dll`, version 1.31).
//! [`HeronLayout::new`] refuses weights whose SHA-256 isn't the pinned one, so a model under
//! another licence can't be wired in by accident.
//!
//! ```no_run
//! use std::sync::Arc;
//! use laminapdf_extract::{ExtractOptions, ModelSlot, Models};
//! use laminapdf_layout_onnx::HeronLayout;
//!
//! # fn renderer() -> Arc<dyn laminapdf_extract::PageRenderer> { unimplemented!() }
//! let weights = std::fs::read("model.onnx")?;
//! let heron = HeronLayout::new(&weights, "/usr/lib/libonnxruntime.so")?;
//! let options = ExtractOptions::new().with_models(
//!     Models::new().with_layout(ModelSlot::Plugin(Arc::new(heron))).with_renderer(renderer()),
//! );
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md

mod post;
mod pre;

use std::path::Path;
use std::sync::{Mutex, OnceLock};

use laminapdf_extract::{LayoutModel, LayoutRegion, ModelError, ModelInfo, PageRaster};
use laminapdf_syntax::CancelToken;
use ort::environment::Environment;
use ort::session::Session;
use ort::value::Tensor;
use sha2::{Digest as _, Sha256};

pub use post::label;

/// The model's name in `tools/models/models.toml` and `generator.models`.
pub const NAME: &str = "docling-layout-heron";
/// The Hugging Face revision the weights are pinned to.
pub const REVISION: &str = "40bde044036bb181c130ddf6c51792187268748f";
/// The pinned weights' SHA-256.
pub const SHA256: &str = "59c81a3a2923042d85034ffc487f8f47e4854117e879aef89b2b9f728fb4922a";
/// The weights' licence.
pub const LICENCE: &str = "Apache-2.0";

/// The side of the square image the model reads, in pixels.
pub(crate) const SIDE: u32 = 640;

/// Why [`HeronLayout::new`] failed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum LoadError {
    /// The weights aren't the pinned file.
    #[error("the weights' SHA-256 is {found}, not the pinned {SHA256}")]
    WrongWeights {
        /// The SHA-256 of the bytes given.
        found: String,
    },
    /// ONNX Runtime couldn't be loaded, or the model couldn't be read.
    #[error("ONNX Runtime: {0}")]
    Runtime(String),
}

/// Heron, loaded and ready to run. One session serves every call, in turn.
pub struct HeronLayout {
    session: Mutex<Session>,
    threshold: f32,
}

impl std::fmt::Debug for HeronLayout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HeronLayout")
            .field("threshold", &self.threshold)
            .finish_non_exhaustive()
    }
}

impl HeronLayout {
    /// The score a region needs unless [`HeronLayout::with_threshold`] says otherwise, as
    /// Docling uses.
    pub const DEFAULT_THRESHOLD: f32 = 0.3;

    /// Loads ONNX Runtime from `runtime` (once per process; later calls reuse it) and Heron's
    /// `weights`.
    ///
    /// # Errors
    /// [`LoadError::WrongWeights`] for weights other than the pinned file;
    /// [`LoadError::Runtime`] when the library or the model can't be loaded.
    pub fn new(weights: &[u8], runtime: impl AsRef<Path>) -> Result<Self, LoadError> {
        let found = hex(&Sha256::digest(weights));
        if found != SHA256 {
            return Err(LoadError::WrongWeights { found });
        }
        let runtime_error = |e: &dyn std::fmt::Display| LoadError::Runtime(e.to_string());
        let environment = environment(runtime.as_ref())?;
        let session = Session::builder(&environment)
            .and_then(|mut b| b.commit_from_memory(weights))
            .map_err(|e| runtime_error(&e))?;
        Ok(Self {
            session: Mutex::new(session),
            threshold: Self::DEFAULT_THRESHOLD,
        })
    }

    /// Keeps only regions scoring at least `threshold` (0 to 1).
    #[must_use]
    pub const fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold;
        self
    }
}

/// ONNX Runtime allows one environment per process: the first [`HeronLayout::new`] builds it
/// from its library path, later ones share it (and their path is not loaded again).
fn environment(runtime: &Path) -> Result<Environment, LoadError> {
    static ENVIRONMENT: OnceLock<Result<Environment, String>> = OnceLock::new();
    ENVIRONMENT
        .get_or_init(|| {
            ort::init_from(runtime)
                .map_err(|e| e.to_string())?
                .build()
                .map_err(|e| e.to_string())
        })
        .clone()
        .map_err(LoadError::Runtime)
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

fn failed(e: &dyn std::fmt::Display) -> ModelError {
    ModelError::Failed { reason: e.to_string() }
}

impl LayoutModel for HeronLayout {
    fn info(&self) -> ModelInfo {
        ModelInfo::new(NAME, REVISION, SHA256, LICENCE)
    }

    fn detect(&self, page: &PageRaster, cancel: &CancelToken) -> Result<Vec<LayoutRegion>, ModelError> {
        if cancel.is_cancelled() {
            return Err(ModelError::Cancelled);
        }
        let image = pre::chw_640(page);
        let side = i64::from(SIDE);
        let images = Tensor::from_array(([1_i64, 3, side, side], image)).map_err(|e| failed(&e))?;
        let original = vec![i64::from(page.width_px), i64::from(page.height_px)];
        let sizes = Tensor::from_array(([1_i64, 2], original)).map_err(|e| failed(&e))?;
        let mut session = self
            .session
            .lock()
            .map_err(|_| failed(&"the session's lock is poisoned"))?;
        let outputs = session
            .run(ort::inputs!["images" => images, "orig_target_sizes" => sizes])
            .map_err(|e| failed(&e))?;
        if cancel.is_cancelled() {
            return Err(ModelError::Cancelled);
        }
        let extract = |name: &str| outputs.get(name).ok_or_else(|| failed(&format!("no `{name}` output")));
        let (_, labels) = extract("labels")?.try_extract_tensor::<i64>().map_err(|e| failed(&e))?;
        let (_, boxes) = extract("boxes")?.try_extract_tensor::<f32>().map_err(|e| failed(&e))?;
        let (_, scores) = extract("scores")?.try_extract_tensor::<f32>().map_err(|e| failed(&e))?;
        let detections = post::Detections { labels, boxes, scores };
        Ok(post::regions(&detections, page, self.threshold))
    }
}
