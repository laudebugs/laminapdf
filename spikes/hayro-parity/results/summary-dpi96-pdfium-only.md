# hayro-parity summary

1188 documents, 2193 rows, 96 DPI, engines: pdfium, 6 parallel jobs, per-page timeout 30 s, wall time 120 s.

## Classes (default thresholds)

| corpus | files | pages | pass | minor | fail | hayro-error | pdfium-error | both-error | timeout | rendered |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| pdfbox | 175 | 317 | 0 (0.0 %) | 0 (0.0 %) | 0 (0.0 %) | 0 (0.0 %) | 10 (3.2 %) | 0 (0.0 %) | 0 (0.0 %) | 307 (96.8 %) |
| pdfjs | 1013 | 1876 | 0 (0.0 %) | 0 (0.0 %) | 0 (0.0 %) | 0 (0.0 %) | 27 (1.4 %) | 0 (0.0 %) | 4 (0.2 %) | 1845 (98.3 %) |
| total | 1188 | 2193 | 0 (0.0 %) | 0 (0.0 %) | 0 (0.0 %) | 0 (0.0 %) | 37 (1.7 %) | 0 (0.0 %) | 4 (0.2 %) | 2152 (98.1 %) |

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
| pdfbox | – | – | -0.0 s | 6.6 ms | 119.0 ms | 7.6 s | – | 27.2 MiB | 36.0 MiB | 47.8 MiB |
| pdfjs | – | – | -0.0 s | 6.5 ms | 250.6 ms | 132.7 s | – | 26.6 MiB | 44.7 MiB | 325.0 MiB |
| total | – | – | -0.0 s | 6.6 ms | 227.9 ms | 140.3 s | – | 26.7 MiB | 44.2 MiB | 325.0 MiB |

## Encrypted documents

40 pages from files containing `/Encrypt`:
- pdfium-error: 22
- rendered: 18

## Harness

- documents whose child crashed after reporting every page (teardown, not a page failure): 0
- pages whose triptych could not be written: 0

## Top hayro errors (pages)


## Top PDFium errors (pages)

- 15 × `PdfiumLibraryInternalError(PasswordError)`
- 9 × `PdfiumLibraryInternalError(FormatError)`
- 4 × `PdfiumLibraryInternalError(SecurityError)`
- 4 × `no pages`
- 4 × `timeout (## s)`
- 2 × `page: PdfiumLibraryInternalError(Unknown)`
- 1 × `crash (signal #)`
- 1 × `empty page box ###x##########`
- 1 × `empty page box #x#`

## hayro warnings by class (pages raising each warning)

| warning | pass | minor | fail |
| --- | ---: | ---: | ---: |
