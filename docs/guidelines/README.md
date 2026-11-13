# Guidelines

How we work. Start with [contributing.md](contributing.md).

| Guideline | Summary |
| --- | --- |
| [contributing.md](contributing.md) | Entry point: the contribution checklist, linking to everything below. |
| [git-workflow.md](git-workflow.md) | Worktree-per-task workflow off `main`, branch naming, Conventional Commits, PRs, review, cleanup. |
| [code-organization.md](code-organization.md) | Monorepo layout (`crates/`, `packages/`, `apps/`, `fixtures/`, `tools/`, `spikes/`), dependency direction, naming, public API, tests, config, generated code, when to split. |
| [rust.md](rust.md) | Rust conventions: edition/MSRV, workspace lints, unsafe policy, errors, API design, tests, docs, dependencies. |
| [typescript.md](typescript.md) | TypeScript conventions: strict tsconfig, ESM-only, portability, Biome, Vitest, naming, errors, types-first API design. |
| [maintainability.md](maintainability.md) | Code that stays easy to change: one home per concept, size budgets that only go down, one public path per item, no test code in shipped code. |
| [docs-writing.md](docs-writing.md) | Reader-facing docs: the site's story order, plain English, no internal IDs, one term per thing, code introduced by a sentence. |
| [bug-triage.md](bug-triage.md) | Working a bug from report to cause: reproduce it as the user does, tell slow from stuck, measure each stage, log every failure, fix the cause, prove it, then record it in `docs/bugs/` and report back. |
| [examples.md](examples.md) | Code examples readers can copy: public API only, no test scaffolding, `using`, named numbers, small functions, matching TS and Rust pairs. |
| [desktop-release.md](desktop-release.md) | Desktop release and signing runbook: the owner's signing setup (Apple, Azure Artifact Signing, GPG, updater keys), the secrets inventory, verification commands, renewals and incidents, SmartScreen. |

Skills: Rust work uses the `rust-best-practices` skill and Tauri work uses the `tauri-v2` skill (see [CLAUDE.md](../../CLAUDE.md) and [rust.md](rust.md)). Prose people read is drafted with `/writing` and reviewed with `/writing-review` ([git workflow §7](git-workflow.md#7-review-and-merge-process)).
