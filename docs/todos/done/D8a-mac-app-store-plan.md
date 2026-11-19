# D8a — Mac App Store: the channel's rules and plan

| Field | Value |
| --- | --- |
| ID | `D8a` |
| Status | `done` |
| Plan | [Desktop app §6 D8](../../plans/desktop-app.md#d8-mac-app-store) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D6c](./D6c-macos-signing-and-notarisation.md) |
| ADRs | Amends [0008](../../decisions/0008-desktop-shell.md) §1 and [0014](../../decisions/0014-desktop-distribution.md) (§11) |
| Branch | `docs/d8a-mac-app-store-plan` |
| PR title | `docs(desktop): the Mac App Store as a second install option (D8a)` |
| Size | S (docs only) |
| PR | — |

## Goal

The owner wants the Mac App Store as another way to install Lamina PDF on a Mac (2026-11-19). This todo records the channel's rules in ADR 0014 §11, points ADR 0008 at them, and plans the work as round D8.

## Tasks

- [x] ADR 0014 §11: a separate store build, App Sandbox with the entitlements a reader needs, security-scoped bookmarks, store signing, the release workflow's upload, crash reports unchanged
- [x] ADR 0008 §1: the distribution line and its alternative point at §11
- [x] Desktop plan: round D8, its to-dos and the answer to open question 1
- [x] Todos D8b, D8c and D8d

## Definition of done

- [x] Docs PR from a fresh worktree; `/writing-review` on the ADR and the plan
- [x] CI green
