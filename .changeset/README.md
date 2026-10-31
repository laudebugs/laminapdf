# Changesets

Each Markdown file here is a **changeset**: which packages a change releases, the bump, and the
changelog entry. `@laminapdf/wasm` and `@laminapdf/pdf` are one fixed group, and `cargo xtask version
sync` gives every crate the same version, so the whole library moves in lockstep
([ADR 0017](../docs/decisions/0017-versioning-and-release.md)).

- Add one with `pnpm changeset` in any PR that changes what users of the library see. See
  [contributing: changesets](../docs/guidelines/contributing.md#changesets) for the bump to pick.
- Release prep: `pnpm changeset version` (bumps both packages and writes their `CHANGELOG.md`),
  then `cargo xtask version sync` (the crates, `Cargo.lock` and the root `CHANGELOG.md`).
- `pre.json` exists while the versions are pre-releases (`-alpha.N`).

Nothing is published from here: publishing is [P1](../docs/todos/ready/P1-reserve-names-and-trusted-publishing.md).
