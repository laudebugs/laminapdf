# Extraction benchmark

`cargo xtask extract-bench` scores Lamina PDF's extraction for LLMs on a public benchmark, so the numbers in [`crates/laminapdf-extract/README.md`](../../crates/laminapdf-extract/README.md#how-it-scores-on-a-public-benchmark) can be checked and rerun. It scores reading order, tables, and text left out of the output, on the benchmark's subsets that aren't made of scans.

## What runs

1. **Fetch.** [`extract-datasets.toml`](extract-datasets.toml) pins each dataset to a revision. Its files go to `target/bench/datasets/` and are never committed. Each is checked against the SHA-256 in the dataset's lock ([`olmocr-bench.lock`](olmocr-bench.lock)).
2. **Extract.** The `extract-bench` example (`crates/laminapdf/examples/differential/`) writes each PDF's Markdown with the default options, under the runner the differential uses: chunks of files under time limits. A file that fails, panics, crashes or hangs gets an empty Markdown file, so it fails its own tests and nothing else.
3. **Score.** [`olmocr_score.py`](olmocr_score.py) runs the benchmark's own tests from the pinned `olmocr` package, with the baseline test the benchmark adds for every page, and averages the groups as the benchmark does.
4. **Report.** The run writes `target/bench/extract/<dataset>/`: the Markdown, each file's outcome and time, the scores per subset and test type, and a Markdown table of them.

## Running it

```sh
python3 -m pip install -r tools/bench/requirements.txt   # once; or set LAMINA_PDF_PYTHON to a virtualenv's python
cargo xtask extract-bench                                 # every subset
cargo xtask extract-bench --subset multi_column --sample 20
cargo xtask extract-bench --tables lattice-and-stream     # also look for tables without lines
```

`--update-lock` rewrites the lock from the files fetched, on a full run only. Use it only after changing a dataset's revision on purpose. A cached file that doesn't match the lock is downloaded again before the run fails.

## Choosing the data

A rule-based extractor reads a PDF's text layer, so the subsets made of scans are left out. The few scans inside the other subsets stay in and count against the score. The manifest records every dataset and subset left out, with the reason:

- scans, which need OCR;
- page images, which aren't PDFs;
- datasets whose licence isn't stated;
- datasets that ship no PDFs, or have no reading-order or table ground truth.

**olmOCR-bench** ([Allen Institute for AI](https://huggingface.co/datasets/allenai/olmOCR-bench), ODC-BY-1.0) gives 1,403 single-page PDFs with unit tests. Lamina PDF uses three of its seven subsets:

- `multi_column`: reading order;
- `table_tests`: table cells and their neighbours;
- `headers_footers`: running headers, footers and page numbers left out.

The scorer's code is olmOCR's (Apache-2.0); this folder holds only a small script that calls it.

So these scores compare with a vendor's only when the data and the metric are the same. olmOCR-bench's published table averages all seven subsets, four of which, the scans and the equations, need OCR or rendered LaTeX. The scores here are its own pass rates on the three subsets a text-layer extractor can read.
