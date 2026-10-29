# E11c — Remove security with authorisation and change passwords

| Field | Value |
| --- | --- |
| ID | `E11c` |
| Status | `ready` |
| Plan | [full-editor §E11](../../plans/full-editor.md#e11-security-and-sanitisation) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E11b](./E11b-encrypt-on-write.md) |
| Blocks | None |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e11c-remove-security` |
| PR title | `feat(save): remove or change document security` |
| Size | S (< 200 lines) |
| PR | — |

## Goal

Remove encryption (or change passwords/permissions) after authenticating with the owner password (or user password when it grants full rights, per E11a policy), as a full rewrite so no old encrypted/plaintext revisions survive.

## Context

- **Design (Paper):** [G06 Protect](../../design/paper/sign-protect-redact.md#g06-protect). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [04 §7.5 User vs owner passwords](../../research/pdf/04-interactive-security.md#75-user-vs-owner-passwords-why-owner-only-protection-isnt-security)

## Scope

**In scope**

- `RemoveEncryption` op with authorisation check
- UI flow

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `G.05` remove security with authorisation
- **Crates and packages:** `lamina-save`, `lamina-ops`
- **`Op` variants:** `RemoveEncryption` (requires rewrite, barrier)
- **`SaveConstraint`:** `RequiresRewrite`: a full rewrite, every earlier revision dropped. On signed files the constraint is `Conflict`: the UI names the signatures that will be invalidated and removed (ADR 0007 §3 override); order redact → rewrite → sign
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `G.05` → `round-trip` (core, pdf, web, desktop)

## Tasks

- [ ] Op
- [ ] Authorisation
- [ ] UI

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Every new op reports exactly one `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug. Expected for `RemoveEncryption` (`requires_rewrite`): `Conflict` on every signed fixture
- [ ] Output has no `/Encrypt` and no earlier revision (single xref section test)

## Post-merge checks

None.

## Tests

- Round-trip

## Docs to update

- User help: security
- [full-editor §E11](../../plans/full-editor.md#e11-security-and-sanitisation) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
