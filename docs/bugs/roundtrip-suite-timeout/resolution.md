# Resolution: the round-trip suite runs past the test time limit on Windows Arm64

[Diagnosis](detail.md).

## The fix

**One test per family of cases.** The op grid's ops are now grouped in `OpFamily` (`crates/laminapdf-save/tests/common/roundtrip.rs`): page order, rotation and labels, imports, metadata, XMP, the initial view, page boxes, page size and content edits. `op_cases(root, family, suite)` runs every fixture × that family's ops × mode × overrides.

The other cases are grouped in `OtherCases`: page copies, images made into PDFs, extracted and split documents, outlines, links, marks and flattened marks. `roundtrip_suite.rs` has one test per family and one per kind of other case. Exhaustive matches map each family and kind to its test, so a new one doesn't build without a test. `run`, which the `roundtrip` example runs for `cargo xtask roundtrip`, still runs every family and every other case. The full lane checks the same outputs with qpdf, pdf.js and PDFium.

Each test still checks that every case saved or was refused as expected and reopened with the expected facts. Each family's test also checks two counts:

- **Saved, per op.** A plain fixture saves in all 12 mode × override cases, so each op must save at least 12 cases per plain fixture it applies to. That is all 17 for most ops, the 5 with more than one page for deleting, moving and reversing pages, and the 9 with a rewritable run of text for `editText`. An op that silently skips fixtures fails.
- **Refused.** The signed and encrypted fixtures refuse some modes in every family, so some case must be refused.

The op list lives in the families alone, so a new op has to join one, and with it a test.

Each capability tag moved to the test whose cases prove it; G.05 (an encrypted file saved with its encryption removed) went to the metadata test. `cargo xtask capabilities` regenerated `docs/capabilities/README.md`.

**The crypto crates are optimised in debug builds.** `Cargo.toml` builds `sha2` and `aes` at `opt-level = 3` in the `dev` profile, as it already did for `miniz_oxide` and `brotli`. Where the hardware backends are found, nothing changes. Where they aren't, the encrypted cases run their portable code optimised.

No time limit was raised: the suite's tests keep the default 60 s and 120 s, so a hang in one family is still caught and named.

## How it was checked

Each test's time, run one at a time on an Apple M1 Pro in a debug build:

| Test | Hardware crypto | Portable crypto | Portable, optimised |
| --- | ---: | ---: | ---: |
| page order | 2.3 s | 20.7 s | 16.8 s |
| rotation and labels | 1.9 s | 41.4 s | 8.6 s |
| imports | 2.0 s | 13.0 s | 10.1 s |
| metadata | 3.3 s | 15.8 s | 11.5 s |
| XMP | 1.5 s | 29.5 s | 4.7 s |
| initial view | 0.6 s | 4.6 s | 3.0 s |
| page boxes | 1.1 s | 10.4 s | 8.8 s |
| page size | 1.6 s | 10.4 s | 7.6 s |
| content | 2.8 s | 10.8 s | 7.2 s |
| every other kind | under 0.2 s each | not measured | not measured |
| **op grid, together** | **17.8 s** | **157 s** | **78 s** |

The old single test took 14.0 s here and over 120 s on Windows Arm64. With portable crypto, the slowest family now takes 17 s. Windows Arm64 runs the small tests about as fast as this machine, so each family there should stay well under nextest's 60 s.

Also run: `cargo clippy -p laminapdf-save --all-targets -- -D warnings`, `cargo nextest run -p laminapdf-save`, `cargo xtask check docs`, `cargo xtask check maintainability` and `cargo xtask capabilities`. The CI legs give the measurement on Windows Arm64 itself.

## Left for later

- **Generic crypto code is still unoptimised.** The `Digest` wrappers are generic, so they compile in `laminapdf-syntax`, unoptimised in a debug build; so does the CBC chaining, that crate's own code in `crypt/aes.rs`. That is why the portable crypto run stops at 78 s rather than near 18 s.
- **The encrypted fixture is opened once per case.** Opening it once per fixture and reusing the parsed document for each case would save most of the remaining key derivations. That needs a cheap copy of an open document, which the suite doesn't have.
- **`cpufeatures` on Windows Arm64.** Detecting the Arm64 crypto extensions on Windows (through `IsProcessorFeaturePresent`) belongs upstream. Release builds on Windows Arm64 also run the portable code, optimised.
