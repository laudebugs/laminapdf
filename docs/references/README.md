# References

External specifications, standards, tools and reading material we rely on. Group new entries by area.

## Specifications & standards

- **ISO 32000-2:2020 (PDF 2.0)** — the current PDF specification, available at no cost via the PDF Association: https://pdfa.org/sponsored-standards/
- **ISO 32000-1:2008 (PDF 1.7)** — Adobe's public copy: https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf
- **PDF 2.0 format description (Library of Congress)** — https://www.loc.gov/preservation/digital/formats/fdd/fdd000474.shtml
- **ISO 32000-2 errata, clause 12 (interactive features & annotations)** — https://pdf-issues.pdfa.org/32000-2-2020/clause12.html
- **ToUnicode mapping file tutorial (Adobe Technical Note #5411)** — https://pdfa.org/norm-refs/5411.ToUnicode.pdf
- **Arlington PDF Model** — machine-readable definition of every PDF object and key: https://github.com/pdf-association/arlington-pdf-model

## Library & engine source references

- **PDFium annotations API header (`fpdf_annot.h`)** — https://pdfium.googlesource.com/pdfium/+/main/public/fpdf_annot.h

## Technical guides & articles

| Topic | Link |
| --- | --- |
| PDF format internals (pypdf docs) | https://pypdf.readthedocs.io/en/5.3.1/dev/pdf-format.html |
| PDF file structure explained (Mapsoft) | https://mapsoft.com/posts/pdf-structure.html |
| Multiple trailers & incremental updates (IDR Solutions) | https://blog.idrsolutions.com/multiple-trailers-in-a-pdf-file/ |
| OCR & text-layer architecture (LlamaIndex) | https://www.llamaindex.ai/blog/pdf-character-recognition |
| Annotations & appearance streams (Nutrient / PSPDFKit) | https://www.nutrient.io/blog/what-are-annotations/ |

## Tools

- **pdfi** — PDF inspector: https://yingtongli.me/git/pdfi

## Products (for comparison)

- **Nitro Sign** — commercial e-signature product: https://checkout.gonitro.com/sign
- **Adobe Acrobat Pro** — the reference feature set for desktop PDF editing (see [research/pdf/06-feature-inventory.md](../research/pdf/06-feature-inventory.md)).

## Owner's research report

- *Technical and Academic Architecture of PDFs* (NotebookLM, 2026-10-28) — kept verbatim in [`research/pdf/sources/notebooklm-report.md`](../research/pdf/sources/notebooklm-report.md), with one correction noted.

Each chapter in [`docs/research/pdf/`](../research/pdf/) also ends with its own topic-specific reference list.
