# fuzz

cargo-fuzz targets for lamina's parsers and readers ([ADR 0018](../docs/decisions/0018-test-strategy.md) tier 9), in the layout OSS-Fuzz expects for Rust (`fuzz/Cargo.toml`, `fuzz/fuzz_targets/<name>.rs`). This folder is its own Cargo workspace, excluded from the root one (`exclude = ["spikes", "fuzz"]`; `cargo xtask portability` checks it), because libFuzzer needs nightly Rust. The root `rust-toolchain.toml` pin does not apply: every command here runs as `cargo +nightly …`.

| Target | Drives | Seeds |
| --- | --- | --- |
| `lex` | `Lexer::next_token` until the end or an error; asserts the lexer always advances | `fixtures/corpus`, `fixtures/golden` |
| `parse_object` | `parse_object_with` and `Parser::indirect_object` (with a failing `/Length` resolver, so the `endstream` scan runs); whatever parses must serialise | as above |
| `flate_decode` | `filters::flate_decode` on the input minus its last four bytes, and the filter pipeline with PNG predictor parameters from those four bytes | the stream data of every fixture, each followed by `[1, 1, 8, 1]` (no predictor) |
| `open` | `Document::open`, then every read path and model view, each view on its own (`exercise.rs`), first with the empty password, then with each line of `seeds/open/passwords.txt` | fixtures, plus everything under `seeds/open/` |
| `ops_json` | `OpLog::from_json`, then the log applied to `corpus/hello.pdf` and `corpus/outline.pdf` as one transaction, saved (`Auto`) and reopened; and op by op, all undone, after which a save must write the base unchanged | `fixtures/ops/v1/` |
| `save_roundtrip` | eight control bytes (mode, overrides, xref layout, numbering; seven steps of rotate, set title, undo, redo) before a PDF: open, apply, save; every saved file must reopen, and a full rewrite must rebase | every fixture PDF (and `fixtures/signed/`) after fixed control bytes |
| `page_tree` | the page-tree walk and page views; input that is not a whole file becomes the body of the `/Pages` object in a small file with a page and an intermediate node (with its own page) to inherit through | fixtures, plus `seeds/page_tree/` |
| `content` | the content-stream interpreter (`lamina-content`, [ADR 0023](../docs/decisions/0023-content-stream-reading.md)) on every page; input that is not a whole file becomes the page content of a small file whose resources hold a font, an image, a form that uses the page's resources, a form that draws itself, a graphics state, a shading, a pattern, a colour space and a property list. Also asserts balanced marked-content and form events | fixtures, plus `seeds/content/` (content streams) |

Each target is one line calling a function in [`crates/lamina-model/tests/common/fuzz_entry.rs`](../crates/lamina-model/tests/common/fuzz_entry.rs) (`content`: [`crates/lamina-content/tests/common/fuzz_entry.rs`](../crates/lamina-content/tests/common/fuzz_entry.rs); the save targets: [`crates/lamina-save/tests/common/fuzz_entry.rs`](../crates/lamina-save/tests/common/fuzz_entry.rs)), which `fuzz/src/lib.rs` includes (with `exercise.rs`), so the integration tests and the fuzzer share the code. Invariants: no panic, no hang (`-timeout=10`), memory under `-rss_limit_mb=2048`, default [`Limits`](../docs/decisions/0003-reader-tolerance-and-limits.md).

## Running

```sh
rustup toolchain install nightly --profile minimal
cargo install cargo-fuzz --locked --version 0.13.2    # or: cargo xtask doctor --install
cargo xtask fuzz <target> --time 600                  # default 60 s
cargo xtask fuzz --all --time 120                     # every target in turn (the full PR lane)
cargo xtask fuzz seeds [<target>]                     # only build fuzz/corpus/<target>/
cargo xtask fuzz cmin [<target>]                      # minimise fuzz/corpus/<target>/ (cargo fuzz cmin)
cargo xtask fuzz tmin <target>                        # minimise every crash in fuzz/artifacts/<target>/
```

`cargo xtask fuzz` first builds the seed corpus `fuzz/corpus/<target>/` (gitignored) from the PDFs in `fixtures/corpus/` and `fixtures/golden/`, plus `fixtures/regressions/<target>/` and everything under `fuzz/seeds/<target>/` (except `passwords.txt`) copied verbatim; seed names carry their source path, then runs `cargo +nightly fuzz run <target> fuzz/corpus/<target> -- -max_total_time=<N> -timeout=10 -rss_limit_mb=2048`. The nightly lane ([`.github/workflows/nightly.yml`](../.github/workflows/nightly.yml), todo X3a) runs every target for 10 minutes each night (and on demand). It starts from the corpus the full lane last saved on `main` (or its own last one), minimises it with `cargo xtask fuzz cmin` before saving it back, minimises crashes with `cargo xtask fuzz tmin <target>` under the same libFuzzer limits, uploads them as artefacts and opens a `nightly` issue with the minimised inputs; the full PR lane ([`.github/workflows/full.yml`](../.github/workflows/full.yml)) runs each target for 2 minutes on every code PR. It starts from the corpus last saved by a push to `main`; only those pushes save it, after `cargo xtask fuzz cmin`, so the cache stays bounded. `--all` runs every target before failing and says whether a failure was a crash (new files in `fuzz/artifacts/<target>/`) or a build or setup problem.

Encrypted seeds are maintained with decryption ([B2b](../docs/todos/done/B2b-standard-security-decryption.md)): the files go in `seeds/open/encrypted/` and their passwords, one per line, in `seeds/open/passwords.txt` (lines starting with `#` are ignored).

## When a target finds a crash

1. Reproduce: `cargo +nightly fuzz run <target> fuzz/artifacts/<target>/<crash-file>`.
2. Minimise: `cargo +nightly fuzz tmin <target> fuzz/artifacts/<target>/<crash-file>` (or `cargo xtask fuzz tmin <target>` for every crash; the nightly issue already carries the minimised input, base64-encoded when it is at most 4 KiB).
3. Commit the minimised input, raw bytes as the target received them, as `fixtures/regressions/<target>/<hash>` (the hash from the artefact name) and list it in `fixtures/README.md`. `crates/lamina-model/tests/regressions.rs` (for `content`, `crates/lamina-content/tests/regressions.rs`; for `ops_json` and `save_roundtrip`, `crates/lamina-save/tests/regressions.rs`) replays every file there through the same `fuzz_entry` function, so it fails before the fix; also add a named test for the behaviour in the crate whose code was wrong. The seed step picks the file up from then on.
4. Fix the bug in the same PR.

## Licence

`libfuzzer-sys` is `(MIT OR Apache-2.0) AND NCSA` (it bundles LLVM's libFuzzer). It is a fuzzing harness, never a dependency of a shipped crate, and lives outside the root workspace, so `deny.toml`'s allow-list does not apply to it.
