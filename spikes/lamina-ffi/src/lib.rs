//! Spike L6a: the `laminapdf` façade bound with `UniFFI` 0.32 for Swift and Kotlin.
//!
//! Every export is a thin wrapper over `laminapdf` (ADR 0028 §1: the edge crate holds binding
//! code only). The questions it answers are in the README: does the façade bind without
//! changes, where the lock lives (ADR 0028 §2: here, a `Mutex` per document), how jobs and
//! cancellation look from Swift and Kotlin, whether panics surface as foreign errors, and
//! how big the libraries are.
//!
//! Throwaway quality, but no `unwrap`/`expect` on exported paths (`docs/guidelines/rust.md`).

#![allow(
    clippy::result_large_err,
    reason = "UniFFI error variants hold their fields by value (no Box); the error crosses the FFI \
              serialised, so its size in Rust only matters on the error path"
)]

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, TryLockError};
use std::task::{Context, Poll, Waker};

use laminapdf::{
    Budget, CancelToken, Document, IncrementalOutput, InfoPatch, Op, OpenOptions, SaveMode, SaveOptions, SetInfo, Step,
};

uniffi::setup_scaffolding!();

// ---------------------------------------------------------------------------------------------
// Errors (ADR 0026): one error type with the stable code, kind, message, hint and docs URL.
// ---------------------------------------------------------------------------------------------

/// A lamina error as Swift `Error` / Kotlin `Exception` (ADR 0026's fields, flattened).
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum FfiError {
    /// Every façade error: `code` is stable and kebab-case (`not-pdf`, `cancelled`, …).
    #[error("{code}: {detail}")]
    Lamina {
        /// The stable code (ADR 0026 §3).
        code: String,
        /// One of the nine kinds (`input`, `usage`, `cancelled`, …; ADR 0026 §2).
        kind: String,
        /// ADR 0026's `message`: the English message (never contains document text). Named
        /// `detail` because a field called `message` clashes with `Throwable.message` in the
        /// generated Kotlin exception (UniFFI 0.32 doesn't rename it).
        detail: String,
        /// What to do about it, when known.
        hint: Option<String>,
        /// The error page.
        docs_url: String,
        /// The whole ADR 0026 object as JSON (location and `data` included).
        json: String,
    },
    /// The document's lock was poisoned by a panic in an earlier call.
    #[error("the document is unusable after an earlier panic")]
    Poisoned,
    /// The façade returned something this binding doesn't know yet (a newer `laminapdf`).
    #[error("unsupported by this binding: {what}")]
    Unsupported {
        /// What it was.
        what: String,
    },
}

impl From<laminapdf::Error> for FfiError {
    fn from(e: laminapdf::Error) -> Self {
        let d = e.diagnostic();
        Self::Lamina {
            code: d.code.clone(),
            kind: d.kind.as_str().to_owned(),
            detail: d.message.clone(),
            hint: d.hint.clone(),
            docs_url: d.docs_url.clone(),
            json: d.to_json().to_string(),
        }
    }
}

type FfiResult<T> = Result<T, FfiError>;

// ---------------------------------------------------------------------------------------------
// Records (ADR 0028 F5: owned records of strings and numbers).
// ---------------------------------------------------------------------------------------------

/// One page's facts: top-left points of the displayed `CropBox` (ADR 0027).
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct FfiPageInfo {
    /// The 0-based page index.
    pub index: u32,
    /// The stable page ID (`p12.0`).
    pub id: String,
    /// Displayed width in points.
    pub width: f64,
    /// Displayed height in points.
    pub height: f64,
    /// `/Rotate`: 0, 90, 180 or 270.
    pub rotation: u16,
    /// The page label, when the document defines one.
    pub label: Option<String>,
}

/// The Info dictionary's text entries.
#[derive(Debug, Clone, PartialEq, Eq, Default, uniffi::Record)]
pub struct FfiDocumentInfo {
    /// `/Title`.
    pub title: Option<String>,
    /// `/Author`.
    pub author: Option<String>,
    /// `/Subject`.
    pub subject: Option<String>,
    /// `/Keywords`.
    pub keywords: Option<String>,
    /// `/Creator`.
    pub creator: Option<String>,
    /// `/Producer`.
    pub producer: Option<String>,
}

