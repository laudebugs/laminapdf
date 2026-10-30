//! [`Warning`] and the [`WarningCursor`] (ADR 0028 F8).

use lamina_cos::{ErrorKind, Location};
use serde::{Serialize, Serializer};

use crate::codes;
use crate::data;
use crate::diagnostic::Diagnostic;

/// A recoverable problem lamina repaired or worked around ([ADR 0003] §2), with the same
/// accessors and serialisation as [`Error`](crate::Error) (no `didYouMean`).
///
/// [ADR 0003]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0003-reader-tolerance-and-limits.md
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Warning {
    /// Raised while reading the document (`lamina-cos`, `lamina-model`, `lamina-content`).
    Document(lamina_cos::Warning),
    /// Raised while embedding a font (`lamina-text`).
    #[cfg(feature = "extract")]
    Font(lamina_text::Warning),
}

impl Warning {
    /// The stable kebab-case code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Document(w) => w.code(),
            #[cfg(feature = "extract")]
            Self::Font(w) => w.code(),
        }
    }

    /// The kind ([ADR 0026] §2).
    ///
    /// [ADR 0026]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
    #[must_use]
    pub const fn kind(&self) -> ErrorKind {
        match self {
            Self::Document(w) => w.kind(),
            #[cfg(feature = "extract")]
            Self::Font(w) => w.kind(),
        }
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
        codes::hint(self.code(), None)
    }

    /// The code's explain page.
    #[must_use]
    pub fn docs_url(&self) -> String {
        codes::docs_url(self.code())
    }

    /// The [ADR 0026] §1 object (see [`Error::diagnostic`](crate::Error::diagnostic)). The
    /// `message` is the code's fixed summary from `fixtures/codes.json`; it never quotes the
    /// document.
    ///
    /// [ADR 0026]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0026-error-and-diagnostic-contract.md
    #[must_use]
    pub fn diagnostic(&self) -> Diagnostic {
        Diagnostic {
            code: self.code().to_owned(),
            kind: self.kind(),
            message: codes::summary(self.code()).unwrap_or(self.code()).to_owned(),
            hint: self.hint(),
            did_you_mean: Vec::new(),
            location: self.location(),
            data: data::of_warning(self),
            docs_url: self.docs_url(),
        }
    }
}

impl Serialize for Warning {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.diagnostic().serialize(serializer)
    }
}

impl From<lamina_cos::Warning> for Warning {
    fn from(w: lamina_cos::Warning) -> Self {
        Self::Document(w)
    }
}

#[cfg(feature = "extract")]
impl From<lamina_text::Warning> for Warning {
    fn from(w: lamina_text::Warning) -> Self {
        Self::Font(w)
    }
}

/// The warnings a document had raised when [`Document::warnings`](crate::Document::warnings)
/// was called, read in batches ([ADR 0028] F8: cursors, not iterators). An owned snapshot:
/// warnings raised later need a new cursor.
///
/// ```
/// # let doc = lamina::Document::open_bytes(include_bytes!("../../../fixtures/corpus/hello-broken.pdf").to_vec(), Default::default())?;
/// let mut cursor = doc.warnings();
/// let first = cursor.next_batch(10);
/// assert_eq!(first[0].code(), "xref-repaired");
/// # Ok::<(), lamina::Error>(())
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
