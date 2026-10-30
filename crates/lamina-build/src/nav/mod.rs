//! Navigation (ISO 32000-2 §12.3–12.4; research 04 §1): destinations and link targets,
//! named destinations, the outline and page labels.

pub(crate) mod dest;
pub(crate) mod labels;
pub(crate) mod outline;

pub use dest::{Fit, LinkTarget};
pub use labels::PageLabel;
pub use outline::OutlineEntry;
