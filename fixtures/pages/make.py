#!/usr/bin/env python3
"""Writes the page-organisation fixtures in fixtures/pages/ (no dependencies).

Every page draws its marker, "Page N" in Helvetica, so a test can tell pages apart by their text
after they move. The files are byte-reproducible: every value, the trailer /ID included, is
fixed, and objects are written in number order.

    python3 fixtures/pages/make.py

inherited.pdf    a three-level tree with inherited /Resources, /MediaBox, /CropBox and /Rotate
unbalanced.pdf   a deep one-kid chain (40 levels) and a page dictionary listed twice
references.pdf   outline items, links, a GoTo action, named destinations in the name tree and
                 the PDF 1.1 /Dests dictionary, /OpenAction, structure elements and widgets,
                 all at page 3, plus some at other pages
labels.pdf       roman front matter, an arabic body, a prefixed appendix, a prefix-only page
boxes.pdf        all five boxes, /UserUnit, each rotation
forms.pdf        a merged field and widget, a field with two kid widgets, an unsigned signature
layers.pdf       two optional content groups
chapters.pdf     three top-level bookmarks with sections, one through a named destination
"""

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


def marker(pdf, n, extra=b""):
    """The content stream that draws "Page n" (font /F1), plus `extra`."""
    return pdf.stream(b"BT /F1 24 Tf 72 700 Td (Page %d) Tj ET%s" % (n, extra))


def font(pdf):
    return pdf.add("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>")


def resources(font_num):
    return f"<< /Font << /F1 {font_num} 0 R >> >>"


def page(parent, contents, extra=""):
    return f"<< /Type /Page /Parent {parent} 0 R /Contents {contents} 0 R{extra} >>"


def kids(nums):
    return "[" + " ".join(f"{n} 0 R" for n in nums) + "]"


def flat(pdf, count, media="[0 0 612 792]", first=1):
    """A root /Pages with `count` pages; returns (root, page numbers)."""
    f = font(pdf)
    root = pdf.reserve()
    pages = [pdf.add(page(root, marker(pdf, first + i))) for i in range(count)]
    pdf.set(root, f"<< /Type /Pages /Kids {kids(pages)} /Count {count} /MediaBox {media} /Resources {resources(f)} >>")
    return root, pages


def inherited():
    """Root (MediaBox, Resources) -> A (Rotate 90, CropBox) -> pages 1-3;
    root -> B (own MediaBox, indirect Resources) -> pages 4-5 and C (Rotate 180) -> page 6;
    root -> page 7 (own Rotate 270); root -> D (CropBox, Rotate null) -> page 8 (own Resources)."""
    pdf = Pdf()
    f = font(pdf)
    res_b = pdf.add(resources(f))
    root, a, b, c, d = (pdf.reserve() for _ in range(5))
    p = [None] + [pdf.reserve() for _ in range(8)]
    for n in range(1, 9):
        parent = {1: a, 2: a, 3: a, 4: b, 5: b, 6: c, 7: root, 8: d}[n]
        extra = {7: " /Rotate 270", 8: f" /Resources {resources(f)}", 5: " /CropBox [10 10 500 700]"}.get(n, "")
        pdf.set(p[n], page(parent, marker(pdf, n), extra))
    pdf.set(root, f"<< /Type /Pages /Kids {kids([a, b, p[7], d])} /Count 8 /MediaBox [0 0 612 792]"
            f" /Resources {resources(f)} >>")
    pdf.set(a, f"<< /Type /Pages /Parent {root} 0 R /Kids {kids(p[1:4])} /Count 3 /Rotate 90"
            " /CropBox [36 36 576 756] >>")
    pdf.set(b, f"<< /Type /Pages /Parent {root} 0 R /Kids {kids([p[4], p[5], c])} /Count 3"
            f" /MediaBox [0 0 595 842] /Resources {res_b} 0 R >>")
    pdf.set(c, f"<< /Type /Pages /Parent {b} 0 R /Kids {kids([p[6]])} /Count 1 /Rotate 180 >>")
    pdf.set(d, f"<< /Type /Pages /Parent {root} 0 R /Kids {kids([p[8]])} /Count 1 /CropBox [72 72 540 720]"
            " /Rotate null >>")
    cat = pdf.add(f"<< /Type /Catalog /Pages {root} 0 R >>")
    pdf.write("inherited.pdf", cat, "0E20A1D2B3C4D5E6F708192A3B4C5D6E")


