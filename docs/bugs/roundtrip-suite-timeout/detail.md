# The round-trip suite runs past the test time limit on Windows Arm64

[Resolution](resolution.md).

`laminapdf-save`'s round-trip suite ran every fixture × op × mode × overrides case in one test, `every_case_saves_or_refuses_as_expected_and_reopens_in_lamina`. On CI's `Rust (windows-arm64)` leg that test ran past nextest's 120 s limit and was killed, on `main` and on pull requests alike ([issue #695](https://github.com/laudebugs/laminapdf/issues/695)).

## What we saw

`.config/nextest.toml` reports a test as slow at 60 s and kills it at 120 s, which is how the PR lanes catch a hang ([ADR 0018](../../decisions/0018-test-strategy.md)). On `main`, run 43977517486 (d7dd3bf4b):

```
SLOW        [> 60.000s] laminapdf-save::roundtrip_suite every_case_saves_or_refuses_as_expected_and_reopens_in_lamina
TIMEOUT     [ 120.035s] laminapdf-save::roundtrip_suite every_case_saves_or_refuses_as_expected_and_reopens_in_lamina
```

Pull requests #689 and #691 failed the same way. Every other leg passed. Each new op adds cases to the suite, and the flatten cases (#692) had just merged.

## How to reproduce

Windows Arm64 runs the `sha2` and `aes` crates' portable code (see below). Forcing the same backends on another machine, in a separate target folder, shows the slowdown:

```sh
CARGO_TARGET_DIR=target/soft RUSTFLAGS='--cfg sha2_backend="soft" --cfg aes_backend="soft"' \
  cargo nextest run -p laminapdf-save --test roundtrip_suite --test-threads 1
```

## Measurements

The same run, the suite's test next to a small test of the same binary (`text_edits_round_trip_and_read_back_as_typed`) on each leg:

| Leg | Small test | Suite | Ratio |
| --- | ---: | ---: | ---: |
| macOS Arm64 | 0.064 s | 23.6 s | 370× |
| Linux Arm64 | 0.034 s | 22.6 s | 670× |
| Linux x64 | 0.044 s | 37.6 s | 850× |
| Windows x64 | 0.049 s | 43.4 s | 890× |
| macOS x64 | 0.110 s | 56.7 s | 520× |
| Windows Arm64 | 0.056 s | > 120 s (killed) | > 2,100× |

Windows Arm64 isn't a slow machine in general, since its small test took about what the others did: only the suite was slow there.

Locally (Apple M1 Pro, debug build, `cargo nextest run -p laminapdf-save --test roundtrip_suite`), the suite's test took 14.0 s. Timing each op over every fixture, and each fixture over every op, put half of that on one file:

- **By fixture:** `corpus/enc.pdf` took 6.8 s of the 14.0 s. The next slowest fixture took 1.2 s.
- **By op:** every op took between 0.4 s and 0.9 s, and the cases beyond the op grid (copies, images, extracted documents, outlines, links, marks, flattening) took 0.35 s together.

`corpus/enc.pdf` is encrypted with AES-256, revision 6. Each case opens it, and so does each reopen of a saved output that kept the encryption. Opening it derives the file key with ISO 32000-2's Algorithm 2.B (`hash_r6` in `laminapdf-syntax`'s `crypt/key.rs`). That takes at least 64 rounds, each one encrypting 64 copies of the password and the last hash with AES-128 and hashing the result with SHA-256, SHA-384 or SHA-512.

The `sha2` and `aes` crates pick a hardware backend at run time through `cpufeatures`. On Arm64, `cpufeatures` 0.3.1 detects features only on Linux, Android and Apple platforms, and answers "no" everywhere else (`src/aarch64.rs`). On Windows Arm64 both crates therefore run their portable code, and in a debug build that code is unoptimised.

Forcing the portable backends locally, as above, reproduces it. With each test run alone, the op grid, split by family, took 157 s with the portable backends against 18 s with the hardware ones, nine times longer.

## The cause

Two things added up:

1. **One test held the whole suite.** Its time is the sum of more than a thousand cases, so it grows with every op, every fixture and every kind of case. A limit meant for one hanging test applied to all of them at once, and a hang in any single case couldn't be told from the suite simply growing.
2. **The encrypted fixture's key derivation ran in unoptimised portable code on Windows Arm64.** That made the suite nine times slower there, so that leg crossed the limit first.
