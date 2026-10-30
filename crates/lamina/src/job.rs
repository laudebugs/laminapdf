//! Jobs, budgets and cancellation ([ADR 0028] §3–§6): long work as a state machine the
//! caller steps, a document revision every step checks, and a shared cancel flag.
//!
//! [ADR 0028]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0028-facade-rules-jobs-and-cancellation.md

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};

use lamina_save::{Save as _, SaveMode, SaveOptions};

use crate::document::{Document, Instance, Saved};
use crate::error::{Error, Result};

/// A cancel flag shared by the jobs and calls it is passed to ([ADR 0028] §5). Cloning
/// shares the flag: cancel one clone and every holder sees it.
///
/// An atomic flag, not a lock or a cache, so the document stays `Send + !Sync` (ADR 0005
/// §7). The core polls it at the start of every step, at every budget check and inside long
/// loops; a tripped token makes the call fail with `cancelled`.
///
/// [ADR 0028]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0028-facade-rules-jobs-and-cancellation.md
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

    /// `Err(cancelled)` once cancelled.
    pub(crate) fn check(&self) -> Result<()> {
        if self.is_cancelled() {
            Err(Error::Cancelled)
        } else {
            Ok(())
        }
    }
}

/// How much work one [`SaveJob::step`] may do, in work units, not time ([ADR 0028] §4): the
/// core has no clock, so the same budget gives the same step boundaries everywhere. One unit
/// is one cheap, indivisible piece of work; each job documents its weights. Edges measure
/// time between steps and adapt the next budget (the crate README records units per
/// millisecond).
///
/// [ADR 0028]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0028-facade-rules-jobs-and-cancellation.md
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Budget {
    units: u32,
}

impl Budget {
    /// No limit: the job runs to completion in one step.
    pub const UNLIMITED: Self = Self { units: u32::MAX };

    /// `units` work units (`u32`, a plain number in JS: ADR 0028 F5). A step always does at
    /// least one unit, so a budget of 0 behaves as 1.
    #[must_use]
    pub const fn units(units: u32) -> Self {
        Self { units }
    }

    /// The number of units.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.units
    }
}

/// What a step returns: progress, or the job's result (Rust-only; edges expose a concrete
/// record per job, ADR 0028 F3).
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
/// [ADR 0028]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0028-facade-rules-jobs-and-cancellation.md
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
}

/// What every job records when it is created (ADR 0028 §3): its token, the document's
/// instance and revision, and whether it has finished.
#[derive(Debug)]
pub(crate) struct JobCore {
    cancel: CancelToken,
    instance: Weak<Instance>,
    revision: u64,
    finished: bool,
}

impl JobCore {
    pub(crate) fn new(doc: &Document, cancel: CancelToken) -> Self {
        Self {
            cancel,
            instance: Arc::downgrade(doc.instance()),
            revision: doc.revision(),
            finished: false,
        }
    }

    /// The checks at the start of every step, before any work: finished, cancelled, the
    /// same document at the same revision.
    pub(crate) fn begin(&mut self, doc: &Document) -> Result<()> {
        if self.finished {
            return Err(Error::JobFinished);
        }
        let outcome = self.cancel.check().and_then(|()| {
            let same = self.instance.ptr_eq(&Arc::downgrade(doc.instance())) && self.revision == doc.revision();
            if same { Ok(()) } else { Err(Error::DocumentChanged) }
        });
        self.ended(outcome)
    }

    /// `result`, finishing the job when it is an error (errors end the job, §3).
    pub(crate) fn ended<T>(&mut self, result: Result<T>) -> Result<T> {
        if result.is_err() {
            self.finished = true;
        }
        result
    }

    pub(crate) fn finish(&mut self) {
        self.finished = true;
    }

    pub(crate) const fn cancel(&self) -> &CancelToken {
        &self.cancel
    }
}

/// Where a [`SaveJob`] is.
#[derive(Debug)]
enum Stage {
    /// Nothing done yet: the next unit plans the save.
    Plan,
    /// A full rewrite of a document with a base: parsing the base's objects ahead of the
    /// write, one unit per object (they are cached, so the write only serialises).
    Parse { numbers: Vec<u32>, next: usize },
    /// The next unit writes the output and rebases the document.
    Write,
}