/// Which Info entry [`FfiDocument::set_info`] writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiInfoKey {
    /// `/Title`.
    Title,
    /// `/Author`.
    Author,
    /// `/Subject`.
    Subject,
    /// `/Keywords`.
    Keywords,
    /// `/Creator`.
    Creator,
    /// `/Producer`.
    Producer,
}

/// How to save.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum FfiSaveMode {
    /// Incremental when allowed, else a full rewrite (ADR 0007 §3).
    Auto,
    /// Append a revision; fails when not allowed.
    Incremental,
    /// Rewrite the whole file.
    FullRewrite,
}

/// Save options (a subset of `laminapdf::SaveOptions`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct FfiSaveOptions {
    /// The mode.
    pub mode: FfiSaveMode,
    /// For incremental saves: return only the appended section instead of the whole file.
    pub appended_only: bool,
}

/// A finished save.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FfiSaved {
    /// The file (or only the appended section with `appended_only`).
    pub bytes: Vec<u8>,
    /// The mode the writer used (`Incremental` or `FullRewrite`; `None` for a mode this
    /// binding doesn't know yet).
    pub mode: Option<FfiSaveMode>,
    /// Where the appended revision starts (incremental saves).
    pub append_offset: Option<u64>,
}

/// One warning (ADR 0026: the same fields as errors; here the code and the JSON object).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FfiWarning {
    /// The stable code.
    pub code: String,
    /// The whole ADR 0026 object as JSON.
    pub json: String,
}

/// What a save-job step returns (ADR 0028 F3: a concrete record per job, no generics).
#[derive(Debug, Clone, PartialEq, uniffi::Enum)]
pub enum FfiSaveStep {
    /// Not done: `done` of `total` units of `phase`.
    Progress {
        /// Units done in this phase.
        done: u32,
        /// Units this phase has, when known.
        total: Option<u32>,
        /// What the job is doing (`write`, `parse`, …).
        phase: String,
    },
    /// Finished with the save's result.
    Done {
        /// The output.
        saved: FfiSaved,
    },
}

fn save_options(o: FfiSaveOptions) -> SaveOptions {
    let mode = match o.mode {
        FfiSaveMode::Auto => SaveMode::Auto,
        FfiSaveMode::Incremental => SaveMode::Incremental,
        FfiSaveMode::FullRewrite => SaveMode::FullRewrite,
    };
    let output = if o.appended_only {
        IncrementalOutput::AppendedOnly
    } else {
        IncrementalOutput::Whole
    };
    SaveOptions::new().with_mode(mode).with_output(output)
}

/// The mode the writer used. `Auto` is a request, never a result, so a mode this list doesn't
/// know is `None`, not a guess (and not an error: the save happened, and its bytes must reach
/// the caller).
fn saved(s: laminapdf::Saved) -> FfiSaved {
    let mode = match s.report.mode() {
        SaveMode::Incremental => Some(FfiSaveMode::Incremental),
        SaveMode::FullRewrite => Some(FfiSaveMode::FullRewrite),
        _ => None,
    };
    FfiSaved {
        append_offset: s.report.append_offset(),
        bytes: s.bytes,
        mode,
    }
}

/// Phase names, spelled out so they match `laminapdf-wasm`'s `phase_name` (crates/laminapdf-wasm/src/dto.rs).
fn phase(p: laminapdf::Phase) -> String {
    match p {
        laminapdf::Phase::Parse => "parse",
        laminapdf::Phase::Layout => "layout",
        laminapdf::Phase::Extract => "extract",
        laminapdf::Phase::Render => "render",
        laminapdf::Phase::Write => "write",
        laminapdf::Phase::Sign => "sign",
        // A phase added to the non-exhaustive enum before this list learns it.
        _ => "other",
    }
    .to_owned()
}

