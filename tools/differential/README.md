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
| pdftotext | `text.mjs --facts pdftotext` (poppler) | `tools/xtask/src/doctor.rs` (`POPPLER_VERSION`; the runner's Homebrew poppler, installed by the nightly when the runner has none, left out with a warning when its version differs, which fails the pages only pdftotext agrees with) |

The XMP differential (todo E1c) compares the XMP values lamina's `crates/laminapdf-model/examples/xmp-facts.rs` reads (`laminapdf_model::xmp::Packet`'s getters: title, authors, subject, keywords, creator tool, producer, the three dates, document and instance IDs, the PDF/A and PDF/UA claims) with ExifTool's, over the external corpus and the PDFs in `fixtures/corpus/`, `fixtures/golden/` and `fixtures/golden/examples/`. xtask (`tools/xtask/src/live/xmp.rs`) runs ExifTool itself, once over every file (`exiftool -j -G1 -n -XMP:all`), and compares field by field: text trimmed, `dc:creator` as a list, dates as one spelling of the same instant (ExifTool prints `2026:10:28 14:30:00+01:00`). A field where they differ fails the run unless a `[[known]]` entry in `fixtures/differential/xmp-exceptions.toml` (`file` glob, `fields`, `issue`, `reason`) excuses it; the report goes to `target/nightly/differential-xmp/`.

