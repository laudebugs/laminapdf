# E10d — PAdES B-B signing core with two-phase external signing

| Field | Value |
| --- | --- |
| ID | `E10d` |
| Status | `ready` |
| Plan | [full-editor §E10](../../plans/full-editor.md#e10-signatures) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E9b](./E9b-field-creation-ops.md), [E10b](./E10b-signatures-feature-plan.md) |
| Blocks | [E10e](./E10e-timestamps-and-tsa-proxy.md), [E10f](./E10f-certification-and-locking.md), [E10g](./E10g-signature-validation.md), [E10i](./E10i-platform-signers.md), [E12d](./E12d-scrub-and-rewrite.md), [E18g](./E18g-leases-and-base-revisions.md) |
| ADRs | Implements ADR 0020; Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e10d-pades-signing-core` |
| PR title | `feat(sign): PAdES B-B signing with prepare/inject API` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Create `lamina-sign`: signature dictionary with `/ByteRange` and fixed-size `/Contents` placeholder (size estimation + retry), CMS SignedData detached (`ETSI.CAdES.detached`, SHA-256, `signing-certificate-v2`), visible or invisible widget, executed by the save planner as its last step; public two-phase API `prepare() → digest`, external sign, `inject(cms)`.

## Context

- ADR 0020
- [ADR 0007](../../decisions/0007-save-planner.md) §4
- [04 §8.2 ByteRange/Contents](../../research/pdf/04-interactive-security.md#82-the-byterange--contents-mechanism)
- [04 §8.5 PAdES baseline](../../research/pdf/04-interactive-security.md#85-pades-baseline-levels-etsi-en-319-142-1-current-v121-2024-01)
- ETSI EN 319 142-1, RFC 5652

## Scope

**In scope**

- `Sign` op (records request; barrier)
- Planner integration (signing revision assembled in memory; digest streamed)
- `Signer` trait and a software signer (PKCS#8 key)
- Two-phase API in `lamina` and `@lamina/pdf`

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `F.03` certificate-based digital signatures; `F.11` signing workflows via external services
- **Crates and packages:** new `lamina-sign` (core), `lamina-save`
- **`Op` variants:** `Sign` (class `Signature`, barrier)
- **`SaveConstraint`:** signing makes the document `AppendOnly`; adding a signature is allowed without certification and under DocMDP P2/P3, and is a violation under P1 (ADR 0007 §2); signing always runs last in a save (redact → rewrite → sign, ADR 0007 §4); `Sign` is a barrier op (ADR 0006 §5)
- **Web vs desktop:** identical in core; key sources in E10i
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `F.03` → `write` (core, pdf); `F.11` → `write` (core, pdf)

## Tasks

- [ ] Crate and deps (deny-checked)
- [ ] Placeholder and estimation
- [ ] CMS builder
- [ ] Planner step
- [ ] Two-phase API
- [ ] Tests

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Signed output validates in pyHanko, `pdfsig` (external), Adobe Reader (manual, recorded) and EU DSS validation (B-B)
- [ ] Signing on a base with a pending rewrite reason that exists already (a repaired base, ADR 0007 §2) produces the full rewrite then the signature as an incremental revision in one save (ADR 0007 §4); the same ordering with a pending redaction is tested in [E12d](E12d-scrub-and-rewrite.md), once `ApplyRedactions` exists
- [ ] Byte layout satisfies the SWA defences (a=0, exact gap, hex-only)

## Post-merge checks

None.

## Tests

- Validation by external tools in CI
- Determinism with a fixed key and time

## Docs to update

- API docs: signing
- [full-editor §E10](../../plans/full-editor.md#e10-signatures) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
- [ ] Squash-merged; dependants' "Depends on" now satisfied
