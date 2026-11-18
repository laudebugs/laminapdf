#!/usr/bin/env python3
"""Writes fixtures/forms/*.pdf (no dependencies): interactive forms for laminapdf-model's form
view and laminapdf-ops' SetFieldValue (todo E8c).

    fields.pdf  two US Letter pages with every field type and flag the view reads:
                  name       text, required, /MaxLen 40, /TU, merged with its widget (page 1)
                  address    multi-line text, /Q 1 (page 1)
                  agree      a check box, on (/Yes) (page 1)
                  colour     radio buttons with RadiosInUnison: two of three widgets share the
                             on-state /red, which is the value (page 1)
                  size       radio buttons with /Opt: on-states /0 and /1 export Small and Large;
                             the value is /1 (page 1)
                  country    an editable combo box with /Opt [(UK) (US) [(FR) (France)]], value US
                  toppings   a multi-select list box, Cheese and Olives chosen, /I [0 2]
                  submit     a push button
                  signature  an unsigned signature field
                  person     a non-terminal field whose /FT, /Ff (read-only) and /DA its kids
                             person.first and person.last inherit; person.last's /Parent names
                             another field, a mismatch the view reports
                  total      two top-level fields with one name, which share a value (page 2)
                  notes      text whose value is a text stream, with /RV rich text (page 2)
                  hidden     a text field whose widget no page lists
                  untyped    a field without /FT (page 2)
                and on page 2 a widget whose parent, `orphan`, the form's /Fields doesn't list.
    tabs.pdf    six pages of four text widgets in a 2 x 2 grid, listed in /Annots as bottom
                right, top left, bottom left, top right, with /Tabs /R, /C, /S, /A, /W and none.
                The structure tree reaches page 3's widgets as top right, bottom left, top left,
                bottom right.

    python3 tools/fixtures/forms.py
"""

from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "forms"

LETTER = "/MediaBox [0 0 612 792]"
DA = "(/Helv 0 Tf 0 g)"
DR = "<< /Font << /Helv 6 0 R >> >>"
HELVETICA = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>"


def write(name, objects, streams, version="1.7"):
    """objects: {num: body}; streams: {num: (dictionary entries, bytes)}. Object 1 is the
    catalog."""
    out = bytearray(f"%PDF-{version}\n%\xe2\xe3\xcf\xd3\n".encode("latin-1"))
    offsets = {}
    size = max(list(objects) + list(streams)) + 1
    for num in range(1, size):
        if num in streams:
            offsets[num] = len(out)
            extra, data = streams[num]
            out += f"{num} 0 obj\n<< {extra} /Length {len(data)} >>\nstream\n".encode() + data + b"\nendstream\nendobj\n"
        elif num in objects:
            offsets[num] = len(out)
            out += f"{num} 0 obj\n{objects[num]}\nendobj\n".encode("latin-1")
    xref = len(out)
    out += f"xref\n0 {size}\n0000000000 65535 f\r\n".encode()
    for num in range(1, size):
        if num in offsets:
            out += f"{offsets[num]:010} 00000 n\r\n".encode()
        else:
            out += b"0000000000 65535 f\r\n"
    out += f"trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n".encode()
    (OUT / name).write_bytes(bytes(out))


def rect(x, y, w=150, h=20):
    return f"/Rect [{x} {y} {x + w} {y + h}]"


def widget(page, x, y, extra="", w=150, h=20):
    """The keys of a widget annotation on `page` at (x, y)."""
    return f"/Type /Annot /Subtype /Widget {rect(x, y, w, h)} /P {page} 0 R /F 4 {extra}"


def button_ap(on, on_ref, off_ref):
    return f"/AP << /N << /{on} {on_ref} 0 R /Off {off_ref} 0 R >> >>"


