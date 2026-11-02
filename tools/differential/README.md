# differential

The external readers of the nightly lane's live differential ([ADR 0018](../../docs/decisions/0018-test-strategy.md) tier 6; todo X3a), run by `cargo xtask differential --live`. Each reads a `<name>\t<path>` list and writes the same JSON: per file the open/repair/fail outcome, the page count and the outline titles, plus the tool's version and whether it reports repairs. lamina's side is `crates/laminapdf-model/examples/differential-facts.rs`; xtask (`tools/xtask/src/live.rs`) compares them and writes `target/nightly/differential/report.md` and `report.json`.

| Tool | Runner | Version pinned in | Reports repairs |
| --- | --- | --- | --- |
| pdf.js | `pdfjs.mjs` (`pdfjs-dist`, no eval, no fonts) | `package.json` + `pnpm-lock.yaml` | no |
| PDFium | `facts.py pdfium` (pypdfium2's bundled PDFium) | `requirements.txt` | no |
| qpdf | `facts.py qpdf` (`qpdf --json=2`; exit 3 with a damage warning = repaired) | `tools/xtask/src/doctor.rs`, the nightly workflow's download (SHA-256) | yes |
| pypdf | `facts.py pypdf` (the PR lane's `tools/corpus-facts/facts.py`) | `requirements.txt` (same as `tools/corpus-facts`) | yes (from its warnings) |

`text.mjs` has the text readers (todos B9c, B9f): pdf.js (`getTextContent`, items joined, a newline after `hasEOL`) and pdftotext (`-enc UTF-8`, split on form feeds) text per page. Given files, it writes `fixtures/differential/text.json` for the PR lane (`cargo xtask differential --text --update`, compared by `crates/laminapdf/tests/text_differential.rs`). With `--facts pdfjs|pdftotext <list.tsv> <out.json>` it is a live reader like the others (`{outcome, pages, page_errors?}` per file: a pdf.js page that throws is `null`; neither tool reports repairs; 60 s per file), run by `cargo xtask differential --live --text` against lamina's `crates/laminapdf/examples/differential/text-facts.rs` over the external corpus ([B9f](../../docs/todos/done/B9f-live-text-differential.md)). xtask compares the pages with whitespace and bidi controls removed; a page agrees when lamina matches at least one tool, and the report (`target/nightly/differential-text/`) has the agreement per corpus source. Text exceptions are `[[class]]` and `[[known]]` entries with `facts = ["text"]` in `fixtures/differential/exceptions.toml`; a text `[[known]]` names the pages it excuses per file (`[known.pages]`), and the PR lane's per-file `[[exception]]`s don't take text. pdftotext takes part only at the pinned poppler: with another version (or none) the run warns and compares with pdf.js alone.

| Text tool | Runner | Version pinned in |
| --- | --- | --- |
| pdf.js | `text.mjs --facts pdfjs` | `package.json` + `pnpm-lock.yaml` |
| pdftotext | `text.mjs --facts pdftotext` (poppler) | `tools/xtask/src/doctor.rs` (`POPPLER_VERSION`; the runner's Homebrew poppler, installed by the nightly when the runner has none, left out with a warning when its version differs) |

xtask runs lamina and the tools at once, each over chunks of 200 inputs, one process per chunk with a 20-minute limit (`tools/xtask/src/runner.rs`). A chunk that crashes, hangs or reports different files is rerun one file per process with a 2-minute limit, so a bad file becomes a `timeout` or `crash` entry (skipped for a tool, a failure for lamina) instead of stopping the run; a reader whose every file in a chunk crashes is reported broken and fails the run after the report is written. Inside a process, the runners also give up on a file after 60 s where they can (Python's `SIGALRM`, a timer in pdf.js), which native code and synchronous parsing can ignore. Each reader's output goes to `target/nightly/differential/<reader>.log`. Inputs are data: they are only read, never executed.

```sh
cargo xtask corpus fetch                                  # the external corpus (fixtures/corpus-manifest.toml)
pnpm --dir tools/differential install --frozen-lockfile   # pdf.js: its own pnpm project, outside the root workspace
python3 -m venv .venv && .venv/bin/pip install -r tools/differential/requirements.txt
LAMINA_PDF_PYTHON=.venv/bin/python cargo xtask differential --live [--sample 300] [--tools pdfjs,qpdf] [--corpus external|mutants|all] [--source pdfjs]
cargo xtask differential --live --text [--sample 50] [--source pdfjs]   # page text: pdf.js and pdftotext (no Python)
```

qpdf must be on `PATH` at the version `cargo xtask doctor` expects. All five are tools only: nothing here ships or links into lamina (pdfjs-dist and pypdfium2 are Apache-2.0, PDFium BSD-3-Clause, qpdf Apache-2.0, pypdf BSD-3-Clause, poppler's pdftotext GPL-2.0, run as a separate program and never linked).