// ---------------------------------------------------------------------------------------------
// The document handle. `laminapdf::Document` is `Send + !Sync`; UniFFI objects must be
// `Send + Sync`, so the lock lives here (ADR 0028 §2).
// ---------------------------------------------------------------------------------------------

/// An open PDF (`Arc` handle; methods may be called from any thread).
#[derive(uniffi::Object)]
pub struct FfiDocument {
    inner: Mutex<Document>,
    /// Async calls waiting for the lock: woken when a guard is dropped, so they wait without
    /// polling (ADR 0029 §4).
    waiters: Wakers,
}

/// Wakers to wake once: a lock release, or a cancel.
#[derive(Default)]
struct Wakers(Mutex<Vec<Waker>>);

impl Wakers {
    fn register(&self, waker: &Waker) {
        if let Ok(mut list) = self.0.lock()
            && !list.iter().any(|w| w.will_wake(waker))
        {
            list.push(waker.clone());
        }
    }

    fn wake_all(&self) {
        let list = self.0.lock().map(|mut l| std::mem::take(&mut *l)).unwrap_or_default();
        for w in list {
            w.wake();
        }
    }
}

/// The document lock's guard: releasing it wakes the async calls waiting for it.
struct DocGuard<'a> {
    guard: Option<MutexGuard<'a, Document>>,
    waiters: &'a Wakers,
}

impl std::ops::Deref for DocGuard<'_> {
    type Target = Document;
    fn deref(&self) -> &Document {
        // `guard` is `Some` until `drop`.
        self.guard.as_deref().unwrap_or_else(|| unreachable!())
    }
}

impl std::ops::DerefMut for DocGuard<'_> {
    fn deref_mut(&mut self) -> &mut Document {
        self.guard.as_deref_mut().unwrap_or_else(|| unreachable!())
    }
}

impl Drop for DocGuard<'_> {
    fn drop(&mut self) {
        drop(self.guard.take());
        self.waiters.wake_all();
    }
}

impl FfiDocument {
    fn lock(&self) -> FfiResult<DocGuard<'_>> {
        let guard = self.inner.lock().map_err(|_| FfiError::Poisoned)?;
        Ok(DocGuard {
            guard: Some(guard),
            waiters: &self.waiters,
        })
    }

    /// Takes the lock without blocking: `Pending` (with `cx`'s waker registered, woken when
    /// the holder releases it or `cancel` fires) while another call holds it.
    fn poll_lock(&self, cx: &Context<'_>, cancel: &FfiCancelToken) -> Poll<FfiResult<DocGuard<'_>>> {
        // Register before trying, so a release between the try and the registration can't be
        // missed; a spurious wake only costs one more try.
        self.waiters.register(cx.waker());
        cancel.waiters.register(cx.waker());
        if cancel.is_cancelled() {
            return Poll::Ready(Err(laminapdf::Error::Cancelled.into()));
        }
        match self.inner.try_lock() {
            Ok(guard) => Poll::Ready(Ok(DocGuard {
                guard: Some(guard),
                waiters: &self.waiters,
            })),
            Err(TryLockError::WouldBlock) => Poll::Pending,
            Err(TryLockError::Poisoned(_)) => Poll::Ready(Err(FfiError::Poisoned)),
        }
    }

    fn wrap(doc: Document) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(doc),
            waiters: Wakers::default(),
        })
    }
}

fn open_options(password: Option<String>) -> OpenOptions {
    match password {
        Some(p) => OpenOptions::default().with_password(p.into_bytes()),
        None => OpenOptions::default(),
    }
}

#[uniffi::export]
impl FfiDocument {
    /// Opens a document from its bytes (owned: the foreign bytes are copied into a `Vec<u8>`
    /// on the way in, and the document keeps them).
    ///
    /// # Errors
    /// `not-pdf`, `password-required`, `wrong-password`, `limit-exceeded`, …
    #[uniffi::constructor]
    pub fn open(bytes: Vec<u8>, password: Option<String>) -> FfiResult<Arc<Self>> {
        Ok(Self::wrap(Document::open_bytes(bytes, open_options(password))?))
    }

