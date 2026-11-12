# XMP packets other readers accept were read as malformed

[Resolution](resolution.md).

## What we saw

The nightly XMP differential failed on its first full run (2026-11-12, run `43327861926`): 107 fields in 20 files differed from ExifTool 12.76's, none excused. In 13 of those files, Lamina PDF reported the packet as malformed (`xmp-invalid`) and read no field, while ExifTool read them all:

- 11 of PDFium's XFA test files (`testing/resources/xfa/` and the `xfa_specific` folders);
- pdf.js's `bug854315.pdf` and `issue20453.pdf`.

A malformed packet reads as absent, so these files showed no title, dates or IDs, and an edit had to replace the packet (`malformed-xmp` unless `replaceMalformed`).

## How to reproduce

`cargo xtask corpus fetch --source pdfium --source pdfjs`, then read any of the 13 files' packets with `laminapdf_model::xmp::Packet::parse`: the XFA files fail with `Encoding`, the two pdf.js files with `UnboundPrefix`.

## Cause

Two kinds of damage that ExifTool and pdf.js read past.

- **A mangled byte-order mark.** The XFA files' header reads `<?xpacket begin="` then `EF BB 3F`: the mark's last byte became `?`. The packet is then not UTF-8, and the reader refused it whole. XML never reads inside a processing instruction, so the damage changes nothing the packet says.
- **Undeclared standard prefixes.**
  - `bug854315.pdf` declares `xap:` for the XMP basic schema but writes `xmp:ModifyDate`.
  - `issue20453.pdf` writes `dc:title` and `dc:description` without declaring `dc:`.

  The reader followed Namespaces in XML and refused a prefix with no declaration.

The other 7 files are differences in how the two readers read valid packets, or files ExifTool can't read. Lamina PDF reads them correctly (pypdf agrees where it reads them), and `fixtures/differential/xmp-exceptions.toml` excuses each, tracked by [#527](https://github.com/laudebugs/laminapdf/issues/527).
