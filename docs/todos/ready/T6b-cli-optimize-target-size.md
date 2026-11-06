# T6b — `laminapdf optimize` with a target size

| Field | Value |
| --- | --- |
| ID | `T6b` |
| Status | `ready` |
| Plan | [trust and proof §T6](../../plans/trust-and-proof.md#t6-chained-jobs-in-the-cli) |
| Round / milestone | Round T (trust and proof) |
| Depends on | [E16c](./E16c-images-and-fonts.md), [T6a](./T6a-cli-page-commands.md) |
| ADRs | Follows and amends [0031](../../decisions/0031-cli-and-mcp-contract.md) (a new command, `laminapdf.optimize/1` in §6's list of schemas, a new `cliErrors` code under §8); follows [0007](../../decisions/0007-save-planner.md) (optimising is a full rewrite: signed and encrypted files are refused), [0026](../../decisions/0026-error-and-diagnostic-contract.md) (the new code), [0028](../../decisions/0028-facade-rules-jobs-and-cancellation.md) (the façade only; `CancelToken` between tries) |
| Branch | `feat/t6b-cli-optimize-target-size` |
| PR title | `feat(cli): optimize with a target size, lossless steps first` |
| Size | M (≈ 350 lines of code plus tests) |
| PR | — |

## Goal

People need a PDF under a portal's or an e-mail's size cap, and the tools they try let them down. On Hacker News, 62 comments ask for it, a share that doubled from 2016–19 to 2023–26: "Is there a .pdf tool which allows compression to a defined file size?" ([26325269](https://news.ycombinator.com/item?id=26325269)); "every “shrink” tool we tried barely compressed the size, and some even made it larger!" ([43884344](https://news.ycombinator.com/item?id=43884344)). Compression that turns every page into a picture is a named failure ([43895227](https://news.ycombinator.com/item?id=43895227)). This PR adds `laminapdf optimize` over E16's optimiser, with `--target-size`. It tries the lossless steps first, then re-encodes images in fixed steps, and never turns text into pictures or writes a larger file. It reports what it did, and when the target can't be met it fails with its own code and the smallest size it reached.

## Context

- **Evidence:** the [editors note](../../research/hacker-news/notes/editors.md) (want 11, "Shrink files to fit an upload limit"; §6, recommendations 1 and 6: compress by re-encoding images and fonts, never by turning text into pictures), the report's [R10](../../research/hacker-news/pdf-users-on-hacker-news.md#r10-grow-the-cli-and-mcp-server-for-chained-jobs) and its fourth [thing the evidence argues against](../../research/hacker-news/pdf-users-on-hacker-news.md#four-things-the-evidence-argues-against) (no flattening pages to images to make files small).
- **The optimiser**, done once E16c is, since E16c depends on E16b: [E16b](./E16b-structural-optimiser.md) (the `Optimise { options }` op, always a full rewrite: unused objects, deduplication, Flate recompression, object streams, linearisation, a size report) and [E16c](./E16c-images-and-fonts.md) (images downsampled to a target resolution and recompressed at a JPEG quality, masks kept, JPX skipped unless decodable; embedded fonts subset to the glyphs used). [E16a](./E16a-optimisation-print-feature-plan.md)'s feature plan fixes the options' names; this todo calls them by what they do. Background: [05 §3.4](../../research/pdf/05-creating-pdfs-from-code.md#34-optimisation-and-compression).
- **Design (Paper):** [X09 Optimise](../../design/paper/convert-compare-produce.md#x09-optimise), the editor's dialog over the same op: sizes "from a trial run", and "This file is already compact" when there is nothing to gain. The CLI follows its wording.
- **[T6a](./T6a-cli-page-commands.md)** adds what this command reuses: write commands over many inputs on L4b's runner, `--output-dir`, the shared output resolution and save-and-write step in `crates/laminapdf-cli/src/commands/pages.rs`, and the write tools' MCP pattern. `pages rotate` there is the model for one input.
- **[ADR 0031](../../decisions/0031-cli-and-mcp-contract.md):** grammar, `-` and `-o -`, `--force`, `--dry-run`, exit codes (a failure that isn't a usage error exits 1), JSON reports with committed schemas (`crates/laminapdf-cli/schemas/`, `src/schema.rs`), `cliErrors` in `fixtures/codes.json` with a `docs/errors/<code>.md` page each.
- **[ADR 0007](../../decisions/0007-save-planner.md):** a full rewrite of a signed file is a `Conflict`, and of an encrypted base it is refused (`encrypted-base`) unless encryption is removed.

## Scope

**In scope**

- **The command:** `laminapdf optimize <INPUT>… [--files-from FILE [-0]] [--target-size SIZE [--lossless]] [--image-ppi N] [--jpeg-quality Q] [--linearize] (-o OUT | --output-dir DIR | --in-place)`, in `crates/laminapdf-cli/src/commands/optimize.rs`. It is one function `optimize(args, &Settings, &mut dyn Io) -> Result<OptimizeReport, CliError>`, and runs over many inputs on T6a's write batch.
  - Without `--target-size`, it runs the lossless steps, plus the image step `--image-ppi` and `--jpeg-quality` set by hand; those two flags exclude `--target-size`.
  - `SIZE` is a number of bytes (`2500000`), or a number with a decimal unit (`KB`, `MB`, `GB`: 1 MB = 1,000,000 bytes) or a binary one (`KiB`, `MiB`, `GiB`: 1 MiB = 1,048,576 bytes), in any letter case, with a decimal point allowed (`1.5MB`). "MB" reads as decimal because that is the smaller of its two meanings, so a file that meets "10 MB" fits a portal that means either. An unparsable size, or zero, is `invalid-command-line`.
- **Step 1, lossless, always first:** E16b's unused-object removal, deduplication, Flate recompression and object streams with an xref stream, and E16c's subsetting of embedded fonts to the glyphs used. Linearisation only with `--linearize`, because it adds bytes.
- **Step 2, the image ladder:** only with `--target-size`, and only when step 1 missed the target. The ladder is E16c's image pipeline at fixed (resolution, JPEG quality) steps, from the least loss to the most:

  | Step | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
  | --- | --- | --- | --- | --- | --- | --- | --- |
  | Resolution (ppi) | 300 | 200 | 150 | 150 | 110 | 96 | 72 |
  | JPEG quality | 85 | 80 | 75 | 60 | 55 | 50 | 40 |

  - at each step, images drawn above that resolution on the page (E16c measures it at their largest placement) are downsampled to it, and continuous-tone images are re-encoded as JPEG at that quality;
  - these are left alone at every step: 1-bit images (CCITT, JBIG2, 1 bit per component), image masks and soft masks (kept lossless, as E16c keeps masks), JPX images E16c can't decode, and any image whose new encoding isn't smaller;
  - beyond step 1, text, fonts, vector paths and annotations never change, and no page is ever rendered to an image.
- **The search:** each try writes the whole file to memory, so every size is exact, not an estimate. The command bisects the ladder, assuming sizes fall as the steps get stronger, and keeps the least lossy step that met the target: at most four full writes, counting the lossless one. `Settings::cancel` is checked between tries (`cancelled`).
- **The floor:** when E16b's and E16c's size report gives the bytes in image streams, the floor is the lossless size minus the image bytes the ladder could shrink. A target below the floor fails at once, without trying the ladder, because no image setting can reach it.
- **Never larger:** unless `--linearize` was asked for, a result that isn't smaller than the input is replaced by the input's own bytes, and the report says `"unchanged": true` ("already compact", X09's wording). An input already under `--target-size` is also kept as it is.
- **The failure:** a new code, `target-size-unreachable` (`cliErrors`, kind `limit`, exit 1). Nothing is written. Its data:
  - `target`: the target in bytes;
  - `smallest` and `step`: the smallest size reached and the step that reached it;
  - `floor`: the floor, when known;
  - `lossless`: whether `--lossless` stopped the ladder.
  Hint: "allow a larger --target-size, or drop --lossless so images can be re-encoded; text and fonts are never turned into images". Its page is `docs/errors/target-size-unreachable.md`, written like `docs/errors/overwrite-refused.md`.
- **Refused before any try:** signed files (`would-invalidate-signatures`, since a full rewrite breaks every signature) and encrypted ones (`encrypted-base`, until E11 can encrypt on write); both exit 1.
- **Conformance claims:** a PDF/A or PDF/UA claim the output can't keep, by E16b's or E16c's report, is a warning in the report (exit 3), never dropped in silence.
- **The report, `laminapdf.optimize/1`** (new; `crates/laminapdf-cli/schemas/optimize.schema.json` from `schema::all()`; `schema::COMMANDS` gains `optimize`):
  - `schema`, `input` and `output`;
  - `before` (the input's bytes), `after` (the chosen result's bytes, measured under `--dry-run` too) and `written` (0 under `--dry-run`);
  - `target` and `met`, with `--target-size`; `unchanged`;
  - `lossless`: the lossless steps applied, with their counts from the size report (objects removed, duplicates merged, streams recompressed, fonts subset, `linearized`);
  - `images`: `null`, or `{ ppi, jpegQuality, reencoded, downsampled, kept }`;
  - `tried`: `[{ ppi, jpegQuality, bytes }]` in the order run (the lossless try has no `ppi` or `jpegQuality`);
  - `warnings`.
- **Text output:** "2.4 MB → 1.1 MB (lossless) → out.pdf", "14.8 MB → 9.6 MB (images at 150 ppi, JPEG 75) → upload.pdf", "already compact: 412 KB, out.pdf unchanged".
- **MCP:** with `--allow-write`, `laminapdf_optimize` joins the write tools, in name order, with `destructiveHint: true` only when the server also has `--allow-in-place`, and every path through `RootedIo`.

**Out of scope** (and which todo covers it)

- Colour conversion, transparency flattening, PDF/A conversion: [E16d](./E16d-colour.md), [E16e](./E16e-preflight-and-conversion.md).
- Removing metadata, attachments, JavaScript or hidden content: the sanitiser, [E11d](./E11d-sanitiser.md).
- Keeping an input's encryption: [E11b](./E11b-encrypt-on-write.md). Batches on a server: [E16g](./E16g-server-batch-optimise.md).
- A target size in the editor's X09 dialog: E16b and E16c build that dialog; see the open question on where the search lives.
- An override that optimises signed files: not planned; the signatures would no longer validate.

## Tasks

- [ ] Claim via the work PR (see Definition of done); a fresh worktree off `main`. Load `/rust-best-practices` and follow [docs/guidelines/rust.md](../../guidelines/rust.md).
- [ ] Read what E16a–E16c merged: the `Optimise` options' real names, the size report's fields, how images are classified. If the façade lacks the op or its report, add them to `laminapdf` first (ADR 0031 §1: commands use only the façade).
- [ ] The size parser with unit tests (`10MB` is 10,000,000; `10MiB` is 10,485,760; `1.5mb`; `0`, `-1`, `10XB` and an empty value refused).
- [ ] The ladder as data in `commands/optimize.rs`, the bisection over a size function (unit-tested with fake size functions: falling, flat, rising at one step, all over the target), the floor rule and the never-larger rule.
- [ ] `optimize` and `OptimizeReport`; the clap wiring in `main.rs`; the batch through T6a's write batch.
- [ ] `target-size-unreachable` in `fixtures/codes.json` (`cliErrors`), its page, `cargo xtask gen codes`, and its exit code in `exit::exit_code_for`'s table test.
- [ ] The schema (`UPDATE_GOLDEN=1 cargo test -p laminapdf-cli --test json_schemas`), `laminapdf schema optimize`, the MCP tool and its `tests/mcp.rs` cases.
- [ ] ADR 0031's amendment, the CLI README and CLAUDE.md (below).

### Example session

```console
$ laminapdf pages merge scans/*.pdf -o - | laminapdf optimize - --target-size 10MB -o upload.pdf
38.2 MB → 9.6 MB (images at 150 ppi, JPEG 75) → upload.pdf
$ laminapdf optimize report.pdf -o report-small.pdf
2.4 MB → 1.1 MB (lossless) → report-small.pdf
$ laminapdf optimize text-only.pdf --target-size 50KB -o out.pdf
error[target-size-unreachable]: the smallest result is 212 KB, above the 50 KB target
  = hint: allow a larger --target-size, or drop --lossless so images can be re-encoded; text and fonts are never turned into images
$ echo $?
1
```

## Acceptance criteria

- [ ] **The fixture** (`crates/laminapdf-cli/tests/optimize.rs`), built in the test with `laminapdf::Builder` as `tests/mcp.rs` builds its fixtures: two pages of text in an embedded font from `fixtures/fonts/`, and one page with a 2,400 × 2,400 px RGB image from a seeded `laminapdf_test_fixtures::Rng` pattern (smooth noise, which Flate barely shrinks and JPEG does), drawn 4 in wide (600 ppi).
- [ ] **Lossless:** `optimize in.pdf -o out.pdf` makes the file smaller; every page's text (`Document::page_texts`) is unchanged; and every page renders to the same RGBA bytes as the input with `laminapdf-render`'s hayro backend at 72 dpi.
- [ ] **A target only images can meet:** the test measures steps 2 and 3 by hand (`--image-ppi 200 --jpeg-quality 80`, then `150` and `75`) and sets `--target-size` between the two sizes. The output is at most the target, `met` is true, and `images` names step 3, the least lossy step that fits. Both text pages still render to the same bytes as the input, and every page's text is unchanged. The image page's render differs from the input's by a mean of at most 12 of 255 per channel.
- [ ] **Unreachable:** a target below the text pages' own size exits 1 with `target-size-unreachable` and `data.smallest` set, and writes nothing. When the size report gives image bytes, `data.floor` is set and only the lossless try ran; otherwise the ladder's last step ran too. With `--lossless`, a target only the image ladder could meet fails the same way with `data.lossless` true.
- [ ] **Never larger, never pictures:** over every file in `fixtures/corpus/` and `fixtures/golden/examples/`, the lossless run's `after` is at most `before`, and each output reopens in Lamina PDF with the same page count and the same text on every page. `fixtures/golden/hello.pdf` comes out byte for byte unchanged, with `unchanged: true`.
- [ ] **Refusals:** `fixtures/signed/approval.pdf` exits 1 with `would-invalidate-signatures`, an encrypted fixture from `fixtures/encrypted/` exits 1 with `encrypted-base`, and neither writes a file.
- [ ] **The contract:** the same input and options give the same bytes twice; `--dry-run` measures and writes nothing; `--force`, `-` and `-o -` behave as in `pages rotate`; reports validate against the committed `optimize.schema.json`. Three inputs with `--output-dir` print NDJSON `result` lines, then the summary.
- [ ] **The chain:** in `tests/pipes.rs`, `laminapdf pages merge a.pdf b.pdf -o - | laminapdf optimize - --target-size … -o out.pdf` writes a file under the target.
- [ ] **MCP:** `laminapdf_optimize` is listed only with `--allow-write`, and an output outside the roots is a tool error with `path-outside-roots`.
- [ ] `cargo xtask check` passes, including the codes check (`crates/laminapdf-cos/tests/codes.rs` finds the new page).

## Post-merge checks

None.

## Tests

- `crates/laminapdf-cli/tests/optimize.rs` (the fixture, the ladder, the floor, refusals, the corpus sweep), `tests/pipes.rs`, `tests/batch.rs`, `tests/json_schemas.rs`, `tests/mcp.rs`, `tests/exit_codes.rs` (the new code's exit).
- Unit tests in `commands/optimize.rs`: the size parser, the bisection, the floor and never-larger rules.
- `laminapdf-render` with the `hayro` feature as a dev-dependency for the render comparisons.

## Docs to update

- ADR 0031: an amendment section (`optimize`, its flags and units, `laminapdf.optimize/1` in §6's list).
- `fixtures/codes.json` and `docs/errors/target-size-unreachable.md`; the generated tables (`cargo xtask gen codes`).
- `crates/laminapdf-cli/README.md` (the command, the ladder table, "use `--lossless` for print") and CLAUDE.md (the `laminapdf-cli` layout line and the CLI paragraph).
- [Trust and proof §10](../../plans/trust-and-proof.md#10-to-dos): tick T6b.

## Risks and open questions

- **Sizes that don't fall at every step.** JPEG at a lower quality and a lower resolution are smaller in nearly every case, but not always. The bisection keeps only a step it measured under the target; the cost of an odd case is a stronger step than needed, never a file over the target.
- **Time on large scans.** Each try re-encodes every image, so four writes of a 200 MB scan take a while. The command shows one progress line per try on stderr, and the MCP server's `--timeout` cancels it. If tries are slow, E16c can keep decoded images between tries (a follow-up).
- **Scanned text stored as images.** A scanned page is an image, so the ladder re-encodes it, and at the last steps small print gets blurry. The report names the step; the README says to pass `--lossless` for print.
- **Open question: where the search lives.** The CLI owns it here. If X09 later offers "fit under N MB", the ladder and the bisection move into the façade, so the CLI and the editor share one search.
- **Open question for the owner: units.** "MB" as decimal is the safe reading. The other choice is binary, as some e-mail services count; either way the README states it.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