def fields():
    # Appearance streams: 60 a check mark, 61 empty (Off).
    check = (b"q 0 g BT /ZaDb 12 Tf 2 4 Td (4) Tj ET Q", "/Type /XObject /Subtype /Form /BBox [0 0 20 20] "
             "/Resources << /Font << /ZaDb 7 0 R >> >>")
    off = (b"", "/Type /XObject /Subtype /Form /BBox [0 0 20 20]")
    streams = {
        60: (check[1], check[0]),
        61: (off[1], off[0]),
        62: ("", "Remember the milk".encode("latin-1")),
    }
    objects = {
        1: "<< /Type /Catalog /Pages 2 0 R /AcroForm 5 0 R >>",
        2: "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>",
        3: f"<< /Type /Page /Parent 2 0 R {LETTER} /Resources << >> "
        "/Annots [10 0 R 11 0 R 12 0 R 14 0 R 15 0 R 16 0 R 18 0 R 19 0 R 20 0 R 21 0 R 22 0 R 23 0 R "
        "25 0 R 26 0 R] >>",
        4: f"<< /Type /Page /Parent 2 0 R {LETTER} /Resources << >> "
        "/Annots [30 0 R 31 0 R 32 0 R 34 0 R 36 0 R] >>",
        5: f"<< /Fields [10 0 R 11 0 R 12 0 R 13 0 R 17 0 R 20 0 R 21 0 R 22 0 R 23 0 R 24 0 R "
        f"30 0 R 31 0 R 32 0 R 33 0 R 36 0 R] /DA {DA} /DR {DR} >>",
        6: HELVETICA,
        7: "<< /Type /Font /Subtype /Type1 /BaseFont /ZapfDingbats >>",
        # Page 1.
        10: f"<< /FT /Tx /T (name) /TU (Your name) /Ff 2 /MaxLen 40 /V (Ada Lovelace) "
        f"{widget(3, 50, 700)} >>",
        11: f"<< /FT /Tx /T (address) /Ff 4096 /Q 1 /V (12 Main Street) {widget(3, 50, 640, h=50)} >>",
        12: f"<< /FT /Btn /T (agree) /V /Yes /AS /Yes {button_ap('Yes', 60, 61)} "
        f"{widget(3, 50, 600, w=20)} >>",
        # colour: Radio (1 << 15) | RadiosInUnison (1 << 25).
        13: "<< /FT /Btn /T (colour) /Ff 33587200 /V /red /Kids [14 0 R 15 0 R 16 0 R] >>",
        14: f"<< /Parent 13 0 R /AS /red {button_ap('red', 60, 61)} {widget(3, 50, 560, w=20)} >>",
        15: f"<< /Parent 13 0 R /AS /Off {button_ap('green', 60, 61)} {widget(3, 80, 560, w=20)} >>",
        16: f"<< /Parent 13 0 R /AS /red {button_ap('red', 60, 61)} {widget(3, 110, 560, w=20)} >>",
        # size: Radio with /Opt.
        17: "<< /FT /Btn /T (size) /Ff 32768 /V /1 /Opt [(Small) (Large)] /Kids [18 0 R 19 0 R] >>",
        18: f"<< /Parent 17 0 R /AS /Off {button_ap('0', 60, 61)} {widget(3, 50, 520, w=20)} >>",
        19: f"<< /Parent 17 0 R /AS /1 {button_ap('1', 60, 61)} {widget(3, 80, 520, w=20)} >>",
        # country: Combo (1 << 17) | Edit (1 << 18).
        20: f"<< /FT /Ch /T (country) /Ff 393216 /Opt [(UK) (US) [(FR) (France)]] /V (US) "
        f"{widget(3, 50, 480)} >>",
        # toppings: MultiSelect (1 << 21).
        21: f"<< /FT /Ch /T (toppings) /Ff 2097152 /Opt [(Cheese) (Ham) (Olives)] "
        f"/V [(Cheese) (Olives)] /I [0 2] {widget(3, 50, 400, h=60)} >>",
        # submit: Pushbutton (1 << 16).
        22: f"<< /FT /Btn /T (submit) /Ff 65536 {widget(3, 50, 360, w=80)} >>",
        23: f"<< /FT /Sig /T (signature) {widget(3, 50, 300, w=200, h=40)} >>",
        # person: read-only (1) and /DA inherited by its kids.
        24: "<< /FT /Tx /T (person) /Ff 1 /DA (/Helv 10 Tf 0 g) /Kids [25 0 R 26 0 R] >>",
        25: f"<< /Parent 24 0 R /T (first) /V (Grace) {widget(3, 50, 260)} >>",
        26: f"<< /Parent 10 0 R /T (last) /V (Hopper) {widget(3, 220, 260)} >>",
        # Page 2.
        30: f"<< /FT /Tx /T (total) /V (42) {widget(4, 50, 700)} >>",
        31: f"<< /FT /Tx /T (total) /V (42) {widget(4, 220, 700)} >>",
        32: f"<< /FT /Tx /T (notes) /Ff 4096 /V 62 0 R /RV (<p>Remember the <b>milk</b></p>) "
        f"{widget(4, 50, 600, h=60)} >>",
        33: f"<< /FT /Tx /T (hidden) /V (not shown) {widget(4, 50, 500)} >>",
        34: f"<< /Parent 35 0 R {widget(4, 50, 450)} >>",
        35: "<< /FT /Tx /T (orphan) /Kids [34 0 R] >>",
        36: f"<< /T (untyped) {widget(4, 50, 400)} >>",
    }
    write("fields.pdf", objects, streams)


