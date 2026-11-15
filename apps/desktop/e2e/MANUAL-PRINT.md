# Printing: manual checks

The print test and the e2e scenario print to each OS's PDF printer, but no runner has a real printer or a person to press Print. Check by hand, once per OS with a dev bundle (see [MANUAL-OPEN.md](MANUAL-OPEN.md#build-a-dev-bundle)), on a physical printer where you can. Record each run in the PR that changes printing.

## Fixtures

- `fixtures/print/mixed-50.pdf`: 50 pages, A4 portrait (one size, despite its name).
- `fixtures/print/mixed-sizes.pdf`: A4 portrait, US Letter landscape, A4 with `/Rotate 90` (shown landscape) and A5 portrait. Each page has a border 18 pt inside its edge and red corner squares, so clipping and scaling show.
- `spikes/desktop-print/fixtures/vectors.pdf`: hairlines, a fine grid and 4–8 pt text.

The spike's comparison tool (`cargo run --bin compare` in `spikes/desktop-print/src-tauri`) scores a print-to-PDF file against the original. The bar is SSIM ≥ 0.95 at 150 dpi, with each page's size and content within 2 pt.

## Checks

| # | Do | Expect |
| --- | --- | --- |
| 1 | Open `mixed-50.pdf`, File ▸ Print…, pages 5–12, print. | Exactly pages 5–12 print, at actual size, with no headers, footers or margins added. |
| 2 | Print `mixed-sizes.pdf` to the OS's PDF printer. | Windows and Linux: each page on paper of its own size and orientation. macOS: each page in its own orientation, on the job's paper ([#626](https://github.com/laudebugs/laminapdf/issues/626)). |
| 3 | Print `mixed-50.pdf` and press Cancel printing in the status bar (macOS, Windows) or GTK's window (Linux) after a few pages. | The job stops within a second, and the app stays responsive. macOS and Windows: nothing more reaches the queue. Linux: GTK ends the job. |
| 4 | Print with 2 copies, collated, duplex where the printer has it. | Two collated copies, two-sided where the printer supports it. |
| 5 | Print `vectors.pdf` to a PDF and score it with the comparison tool. | SSIM ≥ 0.95. |
| 6 | Watch the app's memory (Activity Monitor, Task Manager, `top`) while printing `mixed-50.pdf`. | The main process stays within about twice one page's raster at 300 dpi, about 70 MB for A4. |

## Results

| OS and version | Printer and driver | Check | Pages | Duplex | Result | SSIM |
| --- | --- | --- | --- | --- | --- | --- |
| | | | | | | |