def unbalanced():
    """Root -> pages 1-6, then a chain of 40 one-kid nodes (Rotate 90 at level 10, a MediaBox at
    level 20) with a page at levels 5, 15, 25, 35 and 40; page 3's dictionary is listed twice
    (as pages 3 and 12)."""
    pdf = Pdf()
    f = font(pdf)
    root = pdf.reserve()
    flat_pages = [pdf.reserve() for _ in range(6)]
    chain = [pdf.reserve() for _ in range(40)]
    n = 0
    for i, num in enumerate(flat_pages):
        n += 1
        pdf.set(num, page(root, marker(pdf, n)))
    chain_pages = {}
    for level in (5, 15, 25, 35, 40):
        n += 1
        chain_pages[level] = pdf.add(page(chain[level - 1], marker(pdf, n)))
    for level in range(40, 0, -1):
        num = chain[level - 1]
        parent = root if level == 1 else chain[level - 2]
        sub = [] if level == 40 else [chain[level]]
        own = [chain_pages[level]] if level in chain_pages else []
        count = sum(1 for lv in chain_pages if lv >= level)
        extra = {10: " /Rotate 90", 20: " /MediaBox [0 0 420 595]"}.get(level, "")
        pdf.set(num, f"<< /Type /Pages /Parent {parent} 0 R /Kids {kids(own + sub)} /Count {count}{extra} >>")
    root_kids = flat_pages + [chain[0], flat_pages[2]]
    pdf.set(root, f"<< /Type /Pages /Kids {kids(root_kids)} /Count {len(flat_pages) + 1 + len(chain_pages)}"
            f" /MediaBox [0 0 612 792] /Resources {resources(f)} >>")
    cat = pdf.add(f"<< /Type /Catalog /Pages {root} 0 R >>")
    pdf.write("unbalanced.pdf", cat, "1F31B2E3C4D5E6F708192A3B4C5D6E7F")


def references():
    """Five pages; everything below points at page 3 unless it says otherwise."""
    pdf = Pdf()
    f = font(pdf)
    root = pdf.reserve()
    p = [None] + [pdf.reserve() for _ in range(5)]
    catalog, outlines, o1, o2, o3 = (pdf.reserve() for _ in range(5))
    link1 = pdf.add(f"<< /Type /Annot /Subtype /Link /Rect [72 600 300 630] /Dest [{p[3]} 0 R /Fit] >>")
    goto = pdf.add(f"<< /S /GoTo /D [{p[3]} 0 R /XYZ 0 792 0] >>")
    link2 = pdf.add(f"<< /Type /Annot /Subtype /Link /Rect [72 600 300 630] /A {goto} 0 R >>")
    link4 = pdf.add("<< /Type /Annot /Subtype /Link /Rect [72 600 300 630] /Dest (chap3) >>")
    link5 = pdf.add(f"<< /Type /Annot /Subtype /Link /Rect [72 600 300 630] /Dest [{p[2]} 0 R /Fit] >>")
    # A merged field and widget on page 3 (/P), and a field whose kid widget has no /P but is
    # listed in page 3's /Annots; a third field's widget is on page 4.
    merged = pdf.add(f"<< /Type /Annot /Subtype /Widget /FT /Tx /T (name) /Rect [72 500 300 520] /P {p[3]} 0 R >>")
    parent_field = pdf.reserve()
    kid_widget = pdf.add(f"<< /Type /Annot /Subtype /Widget /Parent {parent_field} 0 R /Rect [72 450 300 470] >>")
    pdf.set(parent_field, f"<< /FT /Tx /T (city) /Kids [{kid_widget} 0 R] >>")
    other = pdf.add(f"<< /Type /Annot /Subtype /Widget /FT /Tx /T (zip) /Rect [72 500 300 520] /P {p[4]} 0 R >>")
    annots = {1: [link1], 2: [link2], 3: [merged, kid_widget], 4: [link4, other], 5: [link5]}
    struct_root, doc_elem, para3, para2 = (pdf.reserve() for _ in range(4))
    for n in range(1, 6):
        mcid = {3: b"\n/P <</MCID 0>> BDC EMC", 2: b"\n/P <</MCID 0>> BDC EMC"}.get(n, b"")
        sp = {3: " /StructParents 0", 2: " /StructParents 1"}.get(n, "")
        pdf.set(p[n], page(root, marker(pdf, n, mcid), f" /Annots {kids(annots[n])}{sp}"))
    pdf.set(root, f"<< /Type /Pages /Kids {kids(p[1:])} /Count 5 /MediaBox [0 0 612 792] /Resources {resources(f)} >>")
    pdf.set(para3, f"<< /Type /StructElem /S /P /P {doc_elem} 0 R /Pg {p[3]} 0 R /K 0 >>")
    pdf.set(para2, f"<< /Type /StructElem /S /P /P {doc_elem} 0 R /Pg {p[2]} 0 R /K 0 >>")
    pdf.set(doc_elem, f"<< /Type /StructElem /S /Document /P {struct_root} 0 R /K [{para2} 0 R {para3} 0 R] >>")
    pdf.set(struct_root, f"<< /Type /StructTreeRoot /K {doc_elem} 0 R"
            f" /ParentTree << /Nums [0 [{para3} 0 R] 1 [{para2} 0 R]] >> /ParentTreeNextKey 2 >>")
    pdf.set(o1, f"<< /Title (Dest to page 3) /Parent {outlines} 0 R /Next {o2} 0 R /Dest [{p[3]} 0 R /Fit] >>")
    pdf.set(o2, f"<< /Title (GoTo page 3) /Parent {outlines} 0 R /Prev {o1} 0 R /Next {o3} 0 R"
            f" /A << /S /GoTo /D [{p[3]} 0 R /FitH 700] >> >>")
    pdf.set(o3, f"<< /Title (Page 2) /Parent {outlines} 0 R /Prev {o2} 0 R /Dest [{p[2]} 0 R /Fit] >>")
    pdf.set(outlines, f"<< /Type /Outlines /First {o1} 0 R /Last {o3} 0 R /Count 3 >>")
    names = pdf.add(f"<< /Names [(chap3) [{p[3]} 0 R /Fit] (chap4) [{p[4]} 0 R /Fit]] >>")
    pdf.set(catalog, f"<< /Type /Catalog /Pages {root} 0 R /Outlines {outlines} 0 R"
            f" /Names << /Dests {names} 0 R >> /Dests << /old3 [{p[3]} 0 R /FitH 700] /old5 [{p[5]} 0 R /Fit] >>"
            f" /OpenAction [{p[3]} 0 R /Fit] /MarkInfo << /Marked true >> /StructTreeRoot {struct_root} 0 R"
            f" /AcroForm << /Fields [{merged} 0 R {parent_field} 0 R {other} 0 R] >> >>")
    pdf.write("references.pdf", catalog, "2A42C3F4D5E6F708192A3B4C5D6E7F80")


