# hayro-parity summary

1188 documents, 2196 rows, 150 DPI, engines: both, 6 parallel jobs, per-page timeout 30 s, wall time 853 s.

## Classes (default thresholds)

| corpus | files | pages | pass | minor | fail | hayro-error | pdfium-error | both-error | timeout | rendered |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| pdfbox | 175 | 317 | 256 (80.8 %) | 45 (14.2 %) | 6 (1.9 %) | 0 (0.0 %) | 1 (0.3 %) | 9 (2.8 %) | 0 (0.0 %) | 0 (0.0 %) |
| pdfjs | 1013 | 1879 | 1462 (77.8 %) | 222 (11.8 %) | 153 (8.1 %) | 5 (0.3 %) | 13 (0.7 %) | 17 (0.9 %) | 7 (0.4 %) | 0 (0.0 %) |
| total | 1188 | 2196 | 1718 (78.2 %) | 267 (12.2 %) | 159 (7.2 %) | 5 (0.2 %) | 14 (0.6 %) | 26 (1.2 %) | 7 (0.3 %) | 0 (0.0 %) |

## Threshold sensitivity

Pass / pass+minor as a share of **all** pages, and of the pages both engines rendered at the same size (`compared`).

| corpus | thresholds | pass | pass+minor | pass (of compared) | pass+minor (of compared) |
| --- | --- | ---: | ---: | ---: | ---: |
| pdfbox | strict (SSIM ≥ 0.95, ≤ 0.1 %; minor SSIM ≥ 0.9, ≤ 1 %) | 50.5 % | 87.1 % | 52.3 % | 90.2 % |
| pdfbox | default (SSIM ≥ 0.9, ≤ 0.5 %; minor SSIM ≥ 0.85, ≤ 2 %) | 80.8 % | 95.0 % | 83.7 % | 98.4 % |
| pdfbox | lenient (SSIM ≥ 0.85, ≤ 1 %; minor SSIM ≥ 0.8, ≤ 5 %) | 87.1 % | 96.5 % | 90.2 % | 100.0 % |
| pdfjs | strict (SSIM ≥ 0.95, ≤ 0.1 %; minor SSIM ≥ 0.9, ≤ 1 %) | 33.8 % | 83.8 % | 34.7 % | 85.8 % |
| pdfjs | default (SSIM ≥ 0.9, ≤ 0.5 %; minor SSIM ≥ 0.85, ≤ 2 %) | 77.8 % | 89.6 % | 79.7 % | 91.8 % |
| pdfjs | lenient (SSIM ≥ 0.85, ≤ 1 %; minor SSIM ≥ 0.8, ≤ 5 %) | 83.8 % | 94.0 % | 85.8 % | 96.3 % |
| total | strict (SSIM ≥ 0.95, ≤ 0.1 %; minor SSIM ≥ 0.9, ≤ 1 %) | 36.2 % | 84.3 % | 37.2 % | 86.5 % |
| total | default (SSIM ≥ 0.9, ≤ 0.5 %; minor SSIM ≥ 0.85, ≤ 2 %) | 78.2 % | 90.4 % | 80.2 % | 92.7 % |
| total | lenient (SSIM ≥ 0.85, ≤ 1 %; minor SSIM ≥ 0.8, ≤ 5 %) | 84.3 % | 94.4 % | 86.5 % | 96.8 % |

## Speed and memory

Per-page render time over pages the engine rendered without error. Peak RSS is per child process (one document, both engines loaded in a two-engine run).

| corpus | hayro median | hayro p95 | hayro total | PDFium median | PDFium p95 | PDFium total | pages hayro faster | peak RSS median | peak RSS p95 | peak RSS max |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| pdfbox | 12.4 ms | 193.4 ms | 12.7 s | 16.3 ms | 192.5 ms | 14.8 s | 57.3 % | 110.0 MiB | 132.5 MiB | 158.3 MiB |
| pdfjs | 12.4 ms | 399.0 ms | 190.1 s | 16.0 ms | 367.3 ms | 176.8 s | 57.7 % | 57.3 MiB | 125.2 MiB | 1526.6 MiB |
| total | 12.4 ms | 349.2 ms | 202.8 s | 16.1 ms | 355.8 ms | 191.6 s | 57.6 % | 66.6 MiB | 129.1 MiB | 1526.6 MiB |

## Encrypted documents

40 pages from files containing `/Encrypt`:
- pass: 9
- minor: 7
- hayro-error: 2
- pdfium-error: 4
- both-error: 18

## Top hayro errors (pages)

- 12 × `Decryption(PasswordProtected)`
- 7 × `Invalid`
- 4 × `Decryption(UnsupportedAlgorithm)`
- 3 × `load: Invalid`
- 3 × `timeout (## s)`
- 2 × `no such page`
- 1 × `load: Decryption(PasswordProtected)`
- 1 × `load: no pages`
- 1 × `panic: called `Option::unwrap()` on a `None` value`

## Top PDFium errors (pages)

- 13 × `PdfiumLibraryInternalError(PasswordError)`
- 6 × `load: no pages`
- 5 × `PdfiumLibraryInternalError(FormatError)`
- 4 × `PdfiumLibraryInternalError(SecurityError)`
- 4 × `load: PdfiumLibraryInternalError(FormatError)`
- 4 × `timeout (## s)`
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
