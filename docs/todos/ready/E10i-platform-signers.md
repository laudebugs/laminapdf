# E10i — Platform key storage: OS keychains and PKCS#11 (desktop), WebCrypto and PKCS#12 (web)

| Field | Value |
| --- | --- |
| ID | `E10i` |
| Status | `ready` |
| Plan | [full-editor §E10](../../plans/full-editor.md#e10-signatures) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [C3a](../done/C3a-viewer-worker-rpc.md), [D2c](./D2c-engine-sandbox.md), [E10d](./E10d-pades-signing-core.md) |
| Blocks | [E10j](./E10j-signature-ui.md), [E11e](./E11e-public-key-security.md) |
| Dependency notes | [E10d](E10d-pades-signing-core.md) (`Signer` trait), [D2c](D2c-engine-sandbox.md) (the engine sandbox policy that must allow keychain/CNG/NSS and PKCS#11 module access), [C3a](../done/C3a-viewer-worker-rpc.md) (the web worker that holds WebCrypto keys) |
| ADRs | Implements ADR 0020; Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e10i-platform-signers` |
| PR title | `feat(sign): platform signers for desktop and web` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Implement `Signer` adapters at the edges: macOS Keychain, Windows CNG/cert store, Linux NSS/PKCS#11; PKCS#11 smart cards and tokens on all desktops; WebCrypto non-extractable keys and imported PKCS#12 kept on the device on web; keys never leave the device.

## Context

- **Design (Paper):** [G02 Sign with a certificate](../../design/paper/sign-protect-redact.md#g02-sign-with-a-certificate). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- ADR 0020
- [full-editor §E10](../../plans/full-editor.md#e10-signatures) web vs desktop

## Scope

**In scope**

- Desktop Rust adapters (load `/tauri-v2`)
- Web adapter in the worker
- PKCS#12 import with password
- Certificate picker UI
- Adapter shape that leaves room for a key-unwrap/decrypt operation on the same key handles (implemented in [E11e](E11e-public-key-security.md))

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `F.03` certificate-based digital signatures
- **Crates and packages:** `apps/desktop/src-tauri` (edge crate e.g. `laminapdf-sign-native`), `laminapdf-wasm`/web worker
- **`SaveConstraint`:** signing makes the document `AppendOnly`; adding a signature is allowed without certification and under DocMDP P2/P3, and is a violation under P1 (ADR 0007 §2); signing always runs last in a save (redact → rewrite → sign, ADR 0007 §4); `Sign` is a barrier op (ADR 0006 §5)
- **Web vs desktop:** desktop: keychains + PKCS#11; web: WebCrypto + PKCS#12
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `F.03` → `write` (web, desktop) (signing with a platform key works in both apps; [E10j](E10j-signature-ui.md) flips it `write` → `round-trip` once sign → reopen → validate passes e2e)

## Tasks

- [ ] Adapters
- [ ] Picker
- [ ] Tests with SoftHSM

## Acceptance criteria

- [ ] Sign with a SoftHSM token in CI and an OS keychain test identity on each OS
- [ ] Web signing with a PKCS#12 never sends key material off-device (network assertions)

## Post-merge checks

None.

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

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