def labels():
    """Ten pages: i-iii, 1-4, A-1 and A-2, then "Back cover" (prefix only)."""
    pdf = Pdf()
    root, _ = flat(pdf, 10)
    cat = pdf.add(f"<< /Type /Catalog /Pages {root} 0 R /PageLabels << /Nums [0 << /S /r >> 3 << /S /D >>"
                  " 7 << /S /D /P (A-) >> 9 << /P (Back cover) >>] >> >>")
    pdf.write("labels.pdf", cat, "3B53D4A5E6F708192A3B4C5D6E7F8091")


def boxes():
    """Four pages, one per rotation, each with all five boxes; page 2 has /UserUnit 2."""
    pdf = Pdf()
    f = font(pdf)
    root = pdf.reserve()
    pages = []
    for i, rotate in enumerate((0, 90, 180, 270)):
        unit = " /UserUnit 2" if i == 1 else ""
        pages.append(pdf.add(page(root, marker(pdf, i + 1),
                                  f" /MediaBox [0 0 612 792] /CropBox [18 18 594 774] /BleedBox [27 27 585 765]"
                                  f" /TrimBox [36 36 576 756] /ArtBox [54 54 558 738] /Rotate {rotate}{unit}")))
    pdf.set(root, f"<< /Type /Pages /Kids {kids(pages)} /Count 4 /Resources {resources(f)} >>")
    cat = pdf.add(f"<< /Type /Catalog /Pages {root} 0 R >>")
    pdf.write("boxes.pdf", cat, "4C64E5B6F708192A3B4C5D6E7F8091A2")


def forms():
    """Two pages: a merged text field and widget (page 1), a checkbox field with kid widgets on
    pages 1 and 2, and an unsigned signature field (page 2). The names "name" and "agree" clash
    with a second copy of the file."""
    pdf = Pdf()
    f = font(pdf)
    root = pdf.reserve()
    p1, p2 = pdf.reserve(), pdf.reserve()
    on = pdf.stream(b"0 0 1 rg 0 0 12 12 re f", "/Type /XObject /Subtype /Form /BBox [0 0 12 12] ")
    text = pdf.add(f"<< /Type /Annot /Subtype /Widget /FT /Tx /T (name) /V (Ada) /Rect [72 600 300 620] /P {p1} 0 R"
                   " /DA (/Helv 12 Tf 0 g) >>")
    agree = pdf.reserve()
    box1 = pdf.add(f"<< /Type /Annot /Subtype /Widget /Parent {agree} 0 R /Rect [72 550 84 562] /P {p1} 0 R"
                   f" /AS /Off /AP << /N << /Yes {on} 0 R >> >> >>")
    box2 = pdf.add(f"<< /Type /Annot /Subtype /Widget /Parent {agree} 0 R /Rect [72 550 84 562] /P {p2} 0 R"
                   f" /AS /Off /AP << /N << /Yes {on} 0 R >> >> >>")
    pdf.set(agree, f"<< /FT /Btn /T (agree) /V /Off /Kids [{box1} 0 R {box2} 0 R] >>")
    sig = pdf.add(f"<< /Type /Annot /Subtype /Widget /FT /Sig /T (signature) /Rect [72 400 300 450] /P {p2} 0 R >>")
    pdf.set(p1, page(root, marker(pdf, 1), f" /Annots {kids([text, box1])}"))
    pdf.set(p2, page(root, marker(pdf, 2), f" /Annots {kids([box2, sig])}"))
    helv = pdf.add("<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>")
    pdf.set(root, f"<< /Type /Pages /Kids {kids([p1, p2])} /Count 2 /MediaBox [0 0 612 792] /Resources {resources(f)} >>")
    cat = pdf.add(f"<< /Type /Catalog /Pages {root} 0 R /AcroForm << /Fields [{text} 0 R {agree} 0 R {sig} 0 R]"
                  f" /DR << /Font << /Helv {helv} 0 R >> >> /DA (/Helv 0 Tf 0 g) >> >>")
    pdf.write("forms.pdf", cat, "5D75F6C708192A3B4C5D6E7F8091A2B3")


