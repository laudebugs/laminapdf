# Examples

Small, dependency-light scripts and the PDFs they produce, used to verify claims in the research chapters. Each PDF was checked with Poppler (`pdfinfo`, and `pdftotext`/`pdffonts`/`pdftoppm` where relevant).

| Folder | Chapter | What's here |
| --- | --- | --- |
| [`01-file-structure/`](01-file-structure/) | [01](../01-file-structure.md) | `hello.py` writes a 599-byte hand-built "Hello, World" PDF with computed xref offsets; `hello_xrefstm.py` writes the same with an object stream + xref stream; `variants.py` produces an incremental update and a deliberately broken xref. |
| [`02-imaging-model/`](02-imaging-model/) | [02](../02-imaging-model.md) | `make_imaging_demo.py`: filled rect, dashed Bézier circle, clipped axial gradient, rotated semi-transparent square. Rendered output in `imaging-demo.png`. |
| [`03-text-and-fonts/`](03-text-and-fonts/) | [03](../03-text-and-fonts.md) | `make_text_example.py` (needs `fonttools` and DejaVu Sans): a Type 0 / Identity-H subset font with `/W` widths and ToUnicode, a kerned Helvetica line, invisible text, a minimal structure tree. |
| [`04-signing-encryption/`](04-signing-encryption/) | [04](../04-interactive-security.md) | `demo.py` (needs `pyhanko`, `pypdf`): a PAdES-style signed PDF, a one-byte-tampered copy that fails validation, and an AES-256 encrypted PDF. Generate a throwaway key with the `openssl req` command in chapter 04 Appendix; no keys are committed. |

Usage: `python3 hello.py hello.pdf`, etc. Scripts write to the current directory or the path given as the first argument.
