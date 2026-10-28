# Guidelines

How we work. Start with [contributing.md](contributing.md).

| Guideline | Summary |
| --- | --- |
| [contributing.md](contributing.md) | Entry point: the contribution checklist, linking to everything below. |
| [git-workflow.md](git-workflow.md) | Worktree-per-task workflow off `main`, branch naming, Conventional Commits, PRs, review, cleanup. |
| [code-organization.md](code-organization.md) | Monorepo layout (`crates/`, `packages/`, `apps/`, `fixtures/`, `tools/`, `spikes/`), dependency direction, naming, public API, tests, config, generated code, when to split. |
| [rust.md](rust.md) | Rust conventions: edition/MSRV, workspace lints, unsafe policy, errors, API design, tests, docs, dependencies. |
| [typescript.md](typescript.md) | TypeScript conventions: strict tsconfig, ESM-only, portability, Biome, Vitest, naming, errors, types-first API design. |

Skills: Rust work uses the `rust-best-practices` skill and Tauri work uses the `tauri-v2` skill (see [CLAUDE.md](../../CLAUDE.md) and [rust.md](rust.md)).
