# hayro-parity summary

1188 documents, 2196 rows, 150 DPI, engines: both, 6 parallel jobs, per-page timeout 30 s, wall time 710 s.

## Classes (default thresholds)

| corpus | files | pages | pass | minor | fail | hayro-error | pdfium-error | both-error | timeout | rendered |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| pdfbox | 175 | 317 | 256 (80.8 %) | 45 (14.2 %) | 6 (1.9 %) | 0 (0.0 %) | 1 (0.3 %) | 9 (2.8 %) | 0 (0.0 %) | 0 (0.0 %) |
| pdfjs | 1013 | 1879 | 1463 (77.9 %) | 222 (11.8 %) | 154 (8.2 %) | 5 (0.3 %) | 13 (0.7 %) | 17 (0.9 %) | 5 (0.3 %) | 0 (0.0 %) |
| total | 1188 | 2196 | 1719 (78.3 %) | 267 (12.2 %) | 160 (7.3 %) | 5 (0.2 %) | 14 (0.6 %) | 26 (1.2 %) | 5 (0.2 %) | 0 (0.0 %) |

## Threshold sensitivity

Pass / pass+minor as a share of **all** pages, and of the pages both engines rendered at the same size (`compared`).

| corpus | thresholds | pass | pass+minor | pass (of compared) | pass+minor (of compared) |
| --- | --- | ---: | ---: | ---: | ---: |
| pdfbox | strict (SSIM ≥ 0.95, ≤ 0.1 %; minor SSIM ≥ 0.9, ≤ 1 %) | 50.5 % | 87.1 % | 52.3 % | 90.2 % |
| pdfbox | default (SSIM ≥ 0.9, ≤ 0.5 %; minor SSIM ≥ 0.85, ≤ 2 %) | 80.8 % | 95.0 % | 83.7 % | 98.4 % |
| pdfbox | lenient (SSIM ≥ 0.85, ≤ 1 %; minor SSIM ≥ 0.8, ≤ 5 %) | 87.1 % | 96.5 % | 90.2 % | 100.0 % |
| pdfjs | strict (SSIM ≥ 0.95, ≤ 0.1 %; minor SSIM ≥ 0.9, ≤ 1 %) | 33.8 % | 83.9 % | 34.6 % | 85.8 % |
| pdfjs | default (SSIM ≥ 0.9, ≤ 0.5 %; minor SSIM ≥ 0.85, ≤ 2 %) | 77.9 % | 89.7 % | 79.6 % | 91.7 % |
| pdfjs | lenient (SSIM ≥ 0.85, ≤ 1 %; minor SSIM ≥ 0.8, ≤ 5 %) | 83.9 % | 94.1 % | 85.8 % | 96.2 % |
| total | strict (SSIM ≥ 0.95, ≤ 0.1 %; minor SSIM ≥ 0.9, ≤ 1 %) | 36.2 % | 84.3 % | 37.1 % | 86.4 % |
| total | default (SSIM ≥ 0.9, ≤ 0.5 %; minor SSIM ≥ 0.85, ≤ 2 %) | 78.3 % | 90.4 % | 80.2 % | 92.7 % |
| total | lenient (SSIM ≥ 0.85, ≤ 1 %; minor SSIM ≥ 0.8, ≤ 5 %) | 84.3 % | 94.4 % | 86.4 % | 96.8 % |

## Speed and memory

Per-page render time over pages the engine rendered without error. Peak RSS is per child process (one document, both engines loaded in a two-engine run).

| corpus | hayro median | hayro p95 | hayro total | PDFium median | PDFium p95 | PDFium total | pages hayro faster | peak RSS median | peak RSS p95 | peak RSS max |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| pdfbox | 12.1 ms | 192.8 ms | 12.6 s | 16.4 ms | 227.6 ms | 16.0 s | 59.9 % | 113.3 MiB | 131.5 MiB | 147.2 MiB |
| pdfjs | 12.2 ms | 321.6 ms | 193.6 s | 13.1 ms | 331.2 ms | 147.3 s | 57.2 % | 62.2 MiB | 130.5 MiB | 1184.6 MiB |
| total | 12.2 ms | 302.5 ms | 206.2 s | 13.3 ms | 316.3 ms | 163.3 s | 57.5 % | 86.8 MiB | 131.2 MiB | 1184.6 MiB |

## Encrypted documents

40 pages from files containing `/Encrypt`:
- pass: 9
- minor: 7
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
| `UnresolvedAnnotationAppearance` | 5 | 7 | 1 |
| `ImageDecodeFailure` | 5 | 0 | 0 |
