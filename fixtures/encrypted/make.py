#!/usr/bin/env python3
"""Generate lamina's encrypted fixtures (todo B2b) from one plaintext source.

Usage (from the repository root):

    python3 -m venv .venv && .venv/bin/pip install pypdf==6.21.3 cryptography==50.0.2
    .venv/bin/python fixtures/encrypted/make.py      # needs qpdf >= 11.7 on PATH
    .venv/bin/python fixtures/encrypted/facts.py     # rewrites fixtures/encrypted/expected.json

Writes, in fixtures/encrypted/:

* source.pdf: the plaintext original (Info strings, an outline with a UTF-16 title, an XMP
  metadata stream, two pages of Flate text, a text annotation), written by this script;
* one file per standard security handler revision and cipher, encrypted by qpdf, each in two
  variants: `-empty` (empty user password, owner password only) and `-user` (user and owner
  passwords, some permissions cleared);
* r6-aes-256-same.pdf (qpdf): the same user and owner password, which Algorithm 2.A reports as
  the owner's;
* written by this script, since qpdf has no flag for them (V4/R4, deterministic IVs):
  identity-stmf.pdf and crypt-filter-identity.pdf (AES-128 strings with `/StmF /Identity` plus
  one stream selecting `/StdCF` through a `/Crypt` filter, and the reverse);
  v4-aes-128-cf-length.pdf and v4-rc4-40-cf-length.pdf (the key length only in
  `/CF/StdCF/Length`, which qpdf 12 ignores); objstm-catalog-aes-128.pdf (the catalog inside an
  encrypted object stream, with an xref stream); objstm-bad-padding-aes-128.pdf (the same with
  invalid AES padding on the object stream: damaged on purpose, so it has no facts);
* passwords.json: every file's passwords (read by facts.py).

It also copies the encrypted files to fuzz/seeds/open/encrypted/ and their passwords, one per
line, to fuzz/seeds/open/passwords.txt (the B2c `open` fuzz target tries each).

Each qpdf output's /V and /R is checked with `qpdf --show-encryption`; a case that comes out at the
wrong revision fails the script. The files are not byte-reproducible (qpdf and AES use random
IVs and R6 salts), but every fact in expected.json is.
"""

from __future__ import annotations

import hashlib
import json
import re
import shutil
import subprocess
import sys
import zlib
from pathlib import Path

from cryptography.hazmat.primitives.ciphers import Cipher, algorithms, modes
import pypdf
from pypdf._crypt_providers import rc4_encrypt
from pypdf._encryption import AlgV4

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
SEEDS = ROOT / "fuzz" / "seeds" / "open"

USER = "user-pw"
OWNER = "owner-pw"
# R5/R6 passwords are UTF-8 (SASLprep); this one is already in NFC with nothing to map.
USER_UNICODE = "Pässwört-6"
SAME = "same-pw"
# Hand-written files whose V4 key length is only in /CF/StdCF/Length, which qpdf 12 ignores.
QPDF_CANNOT_READ = frozenset({"v4-aes-128-cf-length.pdf", "v4-rc4-40-cf-length.pdf"})

QPDF_MIN = (11, 7)


# --- a minimal PDF object model and writer -------------------------------------------------


class Name(str):
    pass


class Ref(int):
    pass


class Str(bytes):
    """A string; `hex=True` writes it as <...>."""

    hex = False


class HexStr(Str):
    hex = True


class Stream:
    def __init__(self, d: dict, data: bytes):
        self.dict = d
        self.data = data


def ser(obj, enc, num: int) -> bytes:
    if isinstance(obj, bool):
        return b"true" if obj else b"false"
    if isinstance(obj, Ref):
        return b"%d 0 R" % obj
    if isinstance(obj, int):
        return b"%d" % obj
    if isinstance(obj, Name):
        return b"/" + obj.encode()
    if isinstance(obj, Str):
        data = enc.string(num, bytes(obj)) if enc else bytes(obj)
        return b"<" + data.hex().encode() + b">" if obj.hex or enc else b"(" + escape(data) + b")"
    if isinstance(obj, list):
        return b"[" + b" ".join(ser(o, enc, num) for o in obj) + b"]"
    if isinstance(obj, dict):
        return b"<<" + b"".join(b" /" + k.encode() + b" " + ser(v, enc, num) for k, v in obj.items()) + b" >>"
    if obj is None:
        return b"null"
    raise TypeError(obj)


def escape(data: bytes) -> bytes:
    return data.replace(b"\\", b"\\\\").replace(b"(", b"\\(").replace(b")", b"\\)")