def layers():
    """Two pages; each draws a square in layer "Draft" (on) and one in "Notes" (off)."""
    pdf = Pdf()
    f = font(pdf)
    root = pdf.reserve()
    draft = pdf.add("<< /Type /OCG /Name (Draft) >>")
    notes = pdf.add("<< /Type /OCG /Name (Notes) >>")
    res = f"<< /Font << /F1 {f} 0 R >> /Properties << /OC1 {draft} 0 R /OC2 {notes} 0 R >> >>"
    layered = b"\n/OC /OC1 BDC 1 0 0 rg 72 400 100 100 re f EMC\n/OC /OC2 BDC 0 0 1 rg 300 400 100 100 re f EMC"
    pages = [pdf.add(page(root, marker(pdf, n, layered))) for n in (1, 2)]
    pdf.set(root, f"<< /Type /Pages /Kids {kids(pages)} /Count 2 /MediaBox [0 0 612 792] /Resources {res} >>")
    cat = pdf.add(f"<< /Type /Catalog /Pages {root} 0 R /OCProperties << /OCGs [{draft} 0 R {notes} 0 R]"
                  f" /D << /Order [{draft} 0 R {notes} 0 R] /ON [{draft} 0 R] /OFF [{notes} 0 R] >> >> >>")
    pdf.write("layers.pdf", cat, "6E8607D8192A3B4C5D6E7F8091A2B3C4")


def chapters():
    """Twelve pages in three chapters (pages 1, 5 and 9), each with two sections; chapter 2's
    bookmark goes through the named destination "chapter-2"."""
    pdf = Pdf()
    root, pages = flat(pdf, 12)
    outlines = pdf.reserve()
    items = []
    for c, first in enumerate((0, 4, 8)):
        item = pdf.reserve()
        subs = [pdf.reserve(), pdf.reserve()]
        items.append((item, subs, c, first))
    for i, (item, subs, c, first) in enumerate(items):
        prev = f" /Prev {items[i - 1][0]} 0 R" if i > 0 else ""
        nxt = f" /Next {items[i + 1][0]} 0 R" if i < 2 else ""
        dest = "/Dest (chapter-2)" if c == 1 else f"/Dest [{pages[first]} 0 R /Fit]"
        pdf.set(item, f"<< /Title (Chapter {c + 1}) /Parent {outlines} 0 R{prev}{nxt} {dest}"
                f" /First {subs[0]} 0 R /Last {subs[1]} 0 R /Count 2 >>")
        for s, sub in enumerate(subs):
            link = f" /Next {subs[1]} 0 R" if s == 0 else f" /Prev {subs[0]} 0 R"
            pdf.set(sub, f"<< /Title (Section {c + 1}.{s + 1}) /Parent {item} 0 R{link}"
                    f" /Dest [{pages[first + 1 + s]} 0 R /XYZ 72 720 0] >>")
    pdf.set(outlines, f"<< /Type /Outlines /First {items[0][0]} 0 R /Last {items[2][0]} 0 R /Count 9 >>")
    names = pdf.add(f"<< /Names [(chapter-2) [{pages[4]} 0 R /Fit]] >>")
    cat = pdf.add(f"<< /Type /Catalog /Pages {root} 0 R /Outlines {outlines} 0 R /Names << /Dests {names} 0 R >>"
                  " /PageMode /UseOutlines >>")
    pdf.write("chapters.pdf", cat, "7F9718E92A3B4C5D6E7F8091A2B3C4D5")


def main():
    inherited()
    unbalanced()
    references()
    labels()
    boxes()
    forms()
    layers()
    chapters()


if __name__ == "__main__":
    main()
