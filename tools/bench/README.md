# Extraction benchmark

`cargo xtask extract-bench` scores Lamina PDF's extraction for LLMs on a public benchmark, so the numbers in [`crates/laminapdf-extract/README.md`](../../crates/laminapdf-extract/README.md#benchmark) can be checked and rerun. It scores reading order, tables, and text left out of the output, on born-digital PDFs only.

## What runs

1. **Fetch.** [`extract-datasets.toml`](extract-datasets.toml) pins each dataset to a revision. Its files go to `target/bench/datasets/` and are never committed, and each is checked against the SHA-256 in the dataset's lock ([`olmocr-bench.lock`](olmocr-bench.lock)).
2. **Extract.** The `extract-bench` example (`crates/laminapdf/examples/differential/`) writes each PDF's Markdown with the default options. A file that fails or panics gets an empty Markdown file, so it fails its own tests and nothing else.
3. **Score.** [`olmocr_score.py`](olmocr_score.py) runs the benchmark's own tests from the pinned `olmocr` package, so a score means what the benchmark's published table means.
4. **Report.** The run writes `target/bench/extract/<dataset>/`: the Markdown, each file's outcome and time, the scores per subset and test type, and a Markdown table of them.

## Running it

```sh
python3 -m pip install -r tools/bench/requirements.txt   # once; or set LAMINA_PDF_PYTHON to a virtualenv's python
cargo xtask extract-bench                                 # every subset
cargo xtask extract-bench --subset multi_column --sample 20
cargo xtask extract-bench --tables lattice-and-stream     # also look for tables without lines
```

`--update-lock` rewrites the lock from the files fetched. Use it only after changing a dataset's revision on purpose.

## Choosing the data

A rule-based extractor reads a PDF's text layer, so only born-digital PDFs are scored. The manifest records every dataset and subset left out, with the reason:

- scans, which need OCR;
- page images, which aren't PDFs;
- datasets whose licence isn't stated;
- datasets that ship no PDFs, or have no reading-order or table ground truth.

**olmOCR-bench** ([Allen Institute for AI](https://huggingface.co/datasets/allenai/olmOCR-bench), ODC-BY-1.0) gives 1,403 single-page PDFs with unit tests. Lamina PDF uses three of its seven subsets:

- `multi_column`: reading order;
- `table_tests`: table cells and their neighbours;
- `headers_footers`: running headers, footers and page numbers left out.

The scorer's code is olmOCR's (Apache-2.0); this folder holds only a small script that calls it.

Vendor scores compare only when the data and the metric are the same. The scores here are olmOCR-bench's own pass rates on these three subsets. Its published table averages all seven subsets, and four of them, the scans and the equations, need OCR or rendered LaTeX, which this extractor doesn't produce.
