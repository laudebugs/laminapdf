//! The cancel flag shared by long calls and jobs ([ADR 0028] §5). It lives in the lowest core
//! crate so every crate that does long work (the façade's jobs, `laminapdf-render`'s renders) takes
//! the same type; the façade re-exports it as `laminapdf::CancelToken`.
//!
//! [ADR 0028]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0028-facade-rules-jobs-and-cancellation.md

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// A cancel flag shared by the jobs and calls it is passed to ([ADR 0028] §5). Cloning
/// shares the flag: cancel one clone and every holder sees it.
///
/// An atomic flag, not a lock or a cache, so a document holding one stays `Send + !Sync`
/// (ADR 0005 §7). The core polls it at the start of every step, at every budget check and
/// inside long loops; a tripped token makes the call fail with `cancelled`.
///
/// [ADR 0028]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0028-facade-rules-jobs-and-cancellation.md
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    /// A token that is not cancelled.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Cancels every job and call holding this token (or a clone).
    pub fn cancel(&self) {
        // Relaxed is enough: a flag, no data is published through it.
        self.0.store(true, Ordering::Relaxed);
    }

    /// Whether [`CancelToken::cancel`] was called.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::CancelToken;

    #[test]
    fn a_clone_shares_the_flag() {
        let token = CancelToken::new();
        let clone = token.clone();
        assert!(!clone.is_cancelled());
        token.cancel();
        assert!(clone.is_cancelled());
    }
}
