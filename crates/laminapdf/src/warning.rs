//! [`Warning`] and the [`WarningCursor`] (ADR 0028 F8).

use laminapdf_cos::{ErrorKind, Location};
use serde::{Serialize, Serializer};

use crate::codes;
use crate::data;
use crate::{Diagnostic, WarningCode};

/// A recoverable problem lamina repaired or worked around ([ADR 0003] §2), with the same
/// accessors and serialisation as [`Error`](crate::Error) (no `didYouMean`).
///
/// [ADR 0003]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0003-reader-tolerance-and-limits.md
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Warning {
    /// Raised while reading the document (`laminapdf-cos`, `laminapdf-model`, `laminapdf-content`).
    Document(laminapdf_cos::Warning),
    /// Raised while embedding a font (`laminapdf-text`).
    #[cfg(feature = "extract")]
    Font(laminapdf_text::Warning),
}

impl Warning {
    /// The stable kebab-case code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.warning_code().as_str()
    }

    /// The code as the generated [`WarningCode`], from which [`code`](Warning::code),
    /// [`kind`](Warning::kind), [`hint`](Warning::hint), [`docs_url`](Warning::docs_url) and
    /// the message are read. The build fails if any code a wrapped crate declares is missing
    /// from the table.
    #[must_use]
    pub const fn warning_code(&self) -> WarningCode {
        match self {
            Self::Document(w) => codes::warning_code(w.code()),
            #[cfg(feature = "extract")]
            Self::Font(w) => codes::warning_code(w.code()),
        }
    }

    /// The kind ([ADR 0026] §2), from the generated [`WarningCode`](crate::WarningCode) table.
    ///
    /// [ADR 0026]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        self.warning_code().kind()
    }

    /// Where the problem is ([ADR 0026] §1.1).
    ///
    /// [ADR 0026]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
    #[must_use]
    pub fn location(&self) -> Location {
        match self {
            Self::Document(w) => w.location(),
            #[cfg(feature = "extract")]
            Self::Font(w) => w.location(),
        }
    }

    /// The code's hint from `fixtures/codes.json`.
    #[must_use]
    pub fn hint(&self) -> Option<String> {
        codes::fill_hint(self.warning_code().hint(), None)
    }

    /// The code's explain page.
    #[must_use]
    pub fn docs_url(&self) -> String {
        self.warning_code().docs_url()
    }

    /// The [ADR 0026] §1 object (see [`Error::diagnostic`](crate::Error::diagnostic)). The
    /// `message` is the code's fixed summary from `fixtures/codes.json`; it never quotes the
    /// document.
    ///
    /// [ADR 0026]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
    #[must_use]
    pub fn diagnostic(&self) -> Diagnostic {
        Diagnostic::warning(self.warning_code())
            .with_hint(self.hint())
            .with_location(self.location())
            .with_data(data::of_warning(self))
    }
}

impl Serialize for Warning {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.diagnostic().serialize(serializer)
    }
}

impl From<laminapdf_cos::Warning> for Warning {
    fn from(w: laminapdf_cos::Warning) -> Self {
        Self::Document(w)
    }
}

#[cfg(feature = "extract")]
impl From<laminapdf_text::Warning> for Warning {
    fn from(w: laminapdf_text::Warning) -> Self {
        Self::Font(w)
    }
}

/// The warnings a document had raised when [`Document::warnings`](crate::Document::warnings)
/// was called, read in batches ([ADR 0028] F8: cursors, not iterators). An owned snapshot:
/// warnings raised later need a new cursor.
///
/// ```
/// # let doc = laminapdf::Document::open_bytes(include_bytes!("../../../fixtures/corpus/hello-broken.pdf").to_vec(), Default::default())?;
/// let mut cursor = doc.warnings();
/// let first = cursor.next_batch(10);
/// assert_eq!(first[0].code(), "xref-repaired");
/// # Ok::<(), laminapdf::Error>(())
/// ```
///
/// [ADR 0028]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0028-facade-rules-jobs-and-cancellation.md
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WarningCursor {
    items: Vec<Warning>,
    next: usize,
}

impl WarningCursor {
    pub(crate) const fn new(items: Vec<Warning>) -> Self {
        Self { items, next: 0 }
    }

    /// Up to `max` more warnings, oldest first; empty once all were read.
    pub fn next_batch(&mut self, max: u32) -> Vec<Warning> {
        let take = usize::try_from(max).unwrap_or(usize::MAX);
        let batch: Vec<Warning> = self.items.iter().skip(self.next).take(take).cloned().collect();
        self.next += batch.len();
        batch
    }

    /// How many warnings are left to read.
    #[must_use]
    pub fn remaining(&self) -> u32 {
        u32::try_from(self.items.len().saturating_sub(self.next)).unwrap_or(u32::MAX)
    }
}
