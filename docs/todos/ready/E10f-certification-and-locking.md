# E10f — Certification (DocMDP), field locking (FieldMDP) and allowed changes

| Field | Value |
| --- | --- |
| ID | `E10f` |
| Status | `ready` |
| Plan | [full-editor §E10](../../plans/full-editor.md#e10-signatures) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E10d](./E10d-pades-signing-core.md) |
| Blocks | [E10h](./E10h-revision-diff-and-attacks.md) |
| ADRs | Implements ADR 0020; Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e10f-certification-and-locking` |
| PR title | `feat(sign): certification signatures and field locking` |
| Size | M (200–600) |
| PR | — |

## Goal

Certify documents (DocMDP transform with P1/P2/P3 on the first signature), lock fields after signing (`/Lock`, FieldMDP transform), and make the planner's violation rules match exactly what each level allows.

## Context

- [04 §8.6 Certification vs approval; MDP](../../research/pdf/04-interactive-security.md#86-certification-vs-approval-signatures-mdp)
- [ADR 0007](../../decisions/0007-save-planner.md) §2

## Scope

**In scope**

- Certify option on `Sign`
- Lock dict on signature fields
- Planner rule table tests

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `F.05` certify a document; `F.08` lock fields after signing; `F.09` allowed changes after certification
- **Crates and packages:** `lamina-sign`, `lamina-save`
- **`Op` variants:** `Sign { certify: Some(P) , lock: .. }`
- **`SaveConstraint`:** signing makes the document `AppendOnly`; adding a signature is allowed without certification and under DocMDP P2/P3, and is a violation under P1 (ADR 0007 §2); signing always runs last in a save (redact → rewrite → sign, ADR 0007 §4); `Sign` is a barrier op (ADR 0006 §5)
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `F.05` → `round-trip` (core, pdf); `F.08` → `round-trip` (core, pdf); `F.09` → `write` (core, pdf) (the planner enforces each level's allowed changes; [E10h](E10h-revision-diff-and-attacks.md) flips it `write` → `round-trip` once existing revisions are checked against them; web and desktop columns flip in [E10j](E10j-signature-ui.md))

## Tasks

- [ ] Transforms
- [ ] Rules
- [ ] Tests

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Every new op reports exactly one `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug. Expected for `Sign { certify }`: certification is only allowed as the first signature (a violation on every already-signed fixture); `Sign { lock }` follows E10d
- [ ] Certified outputs show the correct permissions in Adobe Reader (manual) and pyHanko
- [ ] Every op class × P level verdict matches ISO 32000-2 §12.8.2.2 (table test)

## Post-merge checks

None.

## Tests

- Rule table tests

## Docs to update

- User help: certifying
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
