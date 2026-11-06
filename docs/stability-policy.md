# Stability policy

The docs site publishes what a Lamina PDF version number promises as the [Stability policy](https://docs.laminapdf.com/reference/stability/) page. Its source, [`apps/docs/src/content/docs/reference/stability.mdx`](../apps/docs/src/content/docs/reference/stability.mdx), is the one place to change the policy. This file keeps what contributors need to apply it. The policy has been in force since the first registry release, `0.2.0-alpha.1`; versioning itself is [ADR 0017](decisions/0017-versioning-and-release.md).

## How the promises are checked

- **Breaking changes:** `cargo xtask semver`, the fast lane's `semver` job, compares the checkout with the newest published release. cargo-semver-checks (pinned in `tools/xtask/src/doctor.rs`) checks every published crate against crates.io, and an API Extractor report of each `@laminapdf/sdk` entry point is compared with the published package's. The policy puts breaking changes in a minor release while the version is `0.x`. So a breaking change fails the job unless the changesets give the fixed group a `minor` (or `major`) bump: the pending ones, plus, between a version PR and npm approving that version, the prepared version's step and the changesets it consumed. A crate missing from crates.io at the published version is listed as unchecked.
- **Installs and identical bytes:** `cargo xtask verify-install` runs the quick start from the packed packages before the release lane publishes, and `cargo xtask verify-install --from-registry` runs it from npm and crates.io after. Every runtime has to write `fixtures/golden/examples/hello.pdf`, byte for byte.
- **Codes, the error object and op JSON:** `crates/laminapdf-cos/tests/codes.rs` and `crates/laminapdf/tests/error_contract.rs` check the codes and the `laminapdf.error/1` object, and the frozen op samples in `fixtures/ops/v1/` load on every PR.

## Changing a promise

A change to what the policy promises is a decision: record it in an ADR (or amend ADR 0017), then edit the page. Which bump a change needs is in the [contributing guide's changesets section](guidelines/contributing.md#changesets).

## Changelog

[`CHANGELOG.md`](../CHANGELOG.md) is generated from the changesets by `cargo xtask version sync`. The docs site's [Changelog](https://docs.laminapdf.com/reference/changelog/) page shows its releases (`apps/docs/plugins/changelog.mjs`), without its title, generator comment and opening paragraph, and without each entry's commit hash.