def write_pdf(objects: dict[int, object], trailer: dict, enc=None) -> bytes:
    out = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
    offsets = {}
    for num in sorted(objects):
        obj = objects[num]
        offsets[num] = len(out)
        out += b"%d 0 obj\n" % num
        if isinstance(obj, Stream):
            data = obj.data
            if enc and num != enc.encrypt_num:
                data = enc.stream(num, obj)
            d = dict(obj.dict, Length=len(data))
            out += ser(d, enc if num != getattr(enc, "encrypt_num", None) else None, num)
            out += b"\nstream\n" + data + b"\nendstream"
        else:
            out += ser(obj, enc if enc and num != enc.encrypt_num else None, num)
        out += b"\nendobj\n"
    size = max(objects) + 1
    xref = len(out)
    out += b"xref\n0 %d\n0000000000 65535 f\r\n" % size
    for num in range(1, size):
        out += b"%010d 00000 n\r\n" % offsets[num]
    out += b"trailer\n" + ser(dict(trailer, Size=size), None, 0) + b"\nstartxref\n%d\n%%%%EOF\n" % xref
    return bytes(out)


def utf16(text: str) -> Str:
    return HexStr(b"\xfe\xff" + text.encode("utf-16-be"))


XMP = b"""<?xpacket begin="\xef\xbb\xbf" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
<rdf:Description rdf:about="" xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title><rdf:Alt>
<rdf:li xml:lang="x-default">Secret document</rdf:li></rdf:Alt></dc:title></rdf:Description>
</rdf:RDF></x:xmpmeta>
<?xpacket end="r"?>"""


def page_content(n: int) -> bytes:
    return (
        b"BT /F1 24 Tf 72 720 Td (Encrypted page %d) Tj ET\n"
        b"BT /F1 12 Tf 72 690 Td (The quick brown fox jumps over the lazy dog.) Tj ET\n" % n
    )


def source_objects() -> tuple[dict[int, object], dict]:
    objs: dict[int, object] = {
        1: {"Type": Name("Catalog"), "Pages": Ref(2), "Outlines": Ref(10), "Metadata": Ref(9),
            "PageMode": Name("UseOutlines"), "Lang": Str(b"en-GB")},
        2: {"Type": Name("Pages"), "Kids": [Ref(3), Ref(4)], "Count": 2, "MediaBox": [0, 0, 612, 792]},
        3: {"Type": Name("Page"), "Parent": Ref(2), "Contents": Ref(5), "Annots": [Ref(14)],
            "Resources": {"Font": {"F1": Ref(7)}}},
        4: {"Type": Name("Page"), "Parent": Ref(2), "Contents": Ref(6), "Resources": {"Font": {"F1": Ref(7)}}},
        5: Stream({"Filter": Name("FlateDecode")}, zlib.compress(page_content(1), 9)),
        6: Stream({"Filter": Name("FlateDecode")}, zlib.compress(page_content(2), 9)),
        7: {"Type": Name("Font"), "Subtype": Name("Type1"), "BaseFont": Name("Helvetica")},
        8: {"Title": Str(b"Secret document"), "Author": Str(b"lamina"), "Subject": utf16("B2b fixtures — Café"),
            "Producer": Str(b"fixtures/encrypted/make.py")},
        9: Stream({"Type": Name("Metadata"), "Subtype": Name("XML")}, XMP),
        10: {"Type": Name("Outlines"), "First": Ref(11), "Last": Ref(12), "Count": 3},
        11: {"Title": Str(b"Chapter 1"), "Parent": Ref(10), "Next": Ref(12), "First": Ref(13), "Last": Ref(13),
             "Count": 1, "Dest": [Ref(3), Name("Fit")]},
        12: {"Title": utf16("Chapitre 2 — Café"), "Parent": Ref(10), "Prev": Ref(11), "Dest": [Ref(4), Name("Fit")]},
        13: {"Title": Str(b"Section 1.1 (nested)"), "Parent": Ref(11), "Dest": [Ref(3), Name("Fit")]},
        14: {"Type": Name("Annot"), "Subtype": Name("Text"), "Rect": [72, 600, 92, 620],
             "Contents": Str(b"An annotation note")},
    }
    file_id = HexStr(hashlib.md5(b"lamina B2b encrypted fixtures").digest())
    trailer = {"Root": Ref(1), "Info": Ref(8), "ID": [file_id, file_id]}
    return objs, trailer


# --- hand-written V4/R4 encryption for the cases qpdf can't write ---------------------------


