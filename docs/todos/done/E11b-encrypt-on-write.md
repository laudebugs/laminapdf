# E11b — AES-256 encryption on write, passwords and permissions; inspect encryption

| Field | Value |
| --- | --- |
| ID | `E11b` |
| Status | `done` |
| Plan | [full-editor §E11](../../plans/full-editor.md#e11-security-and-sanitisation) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [B2b](./B2b-standard-security-decryption.md), [E11a](./E11a-security-and-sanitisation-feature-plan.md) |
| Dependency notes | [E11a](../done/E11a-security-and-sanitisation-feature-plan.md), `B2` ([pdf-library §B2](../../plans/pdf-library.md#b2-document-model): document model and decryption) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e11b-encrypt-on-write` |
| PR title | `feat(save): AES-256 encryption on write with passwords and permissions` |
| Size | L (600–1,500) |
| PR | [#860](https://github.com/laudebugs/laminapdf/pull/860) |

## Goal

Encrypt on full rewrite with the standard security handler R6 (AES-256, `/V 5`, crypt filters `/StdCF` AESV3, `/Perms`, `/EncryptMetadata`), user and owner passwords, `/P` permission bits; keep encryption when saving an encrypted base incrementally (`EncryptedBase` is now a rewrite reason only when the options remove the encryption); and an encryption inspector.

## Context

- **Design (Paper):** [G06 Protect](../../design/paper/sign-protect-redact.md#g06-protect). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [E11 feature plan](../../plans/editor/e11-security-and-sanitisation.md): the ops, the save table (§5), the fixtures (§8) and the tests (§9) this todo follows
- [04 §7.2 Standard security handler revisions](../../research/pdf/04-interactive-security.md#72-standard-security-handler-revisions)
- [04 §7.4 Permission bits](../../research/pdf/04-interactive-security.md#74-permission-bits-p)
- [04 §7.7 PDFex](../../research/pdf/04-interactive-security.md#77-pdfex-müller-et-al-acm-ccs-2019)
- [ADR 0007](../../decisions/0007-save-planner.md) §2 rewrite reasons
- ISO 32000-2 §7.6.4

## Scope

**In scope**

- Crypt filter encryption in `laminapdf-syntax` (RustCrypto `aes`, `cbc`, `sha2`; random IVs from an injected RNG — no ambient randomness in core; shells pass entropy)
- `SetEncryption { user, owner, permissions, encrypt_metadata }` op (requires rewrite)
- Incremental saves of encrypted bases encrypt new objects with the existing key
- PDFex mitigations: refuse partially encrypted output; `/Perms` consistency
- `encryption_info()`: handler, revision, key length, cipher and permissions, for the inspector (G.11)
- `Secret` passwords, the `Entropy` hand-off from the SDK worker and native shells, and the `encryption-*` error codes (plan §2.1)
- Once encrypted files take edits: Comment mode's tools disabled, with the reason, on a file whose permissions forbid annotating (ISO 32000-2 Table 22 bit 6), and a test of it ([E4f](../done/E4f-annotation-tools-ui.md) disables them only through the session's `encrypted` block)

**Out of scope** (and which todo covers it)

- The Protect dialog, the status-bar security item and the encryption inspector panel: [E11f](../ready/E11f-protect-dialog-and-inspector.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `G.01` document-open password; `G.02` encrypt a PDF; `G.03` permission passwords; `G.04` restrict print/copy/edit; `G.11` inspect encryption
- **Crates and packages:** `laminapdf-syntax` (crypt filters write), `laminapdf-save`, `laminapdf-ops`, `laminapdf`, `@laminapdf/sdk`, `@laminapdf/ui` (the permission reasons only)
- **`Op` variants:** `SetEncryption` (class `Document`, `requires_rewrite = true`, barrier per ADR 0006 §5)
- **`SaveConstraint`:** `RequiresRewrite`: a full rewrite, every earlier revision dropped. On signed files the constraint is `Conflict`: the UI names the signatures that will be invalidated and removed (ADR 0007 §3 override); order redact → rewrite → sign
- **Web vs desktop:** identical; passwords never leave the device or the worker
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `G.01`, `G.02`, `G.03`, `G.04` → `round-trip` (core, pdf); `G.11` → `read` (core, pdf). The web and desktop columns flip in E11f

## Tasks

- [x] Encryption writer
- [x] RNG injection interface
- [x] Op + barrier semantics
- [x] Incremental encrypted saves
- [x] Permission-based disabling in Comment mode, with the reason
- [x] Tests

## Acceptance criteria

- [x] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [x] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Encrypted outputs open in pdf.js, PDFium, qpdf (`--check` with password) and Acrobat (manual); wrong passwords fail
- [x] Incremental edits on encrypted files stay encrypted (no plaintext objects: scan test)
- [x] Determinism holds when tests inject a fixed entropy seed

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

- **Acrobat is not tried.** The AC's manual check in Acrobat has not been done: qpdf (`--check` with the password), pypdf, pdftotext, pdf.js and PDFium open the files (the round-trip lane's tools). A person with Acrobat should open `fixtures/encrypted/written/r6-user.pdf` (password `open-pw`) and `r6-owner-permissions.pdf`, and say so on the PR.
- **Deviations from the plan, which the plan now records:** the entropy is a 32-byte seed in `SaveOptions` (expanded by `SeededEntropy`), not a trait object, so the options stay `Copy`; a `Secret` serialises as `null`, not `"<secret>"`, since the placeholder could be a password; `r6-signed.pdf` waits for E10d, the first todo that writes a signature.
- **The viewer still blocks edits to an encrypted file** (`edit.blocked.encrypted`): its edits are drawn from a copy the render backend opens without the password, and keeping the password in the worker for that is a design change of its own. E11b builds the permission gating (`Forbidden`, `forbidden` on the open result and the session state, the reason on every comment command, tested) and E11f lifts the block and extends the gating to the other modes (E11f's scope says so).
- **Deferred from the plan, each with the todo that takes it:**
  - `Restriction::Permissions` (the advisory warning after a `SetEncryption` that forbids something) and the veraPDF check that bit 10 keeps PDF/UA clean: [E11f](../ready/E11f-protect-dialog-and-inspector.md), which draws the warning.
  - Making `SetEncryption` unpersistable in a recovery copy or crash report: no recovery copy of an op log exists yet, and the op serialises its passwords as `null`, so nothing can leak into one. The test greps the op JSON, `Debug` output and the save error; toasts and crash reports have no password to carry until E11f's dialog.
  - **Authorisation for changing the passwords of an encrypted file.** `SetEncryption` over an encrypted base does not yet check `matched_password()` or `/P`, so a restricted user password could re-save the file with every permission. The permission bits are advisory (plan §10, item 2) and that user can already read and copy the file, but the plan's §2.2 check belongs in the core: [E11c](../ready/E11c-remove-security.md) adds it for `RemoveEncryption` and for this case.
  - A fuzz target for the `/Encrypt` dictionary: the written files seed the `open` target, which reads them with each fixture password.
- **Bit 4 and an empty owner password:** bit 4 (edit contents) is off only when the caller gives a permissions password. With none, the save makes one but leaves editing allowed, since the person set no limit on editing; the plan's wording ("whenever a permissions password is set") reads that way.
- **`encrypted-base` is now for the spike-era test writers only:** the planner keeps an encrypted base's protection and adds `RewriteReason::EncryptedBase` only when `with_remove_encryption` asks to drop it (E11c turns that option into the `RemoveEncryption` op).

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