/// A save as a resumable job ([ADR 0028] §3): created by
/// [`Document::save_job`](crate::Document::save_job), stepped with the document it was
/// created from.
///
/// **Units:** planning is 1 unit (it also checks that a save may start); for a full rewrite
/// of an opened document, each object of the base that the session didn't change is 1 unit
/// (parsed and cached, [`Phase::Parse`]); writing and rebasing is 1 indivisible unit
/// ([`Phase::Write`]): `lamina-save`'s writers are not resumable yet, so a
/// step that reaches the write overshoots its budget by the serialisation of the whole file.
/// That is fast next to parsing (see the crate README's measurements), and cancellation
/// lands before it, not inside it.
///
/// Every step first checks that the job hasn't finished (`job-finished`), its token
/// (`cancelled`), and that the document is the same instance at the same revision
/// (`document-changed`): an op committed between two steps fails the save, the document
/// untouched. The save's own rebase bumps the revision once, as its last act.
///
/// ```
/// use lamina::{Budget, Document, SaveMode, SaveOptions, Step};
///
/// # let bytes = include_bytes!("../../../fixtures/corpus/hello.pdf").to_vec();
/// let mut doc = Document::open_bytes(bytes, Default::default())?;
/// let options = SaveOptions::new().with_mode(SaveMode::FullRewrite);
/// let mut job = doc.save_job(&options);
/// let saved = loop {
///     match job.step(&mut doc, Budget::units(64))? {
///         Step::Done(saved) => break saved,
///         Step::Progress { .. } => {} // yield to the event loop, report progress
///         _ => {}
///     }
/// };
/// assert!(saved.bytes.starts_with(b"%PDF-"));
/// # Ok::<(), lamina::Error>(())
/// ```
///
/// [ADR 0028]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0028-facade-rules-jobs-and-cancellation.md
#[derive(Debug)]
pub struct SaveJob {
    core: JobCore,
    options: SaveOptions,
    stage: Stage,
}

impl SaveJob {
    pub(crate) fn new(doc: &Document, options: SaveOptions, cancel: CancelToken) -> Self {
        Self {
            core: JobCore::new(doc, cancel),
            options,
            stage: Stage::Plan,
        }
    }

    /// The job's cancel token (cancel it from anywhere that holds a clone).
    #[must_use]
    pub fn cancel_token(&self) -> CancelToken {
        self.core.cancel().clone()
    }

    /// Runs at most `budget` work units (at least one), then returns progress or the result.
    ///
    /// # Errors
    /// `job-finished`, `cancelled` or `document-changed` (each finishes the job, the document
    /// untouched), or the save's own errors (`requires-rewrite`,
    /// `would-invalidate-signatures`, `encrypted-base`, reading errors), which finish it too.
    pub fn step(&mut self, doc: &mut Document, budget: Budget) -> Result<Step<Saved>> {
        self.core.begin(doc)?;
        let outcome = self.run(doc, budget.get().max(1));
        if matches!(outcome, Ok(Step::Done(_))) {
            self.core.finish();
        }
        self.core.ended(outcome)
    }

    fn run(&mut self, doc: &mut Document, mut units: u32) -> Result<Step<Saved>> {
        loop {
            self.core.cancel().check()?;
            if units == 0 {
                return Ok(self.progress());
            }
            match &mut self.stage {
                Stage::Plan => {
                    units -= 1;
                    // `Save::plan` doesn't check that a save may start; the write would
                    // refuse only at the end, after the parse units were spent.
                    let ops = doc.ops();
                    if ops.save_state() == lamina_save::SaveState::SavedNotRebased {
                        return Err(lamina_ops::Error::SavedNotRebased.into());
                    }
                    if ops.in_transaction() {
                        return Err(lamina_ops::Error::NestedTransaction.into());
                    }
                    let plan = ops.plan(&self.options)?;
                    // Only the base's objects need parsing: overlay objects (new or changed)
                    // are already in memory.
                    let cos = ops.cos();
                    let numbers: Vec<u32> = if plan.mode() == SaveMode::FullRewrite {
                        cos.object_numbers()
                            .filter(|&n| cos.overlay().get(n).is_none())
                            .collect()
                    } else {
                        Vec::new()
                    };
                    self.stage = if numbers.is_empty() {
                        Stage::Write
                    } else {
                        Stage::Parse { numbers, next: 0 }
                    };
                }
                Stage::Parse { numbers, next } => {
                    while units > 0 {
                        let Some(&number) = numbers.get(*next) else { break };
                        self.core.cancel().check()?;
                        // A broken object is the writer's to report (or to drop, as the
                        // tolerant reader does); only errors that must propagate end here.
                        lamina_cos::tolerant(doc.ops().cos().object(number))?;
                        *next += 1;
                        units -= 1;
                    }
                    if *next >= numbers.len() {
                        self.stage = Stage::Write;
                    }
                }
                Stage::Write => return doc.write(self.options).map(Step::Done),
            }
        }
    }

    fn progress(&self) -> Step<Saved> {
        match &self.stage {
            Stage::Plan => Step::Progress {
                done: 0,
                total: None,
                phase: Phase::Parse,
            },
            Stage::Parse { numbers, next } => Step::Progress {
                done: u32::try_from(*next).unwrap_or(u32::MAX),
                total: Some(u32::try_from(numbers.len()).unwrap_or(u32::MAX)),
                phase: Phase::Parse,
            },
            Stage::Write => Step::Progress {
                done: 0,
                total: Some(1),
                phase: Phase::Write,
            },
        }
    }
}