class V4Writer:
    """Encrypts with one crypt filter /StdCF (`cfm` AESV2 or V2): strings always, streams per
    `stm_f` (`StdCF` or `Identity`), and `crypt_stream` through an explicit `/Crypt` filter
    selecting `crypt_name`. `top_length` / `cf_length` set the top-level and the crypt
    filter's `/Length` (omitted when None), `key_bits` the real file key length.
    `bad_padding` lists objects whose AES padding is written wrong (damaged fixtures).

    O, U and the file key come from pypdf's Algorithms 2, 3 and 5, so the file key is checked
    independently when facts.py opens the result with pypdf and qpdf.
    """

    def __init__(self, id0: bytes, *, cfm: str = "AESV2", key_bits: int = 128, top_length: int | None = 128,
                 cf_length: int | None = 16, stm_f: str = "StdCF", crypt_stream: int | None = None,
                 crypt_name: str = "StdCF", bad_padding: frozenset[int] = frozenset()):
        self.encrypt_num = 15
        self.p = -3904 & 0xFFFFFFFF  # print, copy, accessibility, form fill; reserved bits set
        okey = AlgV4.compute_O_value_key(OWNER.encode(), 4, key_bits)
        self.o = AlgV4.compute_O_value(okey, USER.encode(), 4)
        self.key = AlgV4.compute_key(USER.encode(), 4, key_bits, self.o, self.p, id0, True)
        self.u = AlgV4.compute_U_value(self.key, 4, id0)
        self.cfm, self.top_length, self.cf_length = cfm, top_length, cf_length
        self.stm_f, self.crypt_stream, self.crypt_name = stm_f, crypt_stream, crypt_name
        self.bad_padding = bad_padding

    def dict(self) -> dict:
        cf = {"Type": Name("CryptFilter"), "CFM": Name(self.cfm), "AuthEvent": Name("DocOpen")}
        if self.cf_length is not None:
            cf["Length"] = self.cf_length
        d = {"Filter": Name("Standard"), "V": 4, "R": 4}
        if self.top_length is not None:
            d["Length"] = self.top_length
        d.update({"CF": {"StdCF": cf}, "StmF": Name(self.stm_f), "StrF": Name("StdCF"), "O": HexStr(self.o),
                  "U": HexStr(self.u), "P": self.p - (1 << 32)})
        return d

    def _encrypt(self, num: int, data: bytes) -> bytes:
        aes = self.cfm == "AESV2"
        h = hashlib.md5(self.key + num.to_bytes(3, "little") + b"\0\0" + (b"sAlT" if aes else b"")).digest()
        if not aes:
            return rc4_encrypt(h[: min(len(self.key) + 5, 16)], data)
        iv = hashlib.md5(b"iv" + num.to_bytes(4, "little") + data).digest()  # deterministic, test only
        pad = 16 - len(data) % 16
        padding = bytes(pad) if num in self.bad_padding else bytes([pad]) * pad
        enc = Cipher(algorithms.AES(h), modes.CBC(iv)).encryptor()
        return iv + enc.update(data + padding) + enc.finalize()

    def string(self, num: int, data: bytes) -> bytes:
        return self._encrypt(num, data)

    def stream(self, num: int, stream: Stream) -> bytes:
        if num == self.crypt_stream:
            stream.dict = dict(stream.dict, Filter=[Name("Crypt"), Name("FlateDecode")],
                               DecodeParms=[{"Type": Name("CryptFilterDecodeParms"), "Name": Name(self.crypt_name)},
                                            None])
            return self._encrypt(num, stream.data) if self.crypt_name == "StdCF" else stream.data
        return self._encrypt(num, stream.data) if self.stm_f == "StdCF" else stream.data


def write_v4_case(name: str, **options) -> None:
    objs, trailer = source_objects()
    enc = V4Writer(bytes(trailer["ID"][0]), **options)
    objs[15] = enc.dict()
    trailer = dict(trailer, Encrypt=Ref(15))
    (HERE / name).write_bytes(write_pdf(objs, trailer, enc))


# Objects stored in the object stream of the object-stream cases (the catalog among them).
IN_OBJSTM = (1, 2, 3, 4, 7, 8, 10, 11, 12, 13, 14)


