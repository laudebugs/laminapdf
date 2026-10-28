# E10b — Signatures: feature plan

| Field | Value |
| --- | --- |
| ID | `E10b` |
| Status | `ready` |
| Plan | [full-editor §E10](../../plans/full-editor.md#e10-signatures) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E9a](./E9a-form-authoring-feature-plan.md), [E10a](./E10a-adr-0020-signatures.md) |
| Blocks | [E10c](./E10c-visual-signatures.md), [E10d](./E10d-pades-signing-core.md), [E11a](./E11a-security-and-sanitisation-feature-plan.md) |
| ADRs | Follows ADR 0005, 0006, 0007 |
| Branch | `docs/e10b-signatures-feature-plan` |
| PR title | `docs(editor): E10 feature plan (signatures)` |
| Size | S (< 200 lines) |
| PR | — |

## Goal

The short feature-plan PR that opens round E10 ([full-editor §E10](../../plans/full-editor.md#e10-signatures)): visual (non-cryptographic) signatures, PAdES B-B/B-T signing with two-phase external signing, certification and field locking, timestamps, validation with revision-diff analysis and attack-catalogue tests, platform key storage and the optional TSA proxy. It fixes the op list, test plan, fixtures and PR stack before any code, as every editor round requires ([full-editor §7](../../plans/full-editor.md#7-phases-and-rounds): "each starts with a short feature-plan PR").

## Context

- [full-editor §E10](../../plans/full-editor.md#e10-signatures): the round as planned (ships, crates, save, web vs desktop)
- [06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) and [06 §7](../../research/pdf/06-feature-inventory.md#7-difficulty-map): mechanism and difficulty of each operation in scope
- [full-editor §3](../../plans/full-editor.md#3-tracking-the-capability-matrix) and [§5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round): capability-matrix statuses and the requirements every round meets
- [ADR 0006](../../decisions/0006-op-log-and-undo.md) §1 and [ADR 0007](../../decisions/0007-save-planner.md) §2: how each op is classified and how the planner gates it
- [ADR 0018](../../decisions/0018-test-strategy.md): test tiers the plan must assign (golden, corpus, differential, fuzz)

## Scope

**In scope**

- The feature plan `docs/plans/editor/e10-signatures.md`
- Capability-manifest rows for the round's operations
- Adjustments to the E10 implementation todos (split, merge, dependency fixes)

**Out of scope** (and which todo covers it)

- Any code: each implementation todo of the round

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `F.01` handwritten signature image; `F.02` draw/type e-signature; `F.03` certificate-based digital signatures; `F.04` verify signatures; `F.05` certify a document; `F.06` trusted timestamps; `F.07` inspect signer identity and certificates; `F.08` lock fields after signing; `F.09` allowed changes after certification; `F.10` signature status and modification warnings; `F.11` signing workflows via external services
- **Crates and packages:** (planned for the round) ADR 0020; `lamina-sign` over `lamina-save` (ByteRange placeholder, incremental append), RustCrypto + `cms`
- **`SaveConstraint`:** (round summary from the plan; the feature plan details it per op) signing makes the document `AppendOnly`; signing runs after any pending rewrite
- **Web vs desktop:** web: WebCrypto keys or PKCS#12 on the device; desktop adds OS keychains and PKCS#11
- **Server features:** optional TSA proxy Worker (hash only)
- **Capability-matrix rows to flip:** none flipped here; rows added with status `unsupported` and the round as target

## Tasks

- [ ] Create `docs/plans/editor/e10-signatures.md` (Status: Draft) with: goal; the operations in scope by inventory ID (`F.01`, `F.02`, `F.03`, `F.04`, `F.05`, `F.06`, `F.07`, `F.08`, `F.09`, `F.10`, `F.11`) and the target status of each; for each op its `Op` variant name, payload sketch (by `LaminaId`, no ambient inputs), `ModificationClass` and `requires_rewrite`
- [ ] Describe model and crate changes (new typed views, new modules, public API in `lamina` and `@lamina/pdf`), and the UI tools/panels in `@lamina/ui`
- [ ] Write the `SaveConstraint` table for the round: behaviour on `Free`, `AppendOnly` (DocMDP P1/P2/P3, FieldMDP), `RequiresRewrite` and `Conflict`, and the UI warnings
- [ ] List web vs desktop differences through `Platform` capability flags, and any server feature (off by default, opt-in, what leaves the device)
- [ ] List fixtures to add (with provenance and licence in `fixtures/README.md`), and the test plan per op: unit, golden, round-trip (lamina, pdf.js, PDFium, qpdf), differential, fuzz, Playwright and WebdriverIO e2e
- [ ] Add the round's rows to `docs/capabilities/manifest.toml` with `target round = "E10"` and status `unsupported` until each implementation PR flips them
- [ ] Confirm or adjust the round's implementation todos in `docs/todos/ready/` (E10c onwards): split or merge them so each stays one reviewable PR (≤ ~1,500 lines), and keep their `Depends on` explicit
- [ ] Answer or assign an owner to each open question below; record decisions in the feature plan
- [ ] Index the new plan in `docs/plans/README.md` and link it from [full-editor §E10](../../plans/full-editor.md#e10-signatures)

## Acceptance criteria

- [ ] The plan lists every operation in scope by inventory ID with an `Op` name, class and target status
- [ ] The `SaveConstraint` table covers Free, AppendOnly (P1/P2/P3), RequiresRewrite and Conflict
- [ ] Every implementation todo of the round is linked from the plan and each has explicit dependencies
- [ ] `docs/capabilities/manifest.toml` has a row per op with `target round = "E10"`; `cargo xtask capabilities` regenerates without error
- [ ] Each open question below is answered or has a named owner and a deadline round

## Tests

Docs only: the link checker passes; `cargo xtask capabilities` (from `X2`) accepts the new manifest rows.

## Docs to update

- `docs/plans/README.md` index
- [full-editor §E10](../../plans/full-editor.md#e10-signatures) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- Open question: Qualified electronic signatures via remote signing services (eIDAS QES): in scope, which providers? (full-editor §9 Q5)
- Open question: Which trust lists ship on web?
- The plan location `docs/plans/editor/` is a recommendation; if the plans README prescribes another place, use it and update this todo's links

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
