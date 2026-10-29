# hayro-parity summary

1188 documents, 2193 rows, 96 DPI, engines: pdfium, 6 parallel jobs, per-page timeout 30 s, wall time 66 s.

## Classes (default thresholds)

| corpus | files | pages | pass | minor | fail | hayro-error | pdfium-error | both-error | timeout | rendered |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| pdfbox | 175 | 317 | 0 (0.0 %) | 0 (0.0 %) | 0 (0.0 %) | 0 (0.0 %) | 10 (3.2 %) | 0 (0.0 %) | 0 (0.0 %) | 307 (96.8 %) |
| pdfjs | 1013 | 1876 | 0 (0.0 %) | 0 (0.0 %) | 0 (0.0 %) | 0 (0.0 %) | 27 (1.4 %) | 0 (0.0 %) | 3 (0.2 %) | 1846 (98.4 %) |
| total | 1188 | 2193 | 0 (0.0 %) | 0 (0.0 %) | 0 (0.0 %) | 0 (0.0 %) | 37 (1.7 %) | 0 (0.0 %) | 3 (0.1 %) | 2153 (98.2 %) |

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
| pdfbox | – | – | -0.0 s | 4.1 ms | 21.8 ms | 2.2 s | – | 27.2 MiB | 35.7 MiB | 47.6 MiB |
| pdfjs | – | – | -0.0 s | 3.4 ms | 86.6 ms | 61.2 s | – | 26.7 MiB | 46.4 MiB | 449.2 MiB |
| total | – | – | -0.0 s | 3.5 ms | 71.6 ms | 63.4 s | – | 26.8 MiB | 44.4 MiB | 449.2 MiB |

## Encrypted documents

40 pages from files containing `/Encrypt`:
- pdfium-error: 22
- rendered: 18

## Top hayro errors (pages)


## Top PDFium errors (pages)

- 15 × `PdfiumLibraryInternalError(PasswordError)`
- 9 × `PdfiumLibraryInternalError(FormatError)`
- 4 × `PdfiumLibraryInternalError(SecurityError)`
- 4 × `no pages`
- 3 × `timeout (## s)`
- 2 × `page: PdfiumLibraryInternalError(Unknown)`
- 1 × `crash (signal #)`
- 1 × `empty page box ###x##########`
- 1 × `empty page box #x#`

## hayro warnings by class (pages raising each warning)

| warning | pass | minor | fail |
| --- | ---: | ---: | ---: |