    /// Opens a document from borrowed bytes (`UniFFI` 0.32's zero-copy `&[u8]` input): the
    /// foreign buffer is read in place for the call, and copied once here because the
    /// document must own its bytes beyond the call.
    ///
    /// # Errors
    /// As [`FfiDocument::open`].
    #[uniffi::constructor]
    pub fn open_borrowed(bytes: &[u8], password: Option<String>) -> FfiResult<Arc<Self>> {
        Ok(Self::wrap(Document::open_bytes(
            bytes.to_vec(),
            open_options(password),
        )?))
    }

    /// The number of pages.
    ///
    /// # Errors
    /// Reading errors.
    pub fn page_count(&self) -> FfiResult<u32> {
        Ok(self.lock()?.page_count()?)
    }

    /// Every page's facts, in order.
    ///
    /// # Errors
    /// Reading errors.
    pub fn page_infos(&self) -> FfiResult<Vec<FfiPageInfo>> {
        let infos = self.lock()?.page_infos()?;
        Ok(infos
            .into_iter()
            .map(|p| FfiPageInfo {
                index: p.index,
                id: p.id.to_string(),
                width: p.width,
                height: p.height,
                rotation: p.rotation,
                label: p.label,
            })
            .collect())
    }

    /// The Info dictionary's text entries.
    ///
    /// # Errors
    /// Reading errors.
    pub fn info(&self) -> FfiResult<FfiDocumentInfo> {
        let i = self.lock()?.info()?;
        Ok(FfiDocumentInfo {
            title: i.title,
            author: i.author,
            subject: i.subject,
            keywords: i.keywords,
            creator: i.creator,
            producer: i.producer,
        })
    }

    /// Sets (or with `None` removes) one Info entry, as one undoable transaction through the
    /// op log. No `/ModDate` is written, so the output stays byte-deterministic.
    ///
    /// # Errors
    /// `saved-not-rebased` and reading errors.
    pub fn set_info(&self, key: FfiInfoKey, value: Option<String>) -> FfiResult<()> {
        let v = value.as_deref();
        let patch = match key {
            FfiInfoKey::Title => InfoPatch::new().title(v),
            FfiInfoKey::Author => InfoPatch::new().author(v),
            FfiInfoKey::Subject => InfoPatch::new().subject(v),
            FfiInfoKey::Keywords => InfoPatch::new().keywords(v),
            FfiInfoKey::Creator => InfoPatch::new().creator(v),
            FfiInfoKey::Producer => InfoPatch::new().producer(v),
        };
        let _ = self.lock()?.apply(vec![Op::from(SetInfo::patch(patch))])?;
        Ok(())
    }

    /// Undoes the newest transaction; `false` when there was nothing to undo.
    ///
    /// # Errors
    /// `saved-not-rebased` and reading errors.
    pub fn undo(&self) -> FfiResult<bool> {
        Ok(self.lock()?.undo()?.is_some())
    }

    /// Whether there are edits no save has written yet.
    pub fn has_unsaved_changes(&self) -> FfiResult<bool> {
        Ok(self.lock()?.has_unsaved_changes())
    }

    /// The warnings raised so far, oldest first.
    pub fn warnings(&self) -> FfiResult<Vec<FfiWarning>> {
        let mut cursor = self.lock()?.warnings();
        Ok(cursor
            .next_batch(u32::MAX)
            .into_iter()
            .map(|w| FfiWarning {
                code: w.code().to_owned(),
                json: w.diagnostic().to_json().to_string(),
            })
            .collect())
    }

    /// Saves and rebases onto the output, to completion (cancel through `cancel`).
    ///
    /// # Errors
    /// `cancelled`, `requires-rewrite`, `would-invalidate-signatures`, `encrypted-base`, …
    pub fn save(&self, options: FfiSaveOptions, cancel: Arc<FfiCancelToken>) -> FfiResult<FfiSaved> {
        Ok(saved(self.lock()?.save_with(&save_options(options), &cancel.token)?))
    }

