# laminapdf-test-fixtures

The Rust test helpers that Lamina PDF's crates share, so no crate keeps a copy:

- `fixtures_dir`, `fixture`, `fixture_text`, `json` and `corpus` read the repository's shared `fixtures/` folder.
- `classic`, `classic_with_root` and `stream_body` build small classic PDF files byte by byte, for tests that need exact bytes rather than what a writer produces.
- `Rng` is a seeded xorshift64* generator, so a failure found with a seed can be reproduced.
- `read_at` is the body of a `ByteSource::read_at` fake over a byte slice.
- `codes` and `code_groups` read `fixtures/codes.json` for the tests that check each crate's error kinds; `cargo xtask gen codes` uses `code_groups` too.

The crate depends on no Lamina PDF crate, so any crate can take it as a dev-dependency, unit tests included, without seeing a second copy of itself. It is never published: crates list it by path only, and `cargo publish` drops such dev-dependencies. A helper that needs a crate's own types, such as opening a document or the spike-era writer, stays in that crate's `tests/common/`. See code-organization.md §6.

```sh
cargo test -p laminapdf-test-fixtures
```
