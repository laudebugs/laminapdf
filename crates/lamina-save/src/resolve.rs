//! Resolving the save mode from the constraint ([ADR 0007] §3).
//!
//! [ADR 0007]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0007-save-planner.md

use crate::constraint::{ConstraintKind, RewriteReason, SaveConstraint, SignatureInfo};
use crate::error::{Result, SaveError};
use crate::options::{SaveMode, SaveOptions};

/// The mode a save takes and the signatures it invalidates, from the constraint and the
/// options: [ADR 0007] §3's table, after the encrypted-base refusal.
///
/// | `kind()` | `Auto` | `Incremental` | `FullRewrite` |
/// | --- | --- | --- | --- |
/// | `Free` | `Incremental` with a base, else `FullRewrite` | OK | OK |
/// | `AppendOnly` | `Incremental` | OK | refused unless overridden |
/// | `RequiresRewrite` | `FullRewrite` | refused | OK |
/// | `Conflict` | refused unless overridden | refused unless overridden, and always with rewrite reasons | refused unless overridden |
///
/// An encrypted base ([`RewriteReason::EncryptedBase`]) is refused in every mode without
/// `remove_encryption` ([`SaveError::EncryptedBase`]), before the table.
///
/// [ADR 0007]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0007-save-planner.md
#[expect(clippy::match_same_arms, reason = "one arm per cell of the ADR's table")]
pub(crate) fn resolve(
    constraint: &SaveConstraint,
    has_base: bool,
    options: SaveOptions,
) -> Result<(SaveMode, Vec<SignatureInfo>)> {
    let reasons = &constraint.rewrite_reasons;
    if reasons.contains(&RewriteReason::EncryptedBase) && !options.remove_encryption() {
        return Err(SaveError::EncryptedBase);
    }
    let overridden = options.invalidate_signatures();
    let refuse = || SaveError::WouldInvalidateSignatures {
        signatures: constraint.signatures.clone(),
        rewrite_reasons: reasons.clone(),
        violations: constraint.violations.clone(),
    };
    let requires = || SaveError::RequiresRewrite {
        reasons: reasons.clone(),
    };
    let mode = match (constraint.kind(), options.mode()) {
        (ConstraintKind::Free, SaveMode::Auto) if has_base => SaveMode::Incremental,
        (ConstraintKind::Free, SaveMode::Auto) => SaveMode::FullRewrite,
        (ConstraintKind::Free, requested) => requested,
        (ConstraintKind::AppendOnly, SaveMode::Auto | SaveMode::Incremental) => SaveMode::Incremental,
        (ConstraintKind::AppendOnly, _) if overridden => SaveMode::FullRewrite,
        (ConstraintKind::AppendOnly, _) => return Err(refuse()),
        (ConstraintKind::RequiresRewrite, SaveMode::Incremental) => return Err(requires()),
        (ConstraintKind::RequiresRewrite, _) => SaveMode::FullRewrite,
        (ConstraintKind::Conflict, _) if !overridden => return Err(refuse()),
        (ConstraintKind::Conflict, SaveMode::Incremental) if !reasons.is_empty() => return Err(requires()),
        (ConstraintKind::Conflict, SaveMode::Incremental | SaveMode::Auto) if reasons.is_empty() => {
            SaveMode::Incremental
        }
        (ConstraintKind::Conflict, _) => SaveMode::FullRewrite,
    };
    Ok((mode, invalidated(constraint, mode)))
}

/// The signatures a save in `mode` breaks: a full rewrite moves every byte, so all of them;
/// an incremental update only those whose restrictions a pending change violates.
fn invalidated(constraint: &SaveConstraint, mode: SaveMode) -> Vec<SignatureInfo> {
    if mode == SaveMode::FullRewrite {
        return constraint.signatures.clone();
    }
    let mut out: Vec<SignatureInfo> = Vec::new();
    for violation in &constraint.violations {
        if !out.contains(&violation.signature) {
            out.push(violation.signature.clone());
        }
    }
    out
}
