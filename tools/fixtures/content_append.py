#!/usr/bin/env python3
"""Writes fixtures/content/append/*.pdf (no dependencies): small pages whose content leaves
q/Q unbalanced, for laminapdf-content's append API (todo E2e). Each page is 200 x 200 points
and draws something in colour, so a render before and after wrapping shows any state a wrap
would change.

    balanced.pdf      q ... Q around a blue square: nothing to fix
    open-q.pdf     two q left open after a scale and a red square
    extra-Q.pdf     q 2 0 0 2 0 0 cm Q Q 0 0 1 rg ...: the second Q is unmatched
    stray-Q.pdf       2 0 0 2 0 0 cm Q 0 0 10 10 re f: a Q at depth 0 after a state change
    split.pdf         an unmatched Q and an open q in each of two /Contents streams
    inline-image.pdf  an inline image whose data is the bytes "QQ", then an unmatched Q
    shared.pdf        two pages sharing one content stream (with an unmatched Q) and inheriting
                      resources that already use the names Fm1 and Gs1
    in-text.pdf       content that ends inside a text object (BT without ET)
    unterminated.pdf  content that ends inside a string: a wrap must refuse it

    python3 tools/fixtures/content_append.py
"""

from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "content" / "append"

MEDIA = "/MediaBox [0 0 200 200]"
HELVETICA = "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>"


def write(name, objects, streams):
    """objects: {num: body}; streams: {num: bytes or (dictionary entries, bytes)}. Object 1 is
    the catalog."""
    out = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
    offsets = {}
    size = max(list(objects) + list(streams)) + 1
    for num in range(1, size):
        if num in streams:
            offsets[num] = len(out)
            extra, data = streams[num] if isinstance(streams[num], tuple) else ("", streams[num])
            out += f"{num} 0 obj\n<< {extra}/Length {len(data)} >>\nstream\n".encode() + data + b"\nendstream\nendobj\n"
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


def one_page(name, contents, resources="<< >>"):
    """A page whose /Contents are `contents` (one stream, or a list of streams)."""
    if isinstance(contents, bytes):
        contents = [contents]
    nums = list(range(10, 10 + len(contents)))
    refs = " ".join(f"{n} 0 R" for n in nums)
    objects = {
        1: "<< /Type /Catalog /Pages 2 0 R >>",
        2: "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        3: f"<< /Type /Page /Parent 2 0 R {MEDIA} /Contents [{refs}] /Resources {resources} >>",
        4: HELVETICA,
    }
    write(name, objects, dict(zip(nums, contents)))


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    one_page("balanced.pdf", b"q 0 0 1 rg 20 20 60 60 re f Q")
    one_page("open-q.pdf", b"q q 2 0 0 2 0 0 cm 1 0 0 rg 10 10 30 30 re f")
    one_page("extra-Q.pdf", b"q 2 0 0 2 0 0 cm 0 1 0 rg 5 5 10 10 re f Q Q 0 0 1 rg 10 10 30 30 re f")
    one_page("stray-Q.pdf", b"2 0 0 2 0 0 cm Q 0 0 10 10 re f")
    one_page(
        "split.pdf",
        [
            b"2 0 0 2 0 0 cm Q q 1 0 0 rg 5 5 20 20 re f",
            b"Q Q 0 0 1 rg 50 50 20 20 re f q 0.5 g 0 0 1 0 0 cm",
        ],
    )
    one_page(
        "inline-image.pdf",
        b"q 40 0 0 40 20 20 cm BI /W 2 /H 1 /BPC 8 /CS /G ID QQ EI Q Q 0 1 0 rg 100 100 30 30 re f",
    )
    one_page("in-text.pdf", b"0 0 1 rg BT /F1 24 Tf 20 100 Td (Hi) Tj", "<< /Font << /F1 4 0 R >> >>")
    one_page("unterminated.pdf", b"0 0 1 rg 10 10 50 50 re f (oops")

    # Two pages share stream 10 and inherit resources from the page tree.
    write(
        "shared.pdf",
        {
            1: "<< /Type /Catalog /Pages 2 0 R >>",
            2: (
                f"<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 {MEDIA}"
                " /Resources << /XObject << /Fm1 5 0 R >> /ExtGState << /Gs1 6 0 R >> >> >>"
            ),
            3: "<< /Type /Page /Parent 2 0 R /Contents 10 0 R >>",
            4: "<< /Type /Page /Parent 2 0 R /Contents 10 0 R >>",
            6: "<< /Type /ExtGState /ca 0.5 >>",
        },
        {
            5: ("/Type /XObject /Subtype /Form /BBox [0 0 20 20] ", b"1 0 0 rg 0 0 20 20 re f"),
            10: b"2 0 0 2 0 0 cm Q /Gs1 gs /Fm1 Do",
        },
    )


if __name__ == "__main__":
    main()