    /// A save as a resumable job (ADR 0028 §3): step it with a budget, cancel it from any
    /// thread.
    pub fn save_job(self: Arc<Self>, options: FfiSaveOptions) -> FfiResult<Arc<FfiSaveJob>> {
        let token = CancelToken::new();
        let job = self.lock()?.save_job_with(&save_options(options), &token);
        Ok(Arc::new(FfiSaveJob {
            doc: self,
            job: Mutex::new(job),
            token,
        }))
    }
}

// Feature-gated exports need their own `#[uniffi::export]` block with the `cfg` on the whole
// block: a `#[cfg]` on one method inside an exported block is not honoured by UniFFI 0.32's
// macro (the scaffolding still references the method, so the build without `text` fails).
#[cfg(feature = "text")]
#[uniffi::export]
impl FfiDocument {
    /// The page's text as lines in reading order (B9d's `page_text`; feature `text`).
    ///
    /// # Errors
    /// `page-index-out-of-range` and reading errors.
    pub fn page_text(&self, index: u32) -> FfiResult<Vec<String>> {
        let text = self.lock()?.page_text(index, &laminapdf::extract::TextOptions::new())?;
        Ok(text.lines())
    }
}

/// A cancel token to pass to run-to-completion calls ([`FfiDocument::save`]).
#[derive(uniffi::Object, Default)]
pub struct FfiCancelToken {
    token: CancelToken,
    /// Async calls to wake on `cancel`, so a call waiting for the lock sees it at once.
    waiters: Wakers,
}

#[uniffi::export]
impl FfiCancelToken {
    /// A token that is not cancelled.
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Cancels every call and job holding this token (from any thread).
    pub fn cancel(&self) {
        self.token.cancel();
        self.waiters.wake_all();
    }

    /// Whether [`FfiCancelToken::cancel`] was called.
    pub fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }
}

/// A running save (ADR 0028 §3).
#[derive(uniffi::Object)]
pub struct FfiSaveJob {
    doc: Arc<FfiDocument>,
    job: Mutex<laminapdf::SaveJob>,
    token: CancelToken,
}

#[uniffi::export]
impl FfiSaveJob {
    /// Runs at most `units` work units, then returns progress or the result.
    ///
    /// # Errors
    /// `cancelled`, `document-changed`, `job-finished`, or the save's own errors.
    pub fn step(&self, units: u32) -> FfiResult<FfiSaveStep> {
        let mut job = self.job.lock().map_err(|_| FfiError::Poisoned)?;
        let mut doc = self.doc.lock()?;
        Ok(match job.step(&mut doc, Budget::units(units))? {
            Step::Progress { done, total, phase: p } => FfiSaveStep::Progress {
                done,
                total,
                phase: phase(p),
            },
            Step::Done(s) => FfiSaveStep::Done { saved: saved(s) },
            // A step kind this binding doesn't know: an error, not fake progress that callers
            // would step forever.
            other => {
                return Err(FfiError::Unsupported {
                    what: format!("save step {other:?}"),
                });
            }
        })
    }

    /// Cancels the job: the next step fails with `cancelled`. Callable from any thread,
    /// including while another thread is inside `step` (it doesn't take the job's lock).
    pub fn cancel(&self) {
        self.token.cancel();
    }

    /// Whether [`FfiSaveJob::cancel`] was called.
    pub fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }
}

// ---------------------------------------------------------------------------------------------
// Probes for the questions the spike must answer with tests.
// ---------------------------------------------------------------------------------------------

/// A toy job of `total` units, for cancellation tests: a real save of a small file finishes
/// in a handful of units, too fast to cancel half-way reliably.
#[derive(uniffi::Object)]
pub struct FfiCountJob {
    total: u32,
    done: AtomicU32,
    token: CancelToken,
}

#[uniffi::export]
impl FfiCountJob {
    /// A job of `total` units.
    #[uniffi::constructor]
    pub fn new(total: u32) -> Arc<Self> {
        Arc::new(Self {
            total,
            done: AtomicU32::new(0),
            token: CancelToken::new(),
        })
    }