| XMP tool | Runner | Version pinned in |
| --- | --- | --- |
| ExifTool | `exiftool`, run by xtask | `tools/xtask/src/doctor.rs` (`EXIFTOOL_VERSION`; the nightly installs Ubuntu 24.04's `libimage-exiftool-perl` at `EXIFTOOL_APT_VERSION`) |

The inspection differential (todo E3g) compares the objects, the newest cross-reference section and the fonts lamina's inspector reads (`crates/laminapdf/examples/differential/inspect-facts.rs`, over `Document::objects`, `Document::trailer` and `Document::fonts`) with qpdf's and pdffonts', over the external corpus and the PDFs in `fixtures/corpus/`, `golden/`, `golden/examples/`, `signed/`, `attachments/` and `revisions/`. xtask (`tools/xtask/src/live/inspect.rs`) runs the tools itself, one file at a time on every core with a 60 s limit: `qpdf --json=2 --json-key=qpdf` for each object's number, generation, kind (stream, dictionary or other), `/Type` and `/Subtype` (else `/S`), and for the newest section's kind (table, cross-reference stream, or hybrid with `/XRefStm`), `/Size` and `/Prev`, read from the trailer since qpdf lists no sections or revisions; `qpdf --show-xref` for where each object is stored (an offset, or an object stream and index); and `pdffonts` for each font's object, name (subset tag apart), type (mapped to its `/Subtype` family), encoding (lamina's `WinAnsiEncoding` written `WinAnsi`, a dictionary `Custom`), embedded, subset and ToUnicode. A file either side repaired has its objects and section left out, since they are then each reader's own. A differing fact fails the run unless a `[[known]]` entry in `fixtures/differential/inspect-exceptions.toml` (`file` glob, `facts`, `issue`, `reason`) excuses it; the report (`target/nightly/differential-inspect/`) counts the files whose objects and font lists match. `--update` writes the tools' facts about 31 committed PDFs (a table, a cross-reference stream and a hybrid file among them) to `fixtures/differential/inspect.json`, which the PR lane's `crates/laminapdf/tests/inspect_differential.rs` compares with. Locally, without the corpus fetched, it compares the committed PDFs only.

| Inspection tool | Runner | Version pinned in |
| --- | --- | --- |
| qpdf | `qpdf --json=2`, `qpdf --show-xref`, run by xtask | `tools/xtask/src/doctor.rs` (`QPDF_VERSION`) |
| pdffonts | `pdffonts`, run by xtask (poppler, GPL-2.0: a separate program, never linked, ADR 0019) | `tools/xtask/src/doctor.rs` (`POPPLER_VERSION`; the nightly builds it with pdftotext in `.github/actions/poppler`) |

The form differential (todo E8o) compares the fields of each PDF form as three readers see them, over the external corpus and the PDFs in `fixtures/forms/`. lamina's side is `crates/laminapdf/examples/differential/form-facts.rs` (`Document::form`), pypdf's is `facts.py pypdf-forms`, and pdf.js's is `pdfjs.mjs --forms`, which reads each page's widget annotations (`fieldName`, `fieldType`, `fieldValue`). Each writes every terminal field's fully qualified name, kind and value, and xtask (`tools/xtask/src/live/forms.rs`) compares lamina with each tool on its own.

xtask pairs the fields by name and compares their kind and value. Kinds use lamina's names (`text`, `checkBox`, `radioGroup`, `pushButton`, `comboBox`, `listBox`, `signature`). Before comparing, xtask normalises each value: an empty text is none; an on-state drops pypdf's leading `/`, and `Off` is none; a choice is the list of its non-empty export values. Push button and signature values aren't compared.

Two runners adjust what their tool reports. pypdf lists every node of the field tree by name and leaves `/FT` and `/Ff` where the file put them, so its runner keeps only the leaf names and takes `/FT` and `/Ff` from the nearest ancestor that sets them. pdf.js reports a value per widget, and a check box widget whose on-state isn't the field's value reports `Off`, so its runner takes the field's value from the widget that turns on to it.

A difference fails the run unless `fixtures/differential/forms-exceptions.toml` excuses it. A `[[class]]` entry turns on one of `forms.rs`'s rules for a way one tool reads a form by design, such as pdf.js reading a text stream as empty, and gives its `reason`. A `[[known]]` entry excuses what is left in the files its `file` glob matches: the `facts` it lists (`name`, `kind`, `value`), with an `issue` and a `reason`. lamina's facts carry what the classes check, such as a button's `/Opt` exports, the names the widgets' `/Parent` chains give, and the fields below an unnamed node. The report goes to `target/nightly/differential-forms/` and keeps the field names and values out of report.md, since they are text from the files.

| Form tool | Runner | Version pinned in |
| --- | --- | --- |
| pypdf | `facts.py pypdf-forms` | `requirements.txt` |
| pdf.js | `pdfjs.mjs --forms` | `package.json` + `pnpm-lock.yaml` |

xtask runs lamina and the tools at once, each over chunks of 200 inputs, one process per chunk with a 20-minute limit (`tools/xtask/src/runner.rs`). A chunk that crashes, hangs or reports different files is rerun one file per process with a 2-minute limit, so a bad file becomes a `timeout` or `crash` entry (skipped for a tool, a failure for lamina) instead of stopping the run; a reader whose every file in a chunk crashes is reported broken and fails the run after the report is written. Inside a process, the runners also give up on a file after 60 s where they can (Python's `SIGALRM`, a timer in pdf.js), which native code and synchronous parsing can ignore. Each reader's output goes to `target/nightly/differential/<reader>.log`. Inputs are data: they are only read, never executed.

```sh
cargo xtask corpus fetch                                  # the external corpus (fixtures/corpus-manifest.toml)
pnpm --dir tools/differential install --frozen-lockfile   # pdf.js: its own pnpm project, outside the root workspace
python3 -m venv .venv && .venv/bin/pip install -r tools/differential/requirements.txt
LAMINA_PDF_PYTHON=.venv/bin/python cargo xtask differential --live [--sample 300] [--tools pdfjs,qpdf] [--corpus external|mutants|all] [--source pdfjs]
cargo xtask differential --live --text [--sample 50] [--source pdfjs]   # page text: pdf.js and pdftotext (no Python)
cargo xtask differential --live --xmp [--sample 50] [--source pdfjs]    # XMP values: ExifTool (no Python)
cargo xtask differential --live --inspect [--sample 50] [--source pdfjs]  # objects, sections and fonts: qpdf and pdffonts (no Python)
cargo xtask differential --live --inspect --update                     # rewrite fixtures/differential/inspect.json
LAMINA_PDF_PYTHON=.venv/bin/python cargo xtask differential --live --forms [--sample 40] [--source pdfjs]  # form fields: pypdf and pdf.js
```

qpdf, pdffonts and ExifTool must be on `PATH` at the versions `cargo xtask doctor` expects. All seven are tools only: nothing here ships or links into lamina (pdfjs-dist and pypdfium2 are Apache-2.0, PDFium BSD-3-Clause, qpdf Apache-2.0, pypdf BSD-3-Clause; pdftotext and pdffonts (GPL-2.0) and ExifTool (GPL-1.0-or-later or Artistic-1.0) run as separate programs and are never linked).
