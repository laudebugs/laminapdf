# differential

The external readers of the nightly lane's live differential ([ADR 0018](../../docs/decisions/0018-test-strategy.md) tier 6; todo X3a), run by `cargo xtask differential --live`. Each reads a `<name>\t<path>` list and writes the same JSON: per file the open/repair/fail outcome, the page count and the outline titles, plus the tool's version and whether it reports repairs. lamina's side is `crates/lamina-model/examples/differential-facts.rs`; xtask (`tools/xtask/src/live.rs`) compares them and writes `target/nightly/differential/report.md` and `report.json`.

| Tool | Runner | Version pinned in | Reports repairs |
| --- | --- | --- | --- |
| pdf.js | `pdfjs.mjs` (`pdfjs-dist`, no eval, no fonts) | `package.json` + `pnpm-lock.yaml` | no |
| PDFium | `facts.py pdfium` (pypdfium2's bundled PDFium) | `requirements.txt` | no |
| qpdf | `facts.py qpdf` (`qpdf --json=2`; exit 3 with a damage warning = repaired) | `tools/xtask/src/doctor.rs`, the nightly workflow's download (SHA-256) | yes |
| pypdf | `facts.py pypdf` (the PR lane's `tools/corpus-facts/facts.py`) | `requirements.txt` (same as `tools/corpus-facts`) | yes (from its warnings) |

xtask runs lamina and the tools at once, each over chunks of 200 inputs, one process per chunk with a 20-minute limit (`tools/xtask/src/runner.rs`). A chunk that crashes, hangs or reports different files is rerun one file per process with a 2-minute limit, so a bad file becomes a `timeout` or `crash` entry (skipped for a tool, a failure for lamina) instead of stopping the run; a reader whose every file in a chunk crashes is reported broken and fails the run after the report is written. Inside a process, the runners also give up on a file after 60 s where they can (Python's `SIGALRM`, a timer in pdf.js), which native code and synchronous parsing can ignore. Each reader's output goes to `target/nightly/differential/<reader>.log`. Inputs are data: they are only read, never executed.

```sh
cargo xtask corpus fetch                                  # the external corpus (fixtures/corpus-manifest.toml)
pnpm --dir tools/differential install --frozen-lockfile   # pdf.js: its own pnpm project, outside the root workspace
python3 -m venv .venv && .venv/bin/pip install -r tools/differential/requirements.txt
LAMINA_PYTHON=.venv/bin/python cargo xtask differential --live [--sample 300] [--tools pdfjs,qpdf] [--corpus external|mutants|all] [--source pdfjs]
```

qpdf must be on `PATH` at the version `cargo xtask doctor` expects. All four are tools only: nothing here ships or links into lamina (pdfjs-dist and pypdfium2 are Apache-2.0, PDFium BSD-3-Clause, qpdf Apache-2.0, pypdf BSD-3-Clause).