    /// Does one unit; `true` when the job is done.
    ///
    /// # Errors
    /// `cancelled` once [`FfiCountJob::cancel`] was called (the façade's own error).
    pub fn step(&self) -> FfiResult<bool> {
        if self.token.is_cancelled() {
            return Err(laminapdf::Error::Cancelled.into());
        }
        let done = self.done.fetch_add(1, Ordering::SeqCst).saturating_add(1);
        Ok(done >= self.total)
    }

    /// Units done so far.
    pub fn done(&self) -> u32 {
        self.done.load(Ordering::SeqCst)
    }

    /// Cancels the job.
    pub fn cancel(&self) {
        self.token.cancel();
    }
}

/// Records whether a Rust future was dropped before it finished: the Kotlin and Swift tests
/// cancel the foreign coroutine/task and then read it. `cancel` stops the future from the Rust
/// side, which is how the Swift layer cancels: `UniFFI` 0.32's generated Swift never drops a
/// future whose task was cancelled.
#[derive(uniffi::Object, Default)]
pub struct FfiDropProbe {
    dropped: AtomicBool,
    finished: AtomicBool,
    cancelled: AtomicBool,
    polls: AtomicU32,
}

#[uniffi::export]
impl FfiDropProbe {
    /// A fresh probe.
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Whether the future holding this probe was dropped unfinished.
    pub fn dropped(&self) -> bool {
        self.dropped.load(Ordering::SeqCst)
    }

    /// Whether the future finished because `cancel` was called.
    pub fn finished(&self) -> bool {
        self.finished.load(Ordering::SeqCst)
    }

    /// Asks the future to finish at its next poll.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// How often the future was polled.
    pub fn polls(&self) -> u32 {
        self.polls.load(Ordering::SeqCst)
    }
}

/// A future that never finishes on its own: it re-wakes itself on every poll (so the foreign
/// executor keeps polling), finishes once the probe is cancelled, and sets the probe's
/// `dropped` flag in `Drop` if it had not finished.
struct Forever {
    probe: Arc<FfiDropProbe>,
}