def write_objstm_case(name: str, bad_padding: bool = False) -> None:
    """AES-128 with the catalog (and every other non-stream object but /Encrypt) inside an
    encrypted object stream, and an xref stream: what repair must decrypt to find /Root."""
    objs, trailer = source_objects()
    objstm_num, xref_num = 16, 17
    enc = V4Writer(bytes(trailer["ID"][0]), bad_padding=frozenset({objstm_num} if bad_padding else ()))
    objs[15] = enc.dict()
    # Members are plaintext inside the stream: the container is encrypted as a whole.
    members = [(num, ser(objs[num], None, num)) for num in IN_OBJSTM]
    header = b" ".join(b"%d %d" % (num, off) for num, off in zip(
        [n for n, _ in members], _offsets([body for _, body in members])))
    body = b"\n".join(body for _, body in members)
    first = len(header) + 1
    objs[objstm_num] = Stream({"Type": Name("ObjStm"), "N": len(members), "First": first,
                               "Filter": Name("FlateDecode")}, zlib.compress(header + b"\n" + body, 9))
    out = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
    offsets = {}
    for num in sorted(n for n in objs if n not in IN_OBJSTM):
        offsets[num] = len(out)
        out += b"%d 0 obj\n" % num
        obj = objs[num]
        use = None if num == enc.encrypt_num else enc
        if isinstance(obj, Stream):
            data = enc.stream(num, obj)
            out += ser(dict(obj.dict, Length=len(data)), use, num) + b"\nstream\n" + data + b"\nendstream"
        else:
            out += ser(obj, use, num)
        out += b"\nendobj\n"
    rows = b"\0\0\0\0\0\xff\xff"  # object 0: free
    for num in range(1, xref_num + 1):
        if num in IN_OBJSTM:
            rows += b"\x02" + objstm_num.to_bytes(4, "big") + IN_OBJSTM.index(num).to_bytes(2, "big")
        else:
            off = len(out) if num == xref_num else offsets[num]
            rows += b"\x01" + off.to_bytes(4, "big") + b"\0\0"
    xref_offset = len(out)
    xref = {"Type": Name("XRef"), "Size": xref_num + 1, "W": [1, 4, 2], "Root": Ref(1), "Info": Ref(8),
            "ID": trailer["ID"], "Encrypt": Ref(15), "Length": len(rows)}
    out += b"%d 0 obj\n" % xref_num + ser(xref, None, xref_num) + b"\nstream\n" + rows + b"\nendstream\nendobj\n"
    out += b"startxref\n%d\n%%%%EOF\n" % xref_offset
    (HERE / name).write_bytes(bytes(out))


def _offsets(bodies: list[bytes]) -> list[int]:
    out, pos = [], 0
    for body in bodies:
        out.append(pos)
        pos += len(body) + 1
    return out


# --- qpdf cases ------------------------------------------------------------------------------

# (name, qpdf --bits/options, expected V, expected R, user-variant permission flags)
CASES = [
    ("r2-rc4-40", ["--bits=40", "--allow-weak-crypto"], 1, 2, ["--print=n", "--extract=n"]),
    ("r3-rc4-128", ["--bits=128", "--use-aes=n", "--allow-weak-crypto"], 2, 3,
     ["--print=low", "--extract=n", "--modify=form"]),
    ("r4-rc4-128", ["--bits=128", "--use-aes=n", "--force-V4", "--allow-weak-crypto"], 4, 4,
     ["--print=low", "--assemble=n"]),
    ("r4-aes-128", ["--bits=128", "--use-aes=y"], 4, 4, ["--extract=n", "--annotate=n"]),
    ("r4-aes-128-cleartext-metadata", ["--bits=128", "--use-aes=y", "--cleartext-metadata"], 4, 4,
     ["--modify=none"]),
    ("r5-aes-256", ["--bits=256", "--force-R5"], 5, 5, ["--print=none", "--accessibility=n"]),
    ("r6-aes-256", ["--bits=256"], 5, 6, ["--print=low", "--extract=n", "--form=n"]),
]

# Object streams for some cases, so encrypted containers (and members not decrypted twice) are covered.
OBJECT_STREAMS = {"r4-aes-128", "r6-aes-256"}


def qpdf(*args: str) -> str:
    return subprocess.run(["qpdf", *args], check=True, capture_output=True, text=True).stdout


def check_qpdf_version() -> str:
    version = qpdf("--version").splitlines()[0].split()[-1]
    if tuple(int(x) for x in version.split(".")[:2]) < QPDF_MIN:
        sys.exit(f"qpdf {version} is too old: need >= {'.'.join(map(str, QPDF_MIN))}")
    return version


def show_encryption(path: Path, password: str) -> str:
    return qpdf(f"--password={password}", "--show-encryption", str(path))


