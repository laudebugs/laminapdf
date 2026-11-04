# 0028 — Façade rules, jobs, cancellation and progress

- Status: Accepted (owner, 2026-10-29)
- Date: 2026-10-29
- Amended 2026-11-04 ([§8](#8-the-wrapped-crates-errors-are-opaque)): the crates' errors inside `laminapdf::Error` are opaque, behind `Error::Core(CoreError)`, and their types are no longer re-exported, except `BuildError` and `TextError` (decided by the owner, todo [Q2y](../todos/done/Q2y-facade-error-wrapping.md)).

## Context

[ADR 0002 §3](0002-binding-strategy.md) requires a handle-based façade in the `laminapdf` crate, designed once and wrapped by `laminapdf-wasm` (wasm-bindgen + tsify), Tauri commands and, later, `laminapdf-ffi` (UniFFI). It names the principles (owned types, opaque handles, bytes in and out, no lifetimes or generics) but not the rules. B5c will write the façade, so the rules have to exist first. Fixing them after `@laminapdf/sdk` ships would break its API ([report: design the façade to the stricter binding now](../research/libraries/shareable-libraries.md#design-the-façade-to-the-stricter-binding-now)).

Constraints from the binding tools:

- **UniFFI** keeps every object "behind an `Arc<>`". "The Rust struct must not expose any methods that take `&mut self`", and "Uniffi enforces all interfaces are `Send + Sync`". Mutation goes through interior mutability that is itself `Send` and `Sync` ([UniFFI manual: interfaces](https://github.com/mozilla/uniffi-rs/blob/main/docs/manual/src/types/interfaces.md)). Errors are enums implementing `std::error::Error` ([errors](https://github.com/mozilla/uniffi-rs/blob/main/docs/manual/src/types/errors.md)). Futures need a foreign executor, and "there's no builtin way to cancel a future" ([futures](https://github.com/mozilla/uniffi-rs/blob/main/docs/manual/src/futures.md)). Zero-copy `&[u8]` arguments (0.32) work only in sync functions ([CHANGELOG](https://github.com/mozilla/uniffi-rs/blob/main/CHANGELOG.md)).
- **wasm-bindgen** exported functions "cannot have their own generic type parameters". A class passed by value is moved out of its JS object, while `&T`/`&mut T` leaves it usable ([exported Rust types](https://wasm-bindgen.github.io/wasm-bindgen/reference/types/exported-rust-types.html)). `u64`/`i64` become `BigInt` ([numbers](https://wasm-bindgen.github.io/wasm-bindgen/reference/types/numbers.html)). Classes are single-threaded JS objects, with no `Send` requirement.
- **Neither** has an iterator or stream type. Neither can cancel a synchronous call: single-threaded wasm can't run JS while Rust is busy, and wasm threads need COOP/COEP, which a library embedded in other people's pages can't demand ([rust-core-bridge note §2](../research/libraries/notes/rust-core-bridge.md#2-one-façade-for-wasm-bindgen-and-uniffi-patterns-and-pitfalls), [§5](../research/libraries/notes/rust-core-bridge.md#5-concurrency-cancellation-progress)).

Constraints from lamina:

- [ADR 0005 §7](0005-document-model.md#7-threads): `Document` is `Send` but `!Sync` (`Arc` objects, `RefCell` caches), with no `Mutex` or atomics-based caches in the core.
- The [portability rules](../guidelines/code-organization.md#2-boundaries-and-dependency-direction): core crates (the `laminapdf` façade included) use no threads and no clocks (`Instant`, `SystemTime`). `cargo xtask portability` enforces this (`FORBIDDEN_IDENTS` in `tools/xtask/src/workspace.rs`). On `wasm32-unknown-unknown`, std's `Instant::now()` panics in any case, because the target has no clock.
- `laminapdf_model::Page<'d>` and `Pages<'d>` borrow the document ([ADR 0005 §4](0005-document-model.md#4-typed-views-laminapdf-model)). `Document::open` is generic over `ByteSource`.

## Decision

### 1. Façade rules

The `laminapdf` façade is a core crate whose public API each binding wraps one-to-one. B5c's acceptance criteria refer to these rules by number.

| # | Rule | Why (constraint) |
| --- | --- | --- |
| F1 | **No lifetimes in façade signatures.** No `Page<'d>`, no borrowed returns. Sub-objects are addressed by value: `doc.page_info(index)`, `doc.page_by_id(&PageId)`. Methods return owned data | Neither binding can return a borrow. UniFFI objects are `Arc`s, and wasm-bindgen classes own their Rust value |
| F2 | **Handles with a back-pointer live in the edge crates.** `PageRef { doc, page: PageId }` is built in `laminapdf-wasm`/`laminapdf-ffi` from the edge's shared document pointer (§2) and resolves its index lazily. It holds the `PageId`, not the index, so moving pages doesn't retarget it ([ADR 0005 §5](0005-document-model.md#5-laminapdfids)) | The core can't share ownership of a `!Sync` document across threads without a lock (§2). An edge handle can |
| F3 | **No generics, `impl Trait` or closures in bound signatures.** Constructors are monomorphised: `open_bytes(Vec<u8>, OpenOptions)`; `open_source(Arc<dyn ByteSource>, OpenOptions)` is for edge crates, which wrap a host-specific source (L2c's `SparseSource`) behind a concrete constructor. Rust-only conveniences (`Job` helpers, generic `Step<T>`) are allowed if every bound type has a concrete form | wasm-bindgen exports can't be generic; UniFFI has no generics (Bitwarden had to add non-generic wrappers, [PR #488](https://github.com/bitwarden/sdk-internal/pull/488)) |
| F4 | **Reads take `&self`; changes take `&mut self` on the façade.** The edge decides how `&mut` is reached (§2). The façade never uses interior mutability to fake `&self` | UniFFI forbids `&mut self` on *objects*. The edge object takes `&self` and locks. The core keeps Rust's borrow rules instead of a hidden lock |
| F5 | **Plain owned data across the boundary**: records of `String`, `bool`, integers, `f64`, `Vec`, `Option` and other records. Counts and indices are `u32`. A `u64` is used only where the range is needed (byte offsets) and is a number in serde/tsify records | Maps to tsify records and `uniffi::Record`. A `u64` in a wasm-bindgen signature becomes `BigInt` |
| F6 | **Options are records with defaults.** Rust keeps private fields and `with_*` builders (as `OpenOptions`, `Limits` do, ADR 0003 §3); edges define their own DTOs (all fields optional) and convert. Unknown keys fail as `unknown-option` with `didYouMean` ([ADR 0026](0026-error-and-diagnostic-contract.md) §1) | Builders don't bind; plain records do. LLM-written JS bypasses types |
| F7 | **Bytes in as `Vec<u8>`** (or `&[u8]` in edges where the tool lends it), **out as `Vec<u8>`**, never as serde arrays (ADR 0002 §1). Incremental saves can return only the appended bytes | `Uint8Array` / `Data` / `ByteArray`; avoids copying a large original back |
| F8 | **Cursors, not iterators.** Sequences that can be long (warnings, search hits, extraction blocks) are `next_batch(&mut self, max: u32) -> Vec<T>` on a cursor value. TS wraps a cursor in an (async) iterator | Neither binding has an iterator or stream type |
| F9 | **One error type**, `laminapdf::Error`, serialising to ADR 0026 §1; the crates' errors inside it are opaque (§8). Edges map it to the UniFFI `LaminaPdfError` enum (one variant per kind, each with a `Diagnostic` record of strings and integers) and to the TS `LaminaPdfError` | A string-payload error enum is the only error shape both bindings express |
| F10 | **Long work is a `Job`** (§3), and every long operation also has a run-to-completion form that takes a `CancelToken`. No progress callbacks cross the boundary | Neither binding can cancel or interrupt a sync call; callbacks cost an FFI crossing each (§6) |
| F11 | **No panics across the boundary.** The core doesn't panic (lints, fuzzing, `Limits`). Native edges build with `panic = "unwind"` so UniFFI's scaffolding can catch a bug, and the web treats a trapped instance as disposable (L2b) | Scaffolding can only catch a panic that unwinds; under wasm `panic=abort` a panic kills the instance ([report: panic safety](../research/libraries/shareable-libraries.md#panic-safety-no-panics-in-core-a-disposable-instance-on-the-web-unwinding-at-native-edges)) |
| F12 | **Same names on every surface**, modulo case (`page_info` → `pageInfo`). One façade method maps to one binding method | Guessable APIs across languages and for models (report) |

The edge wrappers are mechanical. B5c or L6a may generate them with a `macro_rules!` or a small proc-macro crate, as Bitwarden and Matrix do.

### 2. Where the lock lives: in the edge, not in the core

**Decision: the core stays `Send + !Sync` with no `Mutex`, as [ADR 0005 §7](0005-document-model.md#7-threads) requires. ADR 0005 is not amended.** Each edge chooses the shared-ownership wrapper its runtime needs:

| Edge | Wrapper | Why |
| --- | --- | --- |
| `laminapdf-ffi` (UniFFI) | `struct FfiDocument(Mutex<laminapdf::Document>)`, `Arc`'d by UniFFI; `&self` methods lock | `Mutex<T>` is `Send + Sync` when `T: Send`, which is exactly what ADR 0005 §7 provides |
| `laminapdf-wasm` | `Rc<RefCell<laminapdf::Document>>` | wasm32 without threads; wasm-bindgen classes need no `Send`. `RefCell` double borrows can't happen with no re-entrancy |
| Desktop (Tauri, app code) | `Mutex<laminapdf::Document>` in managed state, work in `spawn_blocking` | Tauri state must be `Send + Sync` |
| CLI | The document by value | One thread per document (`--jobs` opens one document per worker) |

Compile-time tests (B5c, L6a) assert `laminapdf::Document: Send`, `laminapdf::Document: !Sync` and `FfiDocument: Send + Sync`.

A poisoned `Mutex` (a panic while locked, caught under unwind) means the document is in an unknown state. `laminapdf-ffi` reports it as an `internal` error (code added by L6a in `codes.json`), and the caller reopens. An edge lock serialises all calls on one document. That is UniFFI's model anyway, and parallelism comes from one document per thread over a shared `Arc<dyn ByteSource>` (ADR 0005 §7). B5c and L6a measure it once real workloads exist. If a measured case needs concurrent reads of one document, the remedy is an amendment of ADR 0005 §7 to a `Sync` core, not a lock added quietly.

### 3. Jobs

Long operations (save, render, extract, search, signing, validation) are resumable state machines the caller steps. Jobs don't borrow the document: it is passed to each step, so the edge can lock it per step and other calls (rendering a tile while an extraction runs) can interleave.

```rust
// Illustrative; B5c defines the façade forms, core crates (laminapdf-extract, L3a) the same shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Budget { units: u32 }
impl Budget {
    pub const fn units(units: u32) -> Self;   // u32 per F5: a plain number in JS, not BigInt
    pub const UNLIMITED: Self;
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Step<T> {
    Progress { done: u32, total: Option<u32>, phase: Phase },  // pages, objects …; `total` None when unknown
    Done(T),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Phase { Parse, Layout, Extract, Render, Write, Sign }

pub struct ExtractJob { /* state machine, CancelToken, document instance and revision */ }
impl ExtractJob {
    pub fn cancel_token(&self) -> CancelToken;
    /// Runs at most `budget` work units, then returns progress or the result.
    /// # Errors
    /// `cancelled`, `document-changed`, `job-finished`, or the operation's own errors.
    pub fn step(&mut self, doc: &Document, budget: Budget) -> Result<Step<Extraction>, Error>;
}
// A job that changes the document takes `doc: &mut Document` (SaveJob).

impl Document {
    pub fn extract_job(&self, options: &ExtractOptions) -> Result<ExtractJob, Error>;
    pub fn extract(&self, options: &ExtractOptions, cancel: &CancelToken) -> Result<Extraction, Error>; // F10
}
```

- **One concrete job type per operation** (`SaveJob`, `RenderJob`, `ExtractJob`), each a binding class. `Step<T>` is Rust-only (F3). Edges expose a concrete step record per job.
- **Identity and staleness, for every job.** B5c adds a document **revision**: a counter that every committed transaction, undo, redo, save and rebase bumps, together with an instance identifier. Every job, reading or writing, records both when it is created. **Every `step` checks them in the core**, before any work. A different instance or revision fails with `document-changed` (kind `state`), and the job is finished. So no interleaving can make a save write mixed state, whatever the edge does: an op committed between two `SaveJob` steps fails the save, and the caller starts a new one. A write job doesn't bump the revision while it runs. Its own final effect (a save's rebase) bumps it once, as the last thing the job does. Stepping a finished, failed or cancelled job fails with `job-finished` (state).
- **Errors end the job.** A cancelled or failed save leaves the document as before (`Dirty`, overlay untouched, [ADR 0005 §3](0005-document-model.md#3-base-overlay-and-the-save-cycle)), and partial output is discarded. A cancelled read job returns no partial result.

### 4. `Budget` is counted in work units, not time

The core has no clock, so a `Budget` is **work units**. One unit is roughly one cheap, indivisible piece of work: one object parsed or written, one content-stream operator interpreted, one glyph shaped, or 1 KiB decoded or written. Each job documents its weights. B5c and L3a record the measured units per millisecond for each job on reference hardware in their READMEs, so edges can pick a starting budget.

- **Edges translate time into units.** An edge runs a step, measures elapsed time with its own clock (`performance.now()`, `Instant`, `ContinuousClock`), and adapts the next budget toward its time slice. Recommendation: about 8 ms per step in a Web Worker (cancel latency and progress cadence) and about 50 ms natively. A step loop may also simply check a host clock between steps (the CLI `--timeout`).
- **Overshoot is bounded by one indivisible unit.** Unbounded single operations (one stream decode) are already bounded by `Limits` (ADR 0003 §3). Finer-grained pausing comes where a measurement shows a long unit.
- **Deterministic:** the same budget gives the same step boundaries on every platform, so tests can step a job with small budgets and compare against a run to completion.

### 5. Cancellation

```rust
#[derive(Clone, Debug, Default)]
pub struct CancelToken(Arc<AtomicBool>);
impl CancelToken {
    pub fn new() -> Self;
    pub fn cancel(&self);                 // Ordering::Relaxed is enough: a flag, no data published
    pub fn is_cancelled(&self) -> bool;
}
```

- The core polls the token at every budget check, at the start of every `step`, and inside long loops (per page, per object, inside XY-Cut recursion). A tripped token makes the current call return `Err` with code **`cancelled`** (kind `cancelled`, ADR 0026 §2), and the job is finished.
- This is an atomic flag, not a lock or a cache, so it doesn't conflict with ADR 0005 §7. On `wasm32-unknown-unknown` without the atomics feature, `AtomicBool` compiles to plain loads and stores. Cancellation lands between steps, when the worker's event loop has run the `cancel` message.
- A token can be shared by several jobs (cancel everything for one request). A run-to-completion call (F10) takes one too.

### 6. Progress and throttling

- Progress is **returned**, never called back: one `Step::Progress` per step at most, with `done`, `total` (when known) and `phase`. The budget throttles it: a bigger budget means fewer reports. No foreign callback crosses the boundary inside a step.
- Edges throttle again before user code. Recommendation: the worker client posts at most one progress event per 50 ms, plus the final one. The CLI redraws at most 10 times a second, and only when stderr is a TTY. Swift/Kotlin layers conflate on their streams.

### 7. No threads in the core; async only at the edges

The core is synchronous and single-threaded. How each surface drives a job:

| Surface | How it drives a job |
| --- | --- |
| TS sync (`@laminapdf/sdk`) | `doc.extract(opts)` runs to completion. `doc.extractJob(opts)` returns an `ExtractJob` with `step(units: number)` returning `{ done: false, progress: { done, total?, phase } }` or `{ done: true, value }`, for callers who slice work themselves. All numbers are plain JS numbers (F5) |
| TS worker client (L2a) | Steps in the worker with a time-sliced budget and yields to the event loop between steps. `AbortSignal` → `cancel { id }` → `token.cancel()`. Rejects with `LaminaPdfError` `cancelled` (`cause` = `signal.reason`). Progress arrives as events |
| Swift (L6a, ADR 0029) | A hand-written `async throws` layer steps the job off the main actor. `withTaskCancellationHandler` trips the token; progress is an `AsyncStream` |
| Kotlin (L6a, ADR 0029) | A `suspend` function on `Dispatchers.Default` steps the job. `ensureActive()` between steps and `invokeOnCancellation` trip the token; progress is a `Flow` |
| CLI (L4a, ADR 0031) | A step loop on the main thread. Ctrl-C trips the token; `--timeout` checks `Instant` between steps and reports a timeout code (kind `limit`, added by L4a); progress on stderr when it is a TTY |
| MCP (L4c) | The CLI loop; the call timeout trips the token |
| Desktop (Tauri) | `spawn_blocking`, locking the document per step (§2) |

UniFFI async exports are not used. lamina's work is CPU-bound, so there is nothing to await, async exports lose zero-copy bytes, and they bring no cancellation. Parallelism (pages across threads, worker pools) is the edges' job: one document per thread or worker.

### 8. The wrapped crates' errors are opaque

*Amendment, 2026-11-04 (todo Q2y).* Until this amendment, `laminapdf::Error` had one public variant per crate below the façade (`Cos`, `Ops`, `Save`, `Text`, `Build`), and the façade re-exported their types (`CosError`, `OpsError`, `SaveError`, `SinkError`, `TextError`, `BuildError`). Every change to one of those enums was therefore a breaking change to the façade, and the code broke rust.md's rule: a crate "SHOULD NOT expose another crate's error type".

There were two ways to settle it:

- **Keep wrapping and amend rust.md**, with an exception for crates in the same lockstep version (ADR 0017 §1). Nothing changes for callers, who can keep matching on inner variants. But a renamed or split variant in any crate stays a façade break, and before 1.0 the inner crates change often.
- **Make the wrapped errors opaque.** Callers read the error through what ADR 0026 already makes stable: `code()`, `error_code()` (match on `ErrorCode`), `kind()`, `location()`, `hint()`, `data` and `diagnostic()`. A caller matching on an inner variant has to switch to the code, which is a break while the version is `0.x-alpha`.

The owner chose the second, so:

- The five variants are one, `Error::Core(CoreError)`. `CoreError` is a struct over a crate-private enum. Its message is the inner error's and its `Debug` shows the inner error, so messages and debugging don't change. The `From` impls stay, so `?` still converts every crate's error.
- The façade's own variants (`PageIndexOutOfRange`, `Cancelled`, `JobFinished`, `DocumentChanged`) stay public: they are the façade's types, not another crate's.
- Anything a caller read from an inner variant gets a method instead. So far that is `Error::pending_ranges()`, the ranges of a `data-pending` error, which `laminapdf-source`'s retry loop needed.
- The re-exports go, except `BuildError` and `TextError`. The builder's types (`Surface`, `PageBuilder` and the rest) come from `laminapdf-build` as they are (§1), and `Font` from `laminapdf-text`; their own methods return those crates' errors, so callers need the names in their own signatures.
- `laminapdf::Warning` still wraps the crates' warning types (`Warning::Document`, `Warning::Font`). Warnings follow the same rule, and making them opaque is the next step; this amendment covers errors.

rust.md's rule stands as it is. The façade now follows it, except for `BuildError` and `TextError`: they come with the builder and `Font` APIs, which the façade takes from their crates as they are.

## Consequences

- One job model serves every surface. Cancel and progress work in single-threaded wasm without SharedArrayBuffer, and identically on native.
- The façade is a little less ergonomic for Rust callers: jobs take the document per step, and sub-objects are addressed by ID. Rust callers who want borrowed views can still use `laminapdf-model` directly.
- Each long operation costs a state machine instead of a straight loop. That is the main cost of this ADR, and B5c, L3a and C-round renderers pay it.
- ADR 0005 §7 holds unchanged. The lock's cost is visible and measurable at one place per edge, and a `Sync` core stays an explicit, later choice.
- B5c adds the document revision counter and its check in every job, the compile-time auto-trait tests, and the codes `cancelled`, `job-finished` and `document-changed`, with their kinds and `explain` pages. The edge crates add F2's `PageRef`. This ADR informs proposed ADR 0009 (worker RPC, C2a): the RPC carries job steps, cancel and progress messages, not proxies.

## Alternatives considered

- **`Sync` core: `Arc<RwLock<…>>` inside the façade** (the report's sketch). One handle type for every edge, but it amends ADR 0005 §7, puts lock traffic on every call in single-threaded wasm, and hides contention inside the core. Revisit only with a measured need (§2).
- **Async core** (`async fn` throughout). Every edge would need an executor, UniFFI async loses zero-copy bytes and has no cancellation, and nothing in PDF work is I/O-bound inside the core (missing ranges already return `data-pending`, ADR 0005 §2).
- **Threads in the core** (rayon per page). Breaks the portability rules and `wasm32-unknown-unknown`, and in browsers it needs COOP/COEP. Parallelism belongs at the edges.
- **Binding-level cancellation** (drop the future, terminate the worker). UniFFI has none, a sync wasm call can't be interrupted, and terminating a worker loses every document in it. Kept only as `client.terminate()`, the hard stop.
- **Progress callbacks across the boundary** (UniFFI foreign traits, `js_sys::Function`). One FFI crossing per report, re-entrancy hazards, and a JS dependency in the core. Returned progress does the same job.
- **A time `Budget` in the core, or a `Clock` trait the caller passes in.** The guidelines allow a caller-supplied trait, but a clock read inside the core makes step boundaries nondeterministic and crosses FFI often. Edges measuring between steps get the same latency with neither problem.
- **Comlink-style proxies** for the worker. Every property access is a round trip, there is no protocol version, and transfers are implicit. L2a uses typed RPC over the same job model.
- **Iterators across the boundary.** Neither tool supports them. Cursors with `next_batch` are as cheap and bind everywhere.
