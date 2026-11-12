//! Jobs ([ADR 0028] §3–§6): the budget a step may spend and what a step returns, shared by
//! every core crate's jobs (`laminapdf-extract`) and the façade's.
//!
//! [ADR 0028]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0028-facade-rules-jobs-and-cancellation.md

/// How much work one step of a job may do, in work units, not time ([ADR 0028] §4): the
/// core has no clock, so the same budget gives the same step boundaries everywhere. One unit
/// is one cheap, indivisible piece of work; each job documents its weights. Edges measure
/// time between steps and adapt the next budget.
///
/// [ADR 0028]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0028-facade-rules-jobs-and-cancellation.md
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Budget {
    units: u32,
}

impl Budget {
    /// No limit: the job runs to completion in one step.
    ///
    /// Added in 0.2.0.
    pub const UNLIMITED: Self = Self { units: u32::MAX };

    /// `units` work units (`u32`, a plain number in JS: ADR 0028 F5). A step always does at
    /// least one unit, so a budget of 0 behaves as 1.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn units(units: u32) -> Self {
        Self { units }
    }

    /// The number of units.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.units
    }
}

/// What a step returns: progress, or the job's result (Rust-only; edges expose a concrete
/// record per job, ADR 0028 F3).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Step<T> {
    /// The job is not done: `done` of `total` units of its current phase (`total` `None`
    /// when unknown).
    Progress {
        /// Units done in this phase.
        done: u32,
        /// Units this phase has, when known.
        total: Option<u32>,
        /// What the job is doing.
        phase: Phase,
    },
    /// The job finished with its result; stepping again fails with `job-finished`.
    Done(T),
}

/// What a job is doing ([ADR 0028] §3).
///
/// [ADR 0028]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0028-facade-rules-jobs-and-cancellation.md
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Phase {
    /// Reading and parsing objects.
    Parse,
    /// Laying out text.
    Layout,
    /// Extracting content.
    Extract,
    /// Rendering.
    Render,
    /// Writing output.
    Write,
    /// Signing.
    Sign,
    /// Waiting for, or running, a layout model or OCR engine.
    Model,
}
