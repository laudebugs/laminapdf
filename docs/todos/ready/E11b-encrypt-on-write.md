# E11b — AES-256 encryption on write, passwords and permissions; inspect encryption

| Field | Value |
| --- | --- |
| ID | `E11b` |
| Status | `ready` |
| Plan | [full-editor §E11](../../plans/full-editor.md#e11-security-and-sanitisation) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [B2b](./B2b-standard-security-decryption.md), [E11a](./E11a-security-and-sanitisation-feature-plan.md) |
| Blocks | [E11c](./E11c-remove-security.md), [E11e](./E11e-public-key-security.md) |
| Dependency notes | [E11a](E11a-security-and-sanitisation-feature-plan.md), `B2` ([pdf-library §B2](../../plans/pdf-library.md#b2-document-model): document model and decryption) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e11b-encrypt-on-write` |
| PR title | `feat(save): AES-256 encryption on write with passwords and permissions` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Encrypt on full rewrite with the standard security handler R6 (AES-256, `/V 5`, crypt filters `/StdCF` AESV3, `/Perms`, `/EncryptMetadata`), user and owner passwords, `/P` permission bits; keep encryption when saving an encrypted base incrementally (removing today's `EncryptedBase` rewrite reason); and an encryption inspector.

## Context

- [04 §7.2 Standard security handler revisions](../../research/pdf/04-interactive-security.md#72-standard-security-handler-revisions)
- [04 §7.4 Permission bits](../../research/pdf/04-interactive-security.md#74-permission-bits-p)
- [04 §7.7 PDFex](../../research/pdf/04-interactive-security.md#77-pdfex-müller-et-al-acm-ccs-2019)
- [ADR 0007](../../decisions/0007-save-planner.md) §2 rewrite reasons
- ISO 32000-2 §7.6.4

## Scope

**In scope**

- Crypt filter encryption in `lamina-syntax` (RustCrypto `aes`, `cbc`, `sha2`; random IVs from an injected RNG — no ambient randomness in core; shells pass entropy)
- `SetEncryption { user, owner, permissions, encrypt_metadata }` op (requires rewrite)
- Incremental saves of encrypted bases encrypt new objects with the existing key
- PDFex mitigations: refuse partially encrypted output; `/Perms` consistency
- Inspector panel (handler, revision, key length, permissions)
- Passwords and permissions dialog

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `G.01` document-open password; `G.02` encrypt a PDF; `G.03` permission passwords; `G.04` restrict print/copy/edit; `G.11` inspect encryption
- **Crates and packages:** `lamina-syntax` (crypt filters write), `lamina-save`, `lamina-ops`, `@lamina/ui`
- **`Op` variants:** `SetEncryption` (class `Document`, `requires_rewrite = true`, barrier per ADR 0006 §5)
- **`SaveConstraint`:** `RequiresRewrite`: a full rewrite, every earlier revision dropped. On signed files the constraint is `Conflict`: the UI names the signatures that will be invalidated and removed (ADR 0007 §3 override); order redact → rewrite → sign
- **Web vs desktop:** identical; passwords never leave the device or the worker
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `G.01` → `round-trip` (core, pdf, web, desktop); `G.02` → `round-trip` (core, pdf, web, desktop); `G.03` → `round-trip` (core, pdf, web, desktop); `G.04` → `round-trip` (core, pdf, web, desktop); `G.11` → `read` (core, pdf, web, desktop)

## Tasks

- [ ] Encryption writer
- [ ] RNG injection interface
- [ ] Op + barrier semantics
- [ ] Incremental encrypted saves
- [ ] Dialogs
- [ ] Tests

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Encrypted outputs open in pdf.js, PDFium, qpdf (`--check` with password) and Acrobat (manual); wrong passwords fail
- [ ] Incremental edits on encrypted files stay encrypted (no plaintext objects: scan test)
- [ ] Determinism holds when tests inject a fixed RNG

## Post-merge checks

None.

## Tests

- Interop with qpdf and pypdf decryption
- Plaintext scan test
- Fuzz on encrypted inputs

## Docs to update

- API docs: encryption
- Security notes: permissions are advisory
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
