//! The render tool's shared code, used by its four binaries: the render goldens
//! (`render-golden`), the nightly render diff (`render-diff`), the docs' previews
//! (`docs-previews`) and the annotation appearance parity suite (`annots-parity`). A tool: never
//! a dependency of a crate.
// The binaries' shared code, not an API anyone else calls: each binary reports the errors it
// gets, so their kinds need no `# Errors` section.
#![expect(
    clippy::missing_errors_doc,
    reason = "the binaries' shared code, not a published API"
)]

pub mod annots_compare;
pub mod annots_matrix;
pub mod annots_move;
pub mod compare;
pub mod hayro_render;
pub mod image;
pub mod settings;
pub mod thresholds;
