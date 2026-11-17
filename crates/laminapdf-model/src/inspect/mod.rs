//! Read-only inspection for power users and developers: the version and the standards a file
//! claims ([`ConformanceClaims`]), its colour profiles ([`ColorProfiles`]), every object with
//! where it is stored ([`ObjectEntry`]), and which objects refer to which
//! ([`ReferenceIndex`]). Nothing here changes the document.

mod colour;
mod conformance;
mod objects;
mod referrers;

pub use colour::{ColorProfile, ColorProfiles, IccHeader, OutputIntent};
pub use conformance::ConformanceClaims;
pub use objects::{ObjectEntry, ObjectKind};
pub use referrers::{ReferenceIndex, Referrer};
