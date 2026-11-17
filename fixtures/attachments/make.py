#!/usr/bin/env python3
"""Writes the attachment fixtures in fixtures/attachments/ (no dependencies).

The files are byte-reproducible: every value, the trailer /ID included, is fixed, and objects
are written in number order. Embedded files are stored unfiltered, so their bytes can be read in
the file.

    python3 fixtures/attachments/make.py

associated.pdf   two pages; the /EmbeddedFiles tree lists notes.txt (with a correct /CheckSum)
                 and readings.csv (whose /CheckSum is wrong on purpose); readings.csv is page 2's
                 associated file (/AF, /AFRelationship /Data), and a FileAttachment annotation on
                 page 1 holds notes.txt too
portfolio.pdf    a portfolio: /Collection with a schema (file name, description, a date and a
                 number column), a sort, the details view and an initial document; two files with
                 collection items (/CI)
"""

import hashlib
from pathlib import Path

HERE = Path(__file__).resolve().parent


class Pdf:
    """A PDF assembled from numbered object bodies."""

    def __init__(self, version="1.7"):
        self.version = version
        self.objects = {}

    def reserve(self):
        num = len(self.objects) + 1
        self.objects[num] = None
        return num

    def add(self, body):
        num = self.reserve()
        self.objects[num] = body
        return num

    def set(self, num, body):
        self.objects[num] = body

    def stream(self, data, extra=""):
        return self.add(b"<< %s/Length %d >>\nstream\n%s\nendstream" % (extra.encode(), len(data), data))

    def write(self, name, root, file_id):
        out = bytearray(b"%%PDF-%s\n%%\xe2\xe3\xcf\xd3\n" % self.version.encode())
        offsets = {}
        for num in sorted(self.objects):
            body = self.objects[num]
            assert body is not None, f"object {num} was reserved but never set"
            if isinstance(body, str):
                body = body.encode()
            offsets[num] = len(out)
            out += b"%d 0 obj\n%s\nendobj\n" % (num, body)
        size = len(self.objects) + 1
        xref = len(out)
        out += f"xref\n0 {size}\n0000000000 65535 f\r\n".encode()
        for num in range(1, size):
            out += f"{offsets[num]:010} 00000 n\r\n".encode()
        out += (
            f"trailer\n<< /Size {size} /Root {root} 0 R /ID [<{file_id}> <{file_id}>] >>\n"
            f"startxref\n{xref}\n%%EOF\n"
        ).encode()
        (HERE / name).write_bytes(out)
        print(f"wrote {name} ({len(out)} bytes)")


NOTES = b"Notes for the attachment fixtures.\n"
READINGS = b"day,reading\n1,12.5\n2,13.1\n"
REPORT = b"%PDF-1.7 (not a real PDF: a portfolio member's bytes)\n"


def embedded(pdf, data, mime, checksum=None, mod_date="D:20261101090000Z"):
    """An unfiltered embedded file stream with /Params (/Size, /ModDate, /CheckSum)."""
    digest = checksum if checksum is not None else hashlib.md5(data).digest()
    params = f"/Params << /Size {len(data)} /ModDate ({mod_date}) /CheckSum <{digest.hex()}> >> "
    return pdf.stream(data, f"/Type /EmbeddedFile /Subtype /{mime} {params}")


def spec(pdf, name, stream, desc, extra=""):
    return pdf.add(
        f"<< /Type /Filespec /F ({name}) /UF ({name}) /EF << /F {stream} 0 R /UF {stream} 0 R >>"
        f" /Desc ({desc}){extra} >>"
    )


def pages(pdf, count, annots=None):
    """A root /Pages with `count` pages drawing "Page n"; `annots` maps a page index to extra
    entries. Returns (root, page numbers)."""
    font = pdf.add("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>")
    root = pdf.reserve()
    nums = []
    for i in range(count):
        content = pdf.stream(b"BT /F1 24 Tf 72 700 Td (Page %d) Tj ET" % (i + 1))
        nums.append(pdf.reserve())
        extra = (annots or {}).get(i, "")
        pdf.set(nums[-1], f"<< /Type /Page /Parent {root} 0 R /Contents {content} 0 R{extra} >>")
    kids = " ".join(f"{n} 0 R" for n in nums)
    pdf.set(root, f"<< /Type /Pages /Kids [{kids}] /Count {count} /MediaBox [0 0 612 792]"
                  f" /Resources << /Font << /F1 {font} 0 R >> >> >>")
    return root, nums


def associated():
    pdf = Pdf()
    notes = spec(pdf, "notes.txt", embedded(pdf, NOTES, "text#2Fplain"), "Reviewer notes")
    readings = spec(pdf, "readings.csv", embedded(pdf, READINGS, "text#2Fcsv", checksum=bytes(16)),
                    "The readings page 2 charts", " /AFRelationship /Data")
    root = pdf.reserve()
    annot = pdf.reserve()
    tree, nums = pages(pdf, 2, {0: f" /Annots [{annot} 0 R]", 1: f" /AF [{readings} 0 R]"})
    pdf.set(annot, f"<< /Type /Annot /Subtype /FileAttachment /Rect [72 600 92 620] /P {nums[0]} 0 R"
                   f" /FS {notes} 0 R /Contents (Reviewer notes) /Name /Paperclip >>")
    pdf.set(root, f"<< /Type /Catalog /Pages {tree} 0 R"
                  f" /Names << /EmbeddedFiles << /Names [(notes.txt) {notes} 0 R (readings.csv)"
                  f" {readings} 0 R] >> >> >>")
    pdf.write("associated.pdf", root, "A1B2C3D4E5F60718293A4B5C6D7E8F90")


def portfolio():
    pdf = Pdf()
    report = spec(pdf, "report.pdf", embedded(pdf, REPORT, "application#2Fpdf"), "The quarterly report",
                  " /CI << /due (D:20261231000000Z) /pages 12 /owner (Finance) >>")
    notes = spec(pdf, "notes.txt", embedded(pdf, NOTES, "text#2Fplain"), "Reviewer notes",
                 " /CI << /owner << /Type /CollectionSubitem /D (Review) /P (Team: ) >> /pages 1 >>")
    root, _ = pages(pdf, 1)
    schema = ("/Schema << /Type /CollectionSchema"
              " /name << /Subtype /F /N (Name) /O 0 >>"
              " /desc << /Subtype /Desc /N (Description) /O 1 >>"
              " /owner << /Subtype /S /N (Owner) /O 2 /E true >>"
              " /due << /Subtype /D /N (Due) /O 3 >>"
              " /pages << /Subtype /N /N (Pages) /O 4 /V false >> >>")
    cat = pdf.add(f"<< /Type /Catalog /Pages {root} 0 R /PageMode /UseAttachments"
                  f" /Collection << /Type /Collection {schema} /D (report.pdf) /View /D"
                  f" /Sort << /S [/owner /due] /A [true false] >> >>"
                  f" /Names << /EmbeddedFiles << /Names [(notes.txt) {notes} 0 R (report.pdf) {report} 0 R] >> >> >>")
    pdf.write("portfolio.pdf", cat, "0F1E2D3C4B5A69788796A5B4C3D2E1F0")


if __name__ == "__main__":
    associated()
    portfolio()
