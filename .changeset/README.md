# Changesets

Each Markdown file here is a **changeset**: which packages a change releases, the bump, and the
changelog entry. `@laminapdf/wasm`, `@laminapdf/pdf`, `@laminapdf/render`, `@laminapdf/render-hayro` and
`@laminapdf/render-pdfium` are one fixed group (`fixed` in `config.json`, which `cargo xtask version
sync` reads, so a new member needs no code change), and the sync gives every crate the same version, so the whole library moves in lockstep
([ADR 0017](../docs/decisions/0017-versioning-and-release.md)).

- Add one with `pnpm changeset` in any PR that changes what users of the library see. See
  [contributing: changesets](../docs/guidelines/contributing.md#changesets) for the bump to pick.
- Release prep: `pnpm changeset version` (bumps the five packages and writes their `CHANGELOG.md`),
  then `cargo xtask version sync` (the crates, `Cargo.lock`, the root `CHANGELOG.md` merged from
  every member's, and
  `@laminapdf/pdf`'s `VERSION` in `packages/pdf/src/version.ts`).
- `pre.json` exists while the versions are pre-releases (`-alpha.N`).

Nothing is published from here: publishing is [P1](../docs/todos/ready/P1-reserve-names-and-trusted-publishing.md).