def tabs():
    order = ["br", "tl", "bl", "tr"]
    place = {"tl": (50, 600), "tr": (300, 600), "bl": (50, 400), "br": (300, 400)}
    values = ["/R", "/C", "/S", "/A", "/W", None]
    objects = {}
    page_refs = []
    field_refs = []
    struct_kids = []
    num = 10
    for p, value in enumerate(values, start=1):
        page = num
        num += 1
        page_refs.append(page)
        annots = []
        by_corner = {}
        for corner in order:
            x, y = place[corner]
            objects[num] = (f"<< /FT /Tx /T (p{p}-{corner}) /V ({corner}) {widget(page, x, y)} "
                            f"/StructParent {p * 10 + len(annots)} >>")
            by_corner[corner] = num
            annots.append(num)
            field_refs.append(num)
            num += 1
        tabs_entry = f"/Tabs {value}" if value else ""
        objects[page] = (f"<< /Type /Page /Parent 2 0 R {LETTER} /Resources << >> {tabs_entry} "
                         f"/Annots [{' '.join(f'{a} 0 R' for a in annots)}] >>")
        if value == "/S":
            for corner in ["tr", "bl", "tl", "br"]:
                elem = num
                num += 1
                objects[elem] = (f"<< /Type /StructElem /S /Form /P 9 0 R /Pg {page} 0 R "
                                 f"/K << /Type /OBJR /Obj {by_corner[corner]} 0 R /Pg {page} 0 R >> >>")
                struct_kids.append(elem)
    objects[1] = ("<< /Type /Catalog /Pages 2 0 R /AcroForm 5 0 R /StructTreeRoot 8 0 R "
                  "/MarkInfo << /Marked true >> >>")
    objects[2] = (f"<< /Type /Pages /Kids [{' '.join(f'{r} 0 R' for r in page_refs)}] "
                  f"/Count {len(page_refs)} >>")
    objects[5] = f"<< /Fields [{' '.join(f'{r} 0 R' for r in field_refs)}] /DA {DA} /DR {DR} >>"
    objects[6] = HELVETICA
    objects[8] = "<< /Type /StructTreeRoot /K 9 0 R >>"
    objects[9] = f"<< /Type /StructElem /S /Document /P 8 0 R /K [{' '.join(f'{k} 0 R' for k in struct_kids)}] >>"
    write("tabs.pdf", objects, {}, version="2.0")


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    fields()
    tabs()


if __name__ == "__main__":
    main()
