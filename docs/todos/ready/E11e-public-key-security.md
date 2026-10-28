# E11e — Public-key (certificate) security: read, write behind a flag

| Field | Value |
| --- | --- |
| ID | `E11e` |
| Status | `ready` |
| Plan | [full-editor §E11](../../plans/full-editor.md#e11-security-and-sanitisation) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E10g](./E10g-signature-validation.md), [E11b](./E11b-encrypt-on-write.md) |
| Blocks | None |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e11e-public-key-security` |
| PR title | `feat(crypto): public-key security handler` |
| Size | M (200–600) |
| PR | — |

## Goal

Open documents encrypted with the public-key handler (`/Adobe.PubSec`, `adbe.pkcs7.s5`, recipients in crypt filters) using a private key from the platform signers (E10i), and write them behind an experimental flag.

## Context

- [04 §7.6 Public-key security handler](../../research/pdf/04-interactive-security.md#76-public-key-security-handler-765-17-764)

## Scope

**In scope**

- Recipient decryption (CMS EnvelopedData)
- Writer behind `experimental_pubsec`

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `G.12` certificate-based security
- **Crates and packages:** `lamina-syntax`, `lamina-sign` (CMS)
- **`SaveConstraint`:** `RequiresRewrite`: a full rewrite, every earlier revision dropped. On signed files the constraint is `Conflict`: the UI names the signatures that will be invalidated and removed (ADR 0007 §3 override); order redact → rewrite → sign (when writing)
- **Web vs desktop:** key sources as E10i
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `G.12` → `read` (core, pdf, web, desktop)

## Tasks

- [ ] Read
- [ ] Write (flagged)
- [ ] Fixtures from Acrobat

## Acceptance criteria

- [ ] Acrobat-made certificate-encrypted fixtures open with the right key and fail clearly otherwise

## Tests

- Fixtures

## Docs to update

- API docs
- [full-editor §E11](../../plans/full-editor.md#e11-security-and-sanitisation) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
