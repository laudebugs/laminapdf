# L2b — Disposal and memory bounds: `Symbol.dispose` on every handle, `disposed` code, capped wasm memory, `reset()` and trap recovery

| Field | Value |
| --- | --- |
| ID | `L2b` |
| Status | `ready` |
| Plan | [shareable-libraries §7 L2](../../plans/shareable-libraries.md#l2-typescript-hardening) |
| Round / milestone | Round L2 (shareable libraries) |
| Depends on | [B6b](../done/B6b-lamina-pdf-package.md), [L1b](./L1b-error-contract-codegen.md) |
| Blocks | None |
| ADRs | Follows [0002](../../decisions/0002-binding-strategy.md) (wasm-bindgen handles, `free()`/`using`), [0003](../../decisions/0003-reader-tolerance-and-limits.md) §3 (`Limits` as the first memory bound), 0016 (build profile, from [B6a](../done/B6a-lamina-wasm-and-adr-0016.md)), 0026 (codes, from [L1a](../done/L1a-contract-adrs.md), if merged), [0018](../../decisions/0018-test-strategy.md) §2 (no wall-clock assertions) |
| Branch | `feat/l2b-disposal-and-memory` |
| PR title | `feat(pdf): Symbol.dispose on every handle, capped wasm memory and instance recovery` |
| Size | M (≈ 300–500 lines of TS, Rust build config and tests) |
| PR | — |

## Goal

Keep `@lamina/pdf` from leaking or growing without bound in long-lived pages and servers. wasm linear memory grows but never shrinks, so one large file can leave an instance huge. A wasm trap (OOM, a bug) poisons the instance for every later call. This todo does five things:

1. Every handle supports `using`.
2. A call on a disposed handle throws a coded error instead of a glue-level null-pointer failure.
3. wasm memory has a link-time maximum.
4. `reset()` reclaims grown memory.
5. A wrapper catches traps, marks the instance poisoned and reinstantiates it, so the next call works ([report: disposal](../../research/libraries/shareable-libraries.md#disposal-using-first-finalizationregistry-as-a-backstop-memory-bounded-three-ways)).

## Context

- [Report: disposal](../../research/libraries/shareable-libraries.md#disposal-using-first-finalizationregistry-as-a-backstop-memory-bounded-three-ways): `using` support by engine, Safari's conflicting status, FinalizationRegistry as a backstop, three memory bounds, `DISPOSED`.
- [TypeScript note §5](../../research/libraries/notes/typescript-web-dx.md#5-resource-management-using-finalizationregistry-linear-memory): `--max-memory` linker flag, `reset()`, memory never shrinks **[unverified]**.
- [Report: panic safety](../../research/libraries/shareable-libraries.md#panic-safety-no-panics-in-core-a-disposable-instance-on-the-web-unwinding-at-native-edges) and [rust-core-bridge note §8](../../research/libraries/notes/rust-core-bridge.md#8-panic-safety-across-ffi): stable wasm32 is `panic=abort`; treat the instance as disposable; reinstantiate on `RuntimeError: unreachable`.
- [B6b](../done/B6b-lamina-pdf-package.md): ships `doc.free()` and `Symbol.dispose` on `Document`, and lists "memory management across the boundary" as a risk. This todo covers every handle and the rest.
- [ADR 0003 §3](../../decisions/0003-reader-tolerance-and-limits.md#3-limits): `Limits` is the first bound. Most hostile input must fail there, before memory runs out.
- [ADR 0006](../../decisions/0006-op-log-and-undo.md): the op log can replay edits onto a re-opened document (an option for recovery; see Risks).

## Scope

**In scope**

- **Disposal on every handle.** `Document`, `Page`, `Builder`, `Font`, transactions and any other wasm-backed class get `[Symbol.dispose]()`. Async-capable ones also get `[Symbol.asyncDispose]`.
  - `free()`/`close()` stay as aliases. Disposing twice is a no-op.
  - Children (a `Page` of a disposed `Document`) become disposed too.
  - Lookup is `Symbol.dispose ?? Symbol.for("Symbol.dispose")`, so runtimes without the symbol still work; the library does not assign to globals (`sideEffects: false`).
- **`disposed` code.** Any method on a disposed handle throws `LaminaError` with code `disposed`. [L1b](./L1b-error-contract-codegen.md) adds the code and the `LaminaError` class; this todo uses them.
- **FinalizationRegistry backstop.** Keep wasm-bindgen's registry. Register TS wrapper handles too, so a forgotten handle is freed eventually. An opt-in debug hook (`onLeak`) reports handles collected without dispose.
- **Capped wasm memory.** Add `-C link-arg=--max-memory=<bytes>` for `lamina-wasm` in the `cargo xtask wasm` build (value set in ADR 0016 or this PR; recommendation: 2 GiB, the safe cross-browser wasm32 ceiling, to verify). Document it in ADR 0016 if it has merged. Growth past the cap traps. The wrapper below turns the trap into a coded error (`out-of-memory`).
- **`reset()`.** A module-level `reset()` drops the instance and re-runs `init()` from the cached `WebAssembly.Module`. Every live handle becomes disposed. Memory returns to the fresh-instance size.
- **Abort-and-reinstantiate wrapper.** Every call into the glue goes through one guard. On a `WebAssembly.RuntimeError` it:
  1. marks the instance poisoned;
  2. rethrows as `LaminaError` (`wasm-trap` or `out-of-memory`);
  3. lets the next call reinstantiate.

  Handles from the poisoned instance throw `instance-reset` (or `disposed`; decide in the PR). Optional recovery: `Document.open(bytes, { recoverable: true })` keeps the input so `doc.reopen()` can rebuild it.
- **Test hooks** behind a non-default `test-hooks` Cargo feature in `lamina-wasm`: `__trap()` and `__grow(bytes)`. They are never in release builds; CI checks the release `.wasm` does not export them.

**Out of scope** (and which todo covers it)

- Worker termination as the memory-reclaim path and worker crash handling: [L2a](./L2a-worker-client.md), [C3a](./C3a-viewer-worker-rpc.md).
- Rust-side allocation budgets beyond today's `Limits`: an ADR 0003 amendment if measurement shows a need.
- `panic=unwind` wasm builds (nightly only): not planned; native `catch_unwind` at the edge belongs to [L6a](../ongoing/L6a-lamina-ffi-spike.md).

## Tasks

- [ ] Load the `/rust-best-practices` skill and follow [docs/guidelines/rust.md](../../guidelines/rust.md) for the `lamina-wasm` changes. Core crates do not change and stay bytes-in/bytes-out
- [ ] Inventory every wasm-backed class exported by `packages/pdf` and `@lamina/wasm`; list them in the PR description
- [ ] `src/internal/handle.ts`:
  - [ ] A base `Handle` with `disposed` state, `[Symbol.dispose]`, `free()`/`close()` aliases, child tracking and the `assertLive()` check
  - [ ] Every public class extends or wraps it
- [ ] `src/internal/instance.ts`:
  - [ ] The single guard around glue calls; poisoned state; reinstantiate from the cached `Module`
  - [ ] Error mapping to codes
  - [ ] `reset()`
- [ ] Add codes `wasm-trap`, `out-of-memory` and `instance-reset` (as decided) to `fixtures/codes.json` in L1b's format (`disposed` comes from L1b); regenerate; Rust and TS codes tests pass
- [ ] `tools/xtask` `wasm`: the `--max-memory` link arg (one constant, documented) and the `test-hooks` build variant for tests; a check that the release build has no `__trap`/`__grow` exports
- [ ] FinalizationRegistry registration and the `onLeak` debug hook
- [ ] TSDoc on every handle: "use `using`; `free()` is an alias"; README section on memory (bounds, `reset()`, workers)
- [ ] Verify Safari's `Symbol.dispose` status on the MDN compatibility table and record it in the README and Risks

## Acceptance criteria

- [ ] Every exported wasm-backed class has `[Symbol.dispose]` (a test iterates the inventory); `using doc = Document.open(bytes)` compiles and frees at scope end
- [ ] Each method of a disposed `Document`, and of a `Page` from it, throws `LaminaError` with code `disposed` (no `null pointer passed to rust` or other glue errors)
- [ ] Memory baseline:
  - Open, read every page and dispose a corpus file 1,000 times in a loop. After the first iteration, `WebAssembly.Memory.buffer.byteLength` stays within the post-warm-up size plus one 64 KiB page.
  - With every handle disposed, the leak hook reports none.
- [ ] `__grow` past the cap throws `LaminaError` with code `out-of-memory`. The next `Document.open` succeeds on a fresh instance. Handles from before the trap throw the decided code
- [ ] `__trap()` gives the same recovery with code `wasm-trap`
- [ ] `reset()` returns `byteLength` to the fresh-instance size after a large open; old handles throw
- [ ] The release `.wasm` exports no test hooks; JS glue + API size stays within ADR 0016's budget

## Post-merge checks

None.

## Tests

- `packages/pdf/test/dispose.test.ts`: inventory, `using`, double dispose, children, `disposed` errors.
- `packages/pdf/test/memory.test.ts`: the 1,000-iteration baseline loop (byte counts, no timing), the cap and `__grow`, `__trap`, `reset()`, leak hook. Run in Node in the fast lane, and in browser mode where [B6c](./B6c-runtime-matrix.md) provides it.
- xtask: unit test for the export check on the release build.

## Docs to update

- [ ] [shareable-libraries §9 L2](../../plans/shareable-libraries.md#9-to-dos): tick the disposal to-do
- [ ] `packages/pdf/README.md` (memory and disposal section); ADR 0016 (max-memory value), if merged; `fixtures/README.md` if the codes format note changes

## Risks and open questions

- **Max memory value:** too low rejects legitimate large files; too high defeats the cap. Recommendation: 2 GiB by default. If a caller-set cap is needed, it requires a different build, because the maximum is fixed at link time (to verify: whether `WebAssembly.Memory` with a lower `maximum` can be imported instead). Owner decides with ADR 0016.
- **Does memory never shrink?** The report marks it unverified. If an engine can shrink memory, `reset()` is still correct, just less necessary.
- **OOM before the cap:** a browser tab can run out of memory below 2 GiB. `Limits` remains the main defence; this cap is the second line.
- **Recovery after a trap** loses unsaved edits unless the op log is replayed. Recommendation: offer `reopen()` from kept bytes now, and leave op-log replay for later, when [E1d](./E1d-edit-session-undo-ui.md) needs it.
- **Safari and `using`:** sources conflict ([report](../../research/libraries/shareable-libraries.md#disposal-using-first-finalizationregistry-as-a-backstop-memory-bounded-three-ways)). TypeScript's downlevelled `using` helper needs `Symbol.dispose` to exist at runtime (to verify). If it does, document the one-line polyfill for apps rather than shipping a global side effect.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
