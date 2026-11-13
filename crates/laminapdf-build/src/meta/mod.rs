//! Document metadata (ISO 32000-2 §14.3; research 04 §5): the Info dictionary and an XMP
//! packet written from the same values, and the catalog's viewer preferences.

pub(crate) mod info;
pub(crate) mod prefs;
pub(crate) mod xmp;

pub use info::Metadata;
pub use laminapdf_model::{PageLayout, PageMode};
pub use prefs::ViewerPreferences;
