# fuzz

cargo-fuzz targets for lamina's parsers and readers ([ADR 0018](../docs/decisions/0018-test-strategy.md) tier 9), in the layout OSS-Fuzz expects for Rust (`fuzz/Cargo.toml`, `fuzz/fuzz_targets/<name>.rs`). This folder is its own Cargo workspace, excluded from the root one (`exclude = ["spikes", "fuzz"]`; `cargo xtask portability` checks it), because libFuzzer needs nightly Rust. The root `rust-toolchain.toml` pin does not apply: every command here runs as `cargo +nightly …`.

| Target | Drives | Seeds |
| --- | --- | --- |
| `lex` | `Lexer::next_token` until the end or an error; asserts the lexer always advances | `fixtures/corpus`, `fixtures/golden` |
| `parse_object` | `parse_object_with` and `Parser::indirect_object` (with a failing `/Length` resolver, so the `endstream` scan runs); whatever parses must serialise | as above |
| `flate_decode` | `filters::flate_decode`, and the filter pipeline with PNG predictor parameters from the first four bytes | the stream data of every fixture |
| `open` | `Document::open`, then every read path and model view (`fuzz/src/lib.rs`, as `tests/common::exercise` does), first with the empty password, then with each line of `seeds/open/passwords.txt` | fixtures, plus everything under `seeds/open/` |
| `page_tree` | the page-tree walk and page views; input that is not a whole file becomes the body of the `/Pages` object in a small file | fixtures, plus `seeds/page_tree/` |

Invariants: no panic, no hang (`-timeout=10`), memory under `-rss_limit_mb=2048`, default [`Limits`](../docs/decisions/0003-reader-tolerance-and-limits.md).

## Running

```sh
rustup toolchain install nightly --profile minimal
cargo install cargo-fuzz --locked --version 0.13.2    # or: cargo xtask doctor --install
cargo xtask fuzz <target> --time 600                  # default 60 s
cargo xtask fuzz seeds [<target>]                     # only build fuzz/corpus/<target>/
```

`cargo xtask fuzz` first builds the seed corpus `fuzz/corpus/<target>/` (gitignored) from `fixtures/corpus/`, `fixtures/golden/`, the target's files in `fixtures/regressions/` and everything under `fuzz/seeds/<target>/` (except `passwords.txt`), then runs `cargo +nightly fuzz run <target> fuzz/corpus/<target> -- -max_total_time=<N> -timeout=10 -rss_limit_mb=2048`. The scheduled workflow [`.github/workflows/fuzz.yml`](../.github/workflows/fuzz.yml) runs every target for 10 minutes each night (and on demand) and uploads crashes as artefacts.

Encrypted seeds are maintained with decryption ([B2b](../docs/todos/ongoing/B2b-standard-security-decryption.md)): the files go in `seeds/open/encrypted/` and their passwords, one per line, in `seeds/open/passwords.txt` (lines starting with `#` are ignored).

## When a target finds a crash

1. Reproduce: `cargo +nightly fuzz run <target> fuzz/artifacts/<target>/<crash-file>`.
2. Minimise: `cargo +nightly fuzz tmin <target> fuzz/artifacts/<target>/<crash-file>`.
3. Commit the minimised input as `fixtures/regressions/<target>-<hash>.pdf` (the hash from the artefact name), list it in `fixtures/README.md`, and add a named regression test that opens it through the public API (in the crate whose code was wrong) and fails before the fix. The seed step picks the file up from then on.
4. Fix the bug in the same PR.

## Licence

`libfuzzer-sys` is `(MIT OR Apache-2.0) AND NCSA` (it bundles LLVM's libFuzzer). It is a fuzzing harness, never a dependency of a shipped crate, and lives outside the root workspace, so `deny.toml`'s allow-list does not apply to it.
