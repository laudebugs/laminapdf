# Contributing

Start here. This page is the checklist. The linked guides have the rules and the reasons behind them.

## Before you start

1. Read [`CLAUDE.md`](../../CLAUDE.md) for the working agreements and commands.
2. For PDF features, read the relevant chapter in [`docs/research/`](../research/) and the ISO 32000 section first (*spec first*).
3. Check [`docs/decisions/`](../decisions/) for ADRs that constrain the change.

## The workflow in brief

1. **Create a new git worktree off up-to-date `main`** with a branch named `<type>/<short-description>`: `git worktree add -b feat/xref-repair feat-xref-repair main`. See [git-workflow.md](git-workflow.md).
2. Make one focused change, following [code-organization.md](code-organization.md) and the language guide ([rust.md](rust.md), [typescript.md](typescript.md)).
3. Commit with [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/): `type(scope): summary`. Types: `feat`, `fix`, `docs`, `refactor`, `test`, `perf`, `build`, `ci`, `chore`, `spike`.
4. Push and open a pull request against `main`, with a summary, sources and a test plan. Don't push directly to `main`.
5. Get a review, address every comment, and merge once checks pass.
6. Remove the worktree and branch.

## Changesets

lamina has one version for every crate and every npm package: `@laminapdf/wasm`, `@laminapdf/sdk`, `@laminapdf/render`, `@laminapdf/render-hayro` and `@laminapdf/render-pdfium` ([ADR 0017](../decisions/0017-versioning-and-release.md)). A PR that changes what users of the library see (an API, behaviour, output bytes, a fix they'd notice) adds a **changeset**: run `pnpm changeset`, pick the packages the change touches (they are one fixed group, so any of them bumps all five, and the root `CHANGELOG.md` merges every package's entries, so naming only a render package is enough), choose the bump and write one or two sentences for the changelog. It writes `.changeset/<name>.md`; commit it with the change. Docs, CI, tests and internal refactors need none.

- **Bump while the version is `0.x`:** `minor` for a breaking change (see the [stability policy](../stability-policy.md) for what counts), `patch` for everything else. Never `major`.
- **Never edit a version by hand.** `cargo xtask check` runs `cargo xtask version sync --check`, which fails when a crate, `Cargo.lock`, `CHANGELOG.md` or `@laminapdf/sdk`'s `VERSION` (`packages/sdk/src/version.ts`) disagrees with the packages' version, or when the fixed group's packages disagree, or when a crate sets its own version instead of `version.workspace = true`.
- **Releases** (the owner, or an agent asked to): `pnpm changeset version` consumes the changesets, bumps the five packages and writes their changelogs; `cargo xtask version sync` then writes the version into every crate, the lockfiles, the root `CHANGELOG.md` and `packages/sdk/src/version.ts`. Commit both in one PR; after it merges, the owner tags `v<version>` on `main`, which runs the release lane (`.github/workflows/release.yml`). Nothing is published.
- While the version is a pre-release (`-alpha.N`), Changesets is in pre mode (`.changeset/pre.json`): consumed changesets move to `.changeset/pre/`.

## Documentation

- Anything worth remembering goes in `docs/`. See [`docs/README.md`](../README.md) for where.
- Significant technical decisions get an ADR in `docs/decisions/`.
- Add every new doc to its folder's `README.md` index.
- Cite primary sources, and label recommendations and opinions as such.

## Dependencies and licences

Check the licence of every new dependency, fixture PDF and font before adding it. No AGPL/GPL/LGPL code goes into shipped artefacts. See [research ch. 05 §8.4](../research/pdf/05-creating-pdfs-from-code.md#84-licensing-traps-to-avoid).
