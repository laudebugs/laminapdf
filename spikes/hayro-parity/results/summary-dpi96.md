# hayro-parity summary

1188 documents, 2196 rows, 96 DPI, engines: both, 6 parallel jobs, per-page timeout 30 s, wall time 410 s.

## Classes (default thresholds)

| corpus | files | pages | pass | minor | fail | hayro-error | pdfium-error | both-error | timeout | rendered |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| pdfbox | 175 | 317 | 277 (87.4 %) | 20 (6.3 %) | 10 (3.2 %) | 0 (0.0 %) | 1 (0.3 %) | 9 (2.8 %) | 0 (0.0 %) | 0 (0.0 %) |
| pdfjs | 1013 | 1879 | 1520 (80.9 %) | 176 (9.4 %) | 143 (7.6 %) | 5 (0.3 %) | 13 (0.7 %) | 17 (0.9 %) | 5 (0.3 %) | 0 (0.0 %) |
| total | 1188 | 2196 | 1797 (81.8 %) | 196 (8.9 %) | 153 (7.0 %) | 5 (0.2 %) | 14 (0.6 %) | 26 (1.2 %) | 5 (0.2 %) | 0 (0.0 %) |

## Threshold sensitivity

Pass / pass+minor as a share of **all** pages, and of the pages both engines rendered at the same size (`compared`).

| corpus | thresholds | pass | pass+minor | pass (of compared) | pass+minor (of compared) |
| --- | --- | ---: | ---: | ---: | ---: |
| pdfbox | strict (SSIM ≥ 0.95, ≤ 0.1 %; minor SSIM ≥ 0.9, ≤ 1 %) | 69.1 % | 90.5 % | 71.6 % | 93.8 % |
| pdfbox | default (SSIM ≥ 0.9, ≤ 0.5 %; minor SSIM ≥ 0.85, ≤ 2 %) | 87.4 % | 93.7 % | 90.5 % | 97.1 % |
| pdfbox | lenient (SSIM ≥ 0.85, ≤ 1 %; minor SSIM ≥ 0.8, ≤ 5 %) | 90.5 % | 95.6 % | 93.8 % | 99.0 % |
| pdfjs | strict (SSIM ≥ 0.95, ≤ 0.1 %; minor SSIM ≥ 0.9, ≤ 1 %) | 62.5 % | 86.5 % | 64.0 % | 88.5 % |
| pdfjs | default (SSIM ≥ 0.9, ≤ 0.5 %; minor SSIM ≥ 0.85, ≤ 2 %) | 80.9 % | 90.3 % | 82.7 % | 92.3 % |
| pdfjs | lenient (SSIM ≥ 0.85, ≤ 1 %; minor SSIM ≥ 0.8, ≤ 5 %) | 86.5 % | 94.7 % | 88.5 % | 96.9 % |
| total | strict (SSIM ≥ 0.95, ≤ 0.1 %; minor SSIM ≥ 0.9, ≤ 1 %) | 63.5 % | 87.1 % | 65.0 % | 89.2 % |
| total | default (SSIM ≥ 0.9, ≤ 0.5 %; minor SSIM ≥ 0.85, ≤ 2 %) | 81.8 % | 90.8 % | 83.9 % | 93.0 % |
| total | lenient (SSIM ≥ 0.85, ≤ 1 %; minor SSIM ≥ 0.8, ≤ 5 %) | 87.1 % | 94.9 % | 89.2 % | 97.2 % |

## Speed and memory

Per-page render time over pages the engine rendered without error. Peak RSS is per child process (one document, both engines loaded in a two-engine run).

| corpus | hayro median | hayro p95 | hayro total | PDFium median | PDFium p95 | PDFium total | pages hayro faster | peak RSS median | peak RSS p95 | peak RSS max |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| pdfbox | 5.6 ms | 162.8 ms | 9.3 s | 6.3 ms | 149.2 ms | 8.7 s | 60.6 % | 58.0 MiB | 66.8 MiB | 107.8 MiB |
| pdfjs | 5.5 ms | 288.4 ms | 147.4 s | 5.3 ms | 252.9 ms | 118.4 s | 57.4 % | 37.7 MiB | 72.5 MiB | 957.8 MiB |
| total | 5.5 ms | 263.9 ms | 156.7 s | 5.5 ms | 240.9 ms | 127.1 s | 57.8 % | 48.2 MiB | 70.7 MiB | 957.8 MiB |

## Encrypted documents

40 pages from files containing `/Encrypt`:
- pass: 12
- minor: 4
- hayro-error: 2
- pdfium-error: 4
- both-error: 18

## Harness

- documents whose child crashed after reporting every page (teardown, not a page failure): 0
- pages whose triptych could not be written: 0

## Top hayro errors (pages)

- 12 × `Decryption(PasswordProtected)`
- 7 × `Invalid`
- 4 × `Decryption(UnsupportedAlgorithm)`
- 3 × `load: Invalid`
- 2 × `no such page`
- 2 × `timeout (## s)`
- 1 × `load: Decryption(PasswordProtected)`
- 1 × `load: no pages`
- 1 × `panic: called `Option::unwrap()` on a `None` value`

## Top PDFium errors (pages)

- 13 × `PdfiumLibraryInternalError(PasswordError)`
- 6 × `load: no pages`
- 5 × `PdfiumLibraryInternalError(FormatError)`
- 4 × `PdfiumLibraryInternalError(SecurityError)`
- 4 × `load: PdfiumLibraryInternalError(FormatError)`
- 3 × `timeout (## s)`
- 2 × `load: PdfiumLibraryInternalError(PasswordError)`
- 2 × `page: PdfiumLibraryInternalError(Unknown)`
- 1 × `crash (signal #)`
- 1 × `empty page box ###x##########`
- 1 × `empty page box #x#`
- 1 × `no pages`

## hayro warnings by class (pages raising each warning)

| warning | pass | minor | fail |
| --- | ---: | ---: | ---: |
| `UnresolvedAnnotationAppearance` | 7 | 2 | 4 |
| `ImageDecodeFailure` | 5 | 0 | 0 |