def check_revision(path: Path, password: str, v: int, r: int, qpdf_reads: bool = True) -> None:
    """/R from `qpdf --show-encryption` (from pypdf for files qpdf can't open), /V from the
    /Encrypt dictionary as pypdf parses it (qpdf doesn't print V)."""
    encrypt = pypdf.PdfReader(path).trailer.get("/Encrypt")
    encrypt = encrypt.get_object() if encrypt is not None else {}
    got_v = encrypt.get("/V")
    if got_v is None:
        sys.exit(f"{path.name}: no /Encrypt /V")
    if qpdf_reads:
        got_r = re.search(r"^R = (\d+)$", show_encryption(path, password), re.M)
        if got_r is None:
            sys.exit(f"{path.name}: qpdf --show-encryption printed no 'R = …' line")
        got_r = got_r.group(1)
    else:
        got_r = encrypt.get("/R")
    if int(got_r) != r or int(got_v) != v:
        sys.exit(f"{path.name}: expected V {v} R {r}, found V {got_v} R {got_r}")


def main() -> None:
    version = check_qpdf_version()
    objs, trailer = source_objects()
    source = HERE / "source.pdf"
    source.write_bytes(write_pdf(objs, trailer))
    passwords: dict[str, dict[str, str]] = {}

    for name, options, v, r, restrictions in CASES:
        modern = r >= 5
        for variant in ("empty", "user"):
            out = HERE / f"{name}-{variant}.pdf"
            user = "" if variant == "empty" else (USER_UNICODE if modern else USER)
            extra = restrictions if variant == "user" else []
            streams = ["--object-streams=generate"] if name in OBJECT_STREAMS else []
            # `--allow-weak-crypto` is a general option in qpdf >= 11 (outside the --encrypt block).
            weak = [o for o in options if o == "--allow-weak-crypto"]
            encrypt = [o for o in options if o != "--allow-weak-crypto"]
            qpdf(*weak, "--encrypt", f"--user-password={user}", f"--owner-password={OWNER}", *encrypt, *extra, "--",
                 *streams, str(source), str(out))
            check_revision(out, OWNER, v, r)
            passwords[out.name] = {"user": user, "owner": OWNER}

    # R6 with the same user and owner password: Algorithm 2.A reports the owner.
    same = HERE / "r6-aes-256-same.pdf"
    qpdf("--encrypt", f"--user-password={SAME}", f"--owner-password={SAME}", "--bits=256", "--", str(source),
         str(same))
    check_revision(same, SAME, 5, 6)
    passwords[same.name] = {"user": SAME, "owner": SAME}

    write_v4_case("identity-stmf.pdf", stm_f="Identity", crypt_stream=6, crypt_name="StdCF")
    write_v4_case("crypt-filter-identity.pdf", stm_f="StdCF", crypt_stream=6, crypt_name="Identity")
    # V4 key length from /CF/StdCF/Length, not the top-level /Length (defined for V2/V3 only).
    write_v4_case("v4-aes-128-cf-length.pdf", top_length=40, cf_length=16)
    write_v4_case("v4-rc4-40-cf-length.pdf", cfm="V2", key_bits=40, top_length=None, cf_length=5)
    write_objstm_case("objstm-catalog-aes-128.pdf")
    hand_written = ("identity-stmf.pdf", "crypt-filter-identity.pdf", "v4-aes-128-cf-length.pdf",
                    "v4-rc4-40-cf-length.pdf", "objstm-catalog-aes-128.pdf")
    for name in hand_written:
        # qpdf 12 takes the V4 key length from the top-level /Length and can't open the
        # /CF-length files; pypdf (and lamina, like pdf.js) read the crypt filter's /Length.
        qpdf_reads = name not in QPDF_CANNOT_READ
        if qpdf_reads:
            qpdf(f"--password={USER}", "--check", str(HERE / name))
        check_revision(HERE / name, USER, 4, 4, qpdf_reads)
        passwords[name] = {"user": USER, "owner": OWNER}
    # Deliberately damaged (no facts): the object stream's AES padding is invalid.
    write_objstm_case("objstm-bad-padding-aes-128.pdf", bad_padding=True)
    damaged = {"objstm-bad-padding-aes-128.pdf": {"user": USER, "owner": OWNER}}

    (HERE / "passwords.json").write_text(
        json.dumps({"qpdf": version, "files": passwords, "damaged": damaged}, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )

    seeds = SEEDS / "encrypted"
    seeds.mkdir(parents=True, exist_ok=True)
    everything = {**passwords, **damaged}
    for name in everything:
        shutil.copyfile(HERE / name, seeds / name)
    lines = sorted({pw for p in everything.values() for pw in p.values() if pw})
    (SEEDS / "passwords.txt").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"wrote {len(passwords)} encrypted fixtures with qpdf {version}")


if __name__ == "__main__":
    main()
