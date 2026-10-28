# E10i — Platform key storage: OS keychains and PKCS#11 (desktop), WebCrypto and PKCS#12 (web)

| Field | Value |
| --- | --- |
| ID | `E10i` |
| Status | `ready` |
| Plan | [full-editor §E10](../../plans/full-editor.md#e10-signatures) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [D4c](./D4c-native-printing.md), [E10d](./E10d-pades-signing-core.md) |
| Blocks | [E10j](./E10j-signature-ui.md) |
| Dependency notes | [E10d](E10d-pades-signing-core.md), `D4` ([desktop-app §D4](../../plans/desktop-app.md#d4-windows-tabs-menus-and-printing): windows, tabs, menus) |
| ADRs | Implements ADR 0020; Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e10i-platform-signers` |
| PR title | `feat(sign): platform signers for desktop and web` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Implement `Signer` adapters at the edges: macOS Keychain, Windows CNG/cert store, Linux NSS/PKCS#11; PKCS#11 smart cards and tokens on all desktops; WebCrypto non-extractable keys and imported PKCS#12 kept on the device on web; keys never leave the device.

## Context

- ADR 0020
- [full-editor §E10](../../plans/full-editor.md#e10-signatures) web vs desktop

## Scope

**In scope**

- Desktop Rust adapters (load `/tauri-v2`)
- Web adapter in the worker
- PKCS#12 import with password
- Certificate picker UI

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `F.03` certificate-based digital signatures
- **Crates and packages:** `apps/desktop/src-tauri` (edge crate e.g. `lamina-sign-native`), `lamina-wasm`/web worker
- **`SaveConstraint`:** signing makes the document `AppendOnly`; signing always runs last in a save (redact → rewrite → sign, ADR 0007 §4); `Sign` is a barrier op (ADR 0006 §5)
- **Web vs desktop:** desktop: keychains + PKCS#11; web: WebCrypto + PKCS#12
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `F.03` → `round-trip` (web, desktop)

## Tasks

- [ ] Adapters
- [ ] Picker
- [ ] Tests with SoftHSM

## Acceptance criteria

- [ ] Sign with a SoftHSM token in CI and an OS keychain test identity on each OS
- [ ] Web signing with a PKCS#12 never sends key material off-device (network assertions)

## Tests

- SoftHSM in CI
- Playwright network assertions

## Docs to update

- User help: signing certificates
- [full-editor §E10](../../plans/full-editor.md#e10-signatures) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
