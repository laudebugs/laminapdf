# hayro-parity summary

1188 documents, 2194 rows, 96 DPI, engines: hayro, 6 parallel jobs, per-page timeout 30 s, wall time 33 s.

## Classes (default thresholds)

| corpus | files | pages | pass | minor | fail | hayro-error | pdfium-error | both-error | timeout | rendered |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| pdfbox | 175 | 317 | 0 (0.0 %) | 0 (0.0 %) | 0 (0.0 %) | 9 (2.8 %) | 0 (0.0 %) | 0 (0.0 %) | 0 (0.0 %) | 308 (97.2 %) |
| pdfjs | 1013 | 1877 | 0 (0.0 %) | 0 (0.0 %) | 0 (0.0 %) | 20 (1.1 %) | 0 (0.0 %) | 0 (0.0 %) | 1 (0.1 %) | 1856 (98.9 %) |
| total | 1188 | 2194 | 0 (0.0 %) | 0 (0.0 %) | 0 (0.0 %) | 29 (1.3 %) | 0 (0.0 %) | 0 (0.0 %) | 1 (0.0 %) | 2164 (98.6 %) |

## Threshold sensitivity

Pass / pass+minor as a share of **all** pages, and of the pages both engines rendered at the same size (`compared`).

| corpus | thresholds | pass | pass+minor | pass (of compared) | pass+minor (of compared) |
| --- | --- | ---: | ---: | ---: | ---: |
| pdfbox | strict (SSIM ≥ 0.95, ≤ 0.1 %; minor SSIM ≥ 0.9, ≤ 1 %) | 0.0 % | 0.0 % | 0.0 % | 0.0 % |
| pdfbox | default (SSIM ≥ 0.9, ≤ 0.5 %; minor SSIM ≥ 0.85, ≤ 2 %) | 0.0 % | 0.0 % | 0.0 % | 0.0 % |
| pdfbox | lenient (SSIM ≥ 0.85, ≤ 1 %; minor SSIM ≥ 0.8, ≤ 5 %) | 0.0 % | 0.0 % | 0.0 % | 0.0 % |
| pdfjs | strict (SSIM ≥ 0.95, ≤ 0.1 %; minor SSIM ≥ 0.9, ≤ 1 %) | 0.0 % | 0.0 % | 0.0 % | 0.0 % |
| pdfjs | default (SSIM ≥ 0.9, ≤ 0.5 %; minor SSIM ≥ 0.85, ≤ 2 %) | 0.0 % | 0.0 % | 0.0 % | 0.0 % |
| pdfjs | lenient (SSIM ≥ 0.85, ≤ 1 %; minor SSIM ≥ 0.8, ≤ 5 %) | 0.0 % | 0.0 % | 0.0 % | 0.0 % |
| total | strict (SSIM ≥ 0.95, ≤ 0.1 %; minor SSIM ≥ 0.9, ≤ 1 %) | 0.0 % | 0.0 % | 0.0 % | 0.0 % |
| total | default (SSIM ≥ 0.9, ≤ 0.5 %; minor SSIM ≥ 0.85, ≤ 2 %) | 0.0 % | 0.0 % | 0.0 % | 0.0 % |
| total | lenient (SSIM ≥ 0.85, ≤ 1 %; minor SSIM ≥ 0.8, ≤ 5 %) | 0.0 % | 0.0 % | 0.0 % | 0.0 % |

## Speed and memory

Per-page render time over pages the engine rendered without error. Peak RSS is per child process (one document, both engines loaded in a two-engine run).

| corpus | hayro median | hayro p95 | hayro total | PDFium median | PDFium p95 | PDFium total | pages hayro faster | peak RSS median | peak RSS p95 | peak RSS max |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| pdfbox | 4.9 ms | 45.2 ms | 3.5 s | – | – | -0.0 s | – | 16.0 MiB | 22.3 MiB | 27.6 MiB |
| pdfjs | 4.9 ms | 148.2 ms | 101.4 s | – | – | -0.0 s | – | 15.5 MiB | 28.9 MiB | 1570.8 MiB |
| total | 4.9 ms | 131.3 ms | 104.9 s | – | – | -0.0 s | – | 15.8 MiB | 27.5 MiB | 1570.8 MiB |

## Encrypted documents

40 pages from files containing `/Encrypt`:
- hayro-error: 20
- rendered: 20

## Top hayro errors (pages)

- 13 × `Decryption(PasswordProtected)`
- 10 × `Invalid`
- 4 × `Decryption(UnsupportedAlgorithm)`
- 1 × `no pages`
- 1 × `panic: called `Option::unwrap()` on a `None` value`
- 1 × `timeout (## s)`

## Top PDFium errors (pages)


## hayro warnings by class (pages raising each warning)

| warning | pass | minor | fail |
| --- | ---: | ---: | ---: |