impl Future for Forever {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let _ = self.probe.polls.fetch_add(1, Ordering::SeqCst);
        if self.probe.cancelled.load(Ordering::SeqCst) {
            self.probe.finished.store(true, Ordering::SeqCst);
            return Poll::Ready(());
        }
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

impl Drop for Forever {
    fn drop(&mut self) {
        if !self.probe.finished.load(Ordering::SeqCst) {
            self.probe.dropped.store(true, Ordering::SeqCst);
        }
    }
}

/// An async export that runs until the caller cancels it: does cancelling the Kotlin
/// coroutine or the Swift task drop the Rust future (and run our `Drop`)? It also finishes
/// when the probe is cancelled.
#[uniffi::export]
pub async fn run_until_cancelled(probe: Arc<FfiDropProbe>) {
    Forever { probe }.await;
}

/// An async export over the document lock, written the way ADR 0029 §4 asks: it never parks
/// the foreign executor's thread on the lock, nor spins: while another call holds the lock it
/// returns `Pending` with its waker registered, and the release (or a cancel) wakes it. It
/// takes a cancel token, which the Swift layer cancels in `onCancel` because the generated
/// Swift never drops a cancelled future.
#[uniffi::export]
pub async fn page_count_async(doc: Arc<FfiDocument>, cancel: Arc<FfiCancelToken>) -> FfiResult<u32> {
    let guard = std::future::poll_fn(|cx| doc.poll_lock(cx, &cancel)).await?;
    Ok(guard.page_count()?)
}

/// Panics on purpose, in an infallible export: Kotlin gets an `InternalException`, but the
/// generated Swift calls infallible functions with `try!`, so in Swift this traps (the app
/// crashes). Not called from the Swift tests; the README records the rule it implies.
#[uniffi::export]
#[allow(clippy::panic)]
pub fn panic_for_test() {
    panic!("deliberate panic from lamina-ffi (spike test)");
}

/// Panics on purpose, in a fallible export: under `panic = "unwind"` `UniFFI` should turn
/// this into a Swift error (`UniffiInternalError.rustPanic`) and a Kotlin `InternalException`
/// instead of aborting the process.
///
/// # Errors
/// Never returns normally.
#[uniffi::export]
#[allow(clippy::panic)]
pub fn panic_for_test_fallible() -> FfiResult<()> {
    panic!("deliberate panic from lamina-ffi (spike test, fallible export)");
}

/// The `lamina-ffi` version and whether the `text` feature is built in.
#[uniffi::export]
pub fn build_info() -> String {
    format!(
        "lamina-ffi {} (uniffi 0.32.2, text={})",
        env!("CARGO_PKG_VERSION"),
        cfg!(feature = "text")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use sha2::Digest as _;

    const HELLO: &[u8] = include_bytes!("../../../fixtures/golden/examples/hello.pdf");
    /// SHA-256 of the scenario's output; the Kotlin and Swift tests assert the same value.
    const SCENARIO_SHA256: &str = "12dc111c8a4c9b7f5b298caf19a84f17381387514b2ed51dafaf554d7feb2966";

    fn opts(mode: FfiSaveMode) -> FfiSaveOptions {
        FfiSaveOptions {
            mode,
            appended_only: false,
        }
    }

    #[test]
    fn scenario_matches_the_facade() -> FfiResult<()> {
        let doc = FfiDocument::open(HELLO.to_vec(), None)?;
        assert_eq!(doc.page_count()?, 1);
        let pages = doc.page_infos()?;
        assert_eq!(pages.len(), 1);
        #[cfg(feature = "text")]
        assert!(!doc.page_text(0)?.is_empty(), "hello.pdf has text");
        doc.set_info(FfiInfoKey::Title, Some("From the FFI".into()))?;
        assert_eq!(doc.info()?.title.as_deref(), Some("From the FFI"));
        let out = doc.save(opts(FfiSaveMode::Incremental), FfiCancelToken::new())?;
        assert_eq!(out.mode, Some(FfiSaveMode::Incremental));

        // The same edit through the façade directly gives the same bytes.
        let mut direct = Document::open_bytes(HELLO.to_vec(), OpenOptions::default()).map_err(FfiError::from)?;
        let _ = direct
            .apply(vec![Op::from(SetInfo::patch(
                InfoPatch::new().title(Some("From the FFI")),
            ))])
            .map_err(FfiError::from)?;
        let expected = direct
            .save(&SaveOptions::new().with_mode(SaveMode::Incremental))
            .map_err(FfiError::from)?;
        assert_eq!(out.bytes, expected.bytes);
        // The Kotlin and Swift tests assert the same hash (SCENARIO_SHA256 / scenarioSHA256).
        let digest = sha2::Sha256::digest(&out.bytes);
        let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, SCENARIO_SHA256);

        let reopened = FfiDocument::open_borrowed(&out.bytes, None)?;
        assert_eq!(reopened.info()?.title.as_deref(), Some("From the FFI"));
        Ok(())
    }

    #[test]
    fn errors_carry_the_adr_0026_fields() {
        match FfiDocument::open(b"not a pdf".to_vec(), None) {
            Err(FfiError::Lamina { code, kind, json, .. }) => {
                assert_eq!(code, "not-pdf");
                assert_eq!(kind, "input");
                assert!(json.contains("\"docsUrl\""));
            }
            other => panic!("expected not-pdf, got {:?}", other.map(|_| ())),
        }
    }

    #[test]
    fn cancelled_job_stops_at_the_next_step() -> FfiResult<()> {
        let job = FfiCountJob::new(1000);
        assert!(!job.step()?);
        job.cancel();
        match job.step() {
            Err(FfiError::Lamina { code, .. }) => assert_eq!(code, "cancelled"),
            other => panic!("expected cancelled, got {other:?}"),
        }
        assert_eq!(job.done(), 1);
        Ok(())
    }

    #[test]
    fn save_job_steps_to_done_and_cancels() -> FfiResult<()> {
        let doc = FfiDocument::open(HELLO.to_vec(), None)?;
        doc.set_info(FfiInfoKey::Subject, Some("job".into()))?;
        let job = Arc::clone(&doc).save_job(opts(FfiSaveMode::Auto))?;
        let mut steps = 0;
        loop {
            steps += 1;
            if let FfiSaveStep::Done { saved } = job.step(1)? {
                assert!(!saved.bytes.is_empty());
                break;
            }
        }
        assert!(steps > 1, "a one-unit budget should take several steps");

        doc.set_info(FfiInfoKey::Subject, Some("job 2".into()))?;
        let job = Arc::clone(&doc).save_job(opts(FfiSaveMode::Auto))?;
        job.cancel();
        assert!(matches!(job.step(1), Err(FfiError::Lamina { code, .. }) if code == "cancelled"));
        Ok(())
    }

    #[test]
    fn cancelling_the_probe_finishes_the_future_without_a_drop() {
        let probe = FfiDropProbe::new();
        let mut future = std::pin::pin!(run_until_cancelled(Arc::clone(&probe)));
        let mut cx = Context::from_waker(std::task::Waker::noop());
        assert!(future.as_mut().poll(&mut cx).is_pending());
        probe.cancel();
        assert!(future.as_mut().poll(&mut cx).is_ready());
        assert!(probe.finished() && !probe.dropped());
    }

    #[test]
    fn dropping_the_future_unfinished_sets_dropped() {
        let probe = FfiDropProbe::new();
        {
            let mut future = Box::pin(run_until_cancelled(Arc::clone(&probe)));
            let mut cx = Context::from_waker(std::task::Waker::noop());
            assert!(future.as_mut().poll(&mut cx).is_pending());
        }
        assert!(probe.dropped() && !probe.finished());
    }

    /// Counts wakes.
    struct CountingWaker(AtomicU32);

    impl std::task::Wake for CountingWaker {
        fn wake(self: Arc<Self>) {
            let _ = self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn page_count_async_waits_without_spinning_and_wakes_on_release() -> FfiResult<()> {
        let doc = FfiDocument::open(HELLO.to_vec(), None)?;
        let counter = Arc::new(CountingWaker(AtomicU32::new(0)));
        let waker = Waker::from(Arc::clone(&counter));
        let mut cx = Context::from_waker(&waker);
        let mut future = Box::pin(page_count_async(Arc::clone(&doc), FfiCancelToken::new()));
        let held = doc.lock()?;
        assert!(future.as_mut().poll(&mut cx).is_pending());
        assert!(future.as_mut().poll(&mut cx).is_pending());
        assert_eq!(
            counter.0.load(Ordering::SeqCst),
            0,
            "no self-wake while the lock is held"
        );
        drop(held);
        assert!(
            counter.0.load(Ordering::SeqCst) >= 1,
            "releasing the lock wakes the waiter"
        );
        assert!(matches!(future.as_mut().poll(&mut cx), Poll::Ready(Ok(1))));
        Ok(())
    }

    #[test]
    fn page_count_async_wakes_and_stops_on_cancel() -> FfiResult<()> {
        let doc = FfiDocument::open(HELLO.to_vec(), None)?;
        let cancel = FfiCancelToken::new();
        let counter = Arc::new(CountingWaker(AtomicU32::new(0)));
        let waker = Waker::from(Arc::clone(&counter));
        let mut cx = Context::from_waker(&waker);
        let mut future = Box::pin(page_count_async(Arc::clone(&doc), Arc::clone(&cancel)));
        let _held = doc.lock()?;
        assert!(future.as_mut().poll(&mut cx).is_pending());
        cancel.cancel();
        assert!(
            counter.0.load(Ordering::SeqCst) >= 1,
            "cancel wakes the waiter while the lock is held"
        );
        assert!(matches!(
            future.as_mut().poll(&mut cx),
            Poll::Ready(Err(FfiError::Lamina { code, .. })) if code == "cancelled"
        ));
        Ok(())
    }
}
