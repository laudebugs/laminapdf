#!/usr/bin/env python3
"""Write fixtures/encrypted/expected.json: facts about the encrypted fixtures, from pypdf and qpdf.

Usage (from the repository root, after make.py; pypdf pinned to 6.21.3, qpdf >= 11.7 on PATH):

    .venv/bin/python fixtures/encrypted/facts.py

Facts never come from lamina: page count, outline titles (flattened, depth first), Info strings,
come from pypdf opened with the user password (and checked again with the owner password); the
SHA-256 of each page's decoded content and of the XMP bytes from pypdf over a copy that
`qpdf --decrypt` wrote (pypdf alone, and /P with qpdf's permission rules, for the two files
qpdf can't open: make.QPDF_CANNOT_READ); permissions, /P and R come from
`qpdf --show-encryption`; /V, /Length and /EncryptMetadata from the /Encrypt dictionary as pypdf
reads it. Only this folder is read and written (B2c's corpus facts live in fixtures/corpus/).
"""

from __future__ import annotations

import hashlib
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path

import pypdf

from make import QPDF_CANNOT_READ

HERE = Path(__file__).resolve().parent
PYPDF = "6.21.3"

# qpdf --show-encryption labels → laminapdf_cos::Permissions methods.
PERMISSIONS = {
    "extract for accessibility": "accessibility",
    "extract for any purpose": "extract",
    "print low resolution": "print_low_res",
    "print high resolution": "print_high_res",
    "modify document assembly": "assemble",
    "modify forms": "fill_forms",
    "modify annotations": "annotate",
    "modify other": "modify",
}


def outline_titles(reader: pypdf.PdfReader) -> list[str]:
    titles: list[str] = []

    def walk(items) -> None:
        for item in items:
            if isinstance(item, list):
                walk(item)
            else:
                titles.append(item.title)

    walk(reader.outline)
    return titles


def facts_from_pypdf(path: Path, password: str | None, qpdf_reads: bool = True) -> dict:
    reader = pypdf.PdfReader(path)
    if reader.is_encrypted and not reader.decrypt(password or ""):
        sys.exit(f"{path.name}: pypdf rejects the password")
    info = reader.trailer["/Info"].get_object()
    facts = {
        "page_count": len(reader.pages),
        "outline_titles": outline_titles(reader),
        "info": {key[1:]: str(info[key]) for key in sorted(info)},
    }
    if not qpdf_reads:
        # qpdf can't open these (see make.QPDF_CANNOT_READ); they have no /Crypt filters, so
        # pypdf decodes their streams itself.
        metadata = reader.trailer["/Root"]["/Metadata"].get_object()
        facts["content_sha256"] = [hashlib.sha256(page.get_contents().get_data()).hexdigest() for page in reader.pages]
        facts["xmp_sha256"] = hashlib.sha256(metadata.get_data()).hexdigest()
        return facts
    # Stream facts from a copy qpdf decrypted: pypdf 6.21 can't decode a /Crypt filter with
    # /Name, which two fixtures use on purpose.
    with tempfile.TemporaryDirectory() as tmp:
        plain = Path(tmp) / "plain.pdf"
        args = [f"--password={password}", "--decrypt"] if password is not None else []
        subprocess.run(["qpdf", *args, str(path), str(plain)], check=True)
        decrypted = pypdf.PdfReader(plain)
        metadata = decrypted.trailer["/Root"]["/Metadata"].get_object()
        facts["content_sha256"] = [
            hashlib.sha256(page.get_contents().get_data()).hexdigest() for page in decrypted.pages
        ]
        facts["xmp_sha256"] = hashlib.sha256(metadata.get_data()).hexdigest()
    return facts


def bit(p: int, n: int) -> bool:
    return bool(p & (1 << (n - 1)))


def permissions_from_p(p: int, r: int) -> dict:
    """qpdf's --show-encryption rules (QPDF::allow*), for the files qpdf can't open."""
    r2 = r < 3
    return {
        "accessibility": bit(p, 5) if r2 else bit(p, 10),
        "extract": bit(p, 5),
        "print_low_res": bit(p, 3),
        "print_high_res": bit(p, 3) and (r2 or bit(p, 12)),
        "assemble": bit(p, 4) if r2 else bit(p, 11),
        "fill_forms": bit(p, 6) if r2 else bit(p, 9),
        "annotate": bit(p, 6),
        "modify": bit(p, 4),
    }


def key_bits(encrypt) -> int:
    """The file key length in bits as the file states it: R5/R6 256; V4 the /Length of the crypt
    filter streams (else strings) use, in bytes or bits, default 128; V2/V3 /Length, default
    40; V1 40."""
    v, r = int(encrypt["/V"]), int(encrypt["/R"])
    if r >= 5:
        return 256
    if v >= 4:
        cf = encrypt.get("/CF", {})
        for key in ("/StmF", "/StrF"):
            name = encrypt.get(key)
            if name is not None and name != "/Identity" and name in cf and "/Length" in cf[name]:
                length = int(cf[name]["/Length"])
                return length * 8 if length <= 16 else length
        return int(encrypt.get("/Length", 128))
    return int(encrypt.get("/Length", 40)) if v >= 2 else 40


def qpdf_encryption(path: Path, password: str) -> dict:
    out = subprocess.run(
        ["qpdf", f"--password={password}", "--show-encryption", str(path)], check=True, capture_output=True, text=True
    ).stdout
    perms = {}
    for label, key in PERMISSIONS.items():
        m = re.search(rf"^{label}: (allowed|not allowed)$", out, re.M)
        if m is None:
            sys.exit(f"{path.name}: no '{label}' in qpdf --show-encryption")
        perms[key] = m.group(1) == "allowed"
    return {
        "R": int(re.search(r"^R = (\d+)$", out, re.M).group(1)),
        "P": int(re.search(r"^P = (-?\d+)$", out, re.M).group(1)),
        "permissions": perms,
    }


def main() -> None:
    if pypdf.__version__ != PYPDF:
        sys.exit(f"pypdf {pypdf.__version__} found; facts are pinned to pypdf {PYPDF}")
    passwords = json.loads((HERE / "passwords.json").read_text(encoding="utf-8"))
    source = facts_from_pypdf(HERE / "source.pdf", None)
    files = {}
    for name, pw in passwords["files"].items():
        path = HERE / name
        qpdf_reads = name not in QPDF_CANNOT_READ
        facts = facts_from_pypdf(path, pw["user"], qpdf_reads)
        if facts_from_pypdf(path, pw["owner"], qpdf_reads) != facts:
            sys.exit(f"{name}: the owner password reads differently from the user password")
        encrypt = pypdf.PdfReader(path).trailer["/Encrypt"].get_object()
        if qpdf_reads:
            enc = qpdf_encryption(path, pw["owner"])
        else:
            p, r = int(encrypt["/P"]), int(encrypt["/R"])
            enc = {"R": r, "P": p, "permissions": permissions_from_p(p & 0xFFFFFFFF, r)}
        files[name] = {
            "user_password": pw["user"],
            "owner_password": pw["owner"],
            "V": int(encrypt["/V"]),
            "R": enc["R"],
            "key_bits": key_bits(encrypt),
            # BooleanObject(False) is truthy; compare its value.
            "encrypt_metadata": encrypt.get("/EncryptMetadata", pypdf.generic.BooleanObject(True)).value,
            "P": enc["P"],
            "permissions": enc["permissions"],
            # True when the XMP packet is stored unencrypted (findable in the raw bytes).
            "xmp_plaintext": b"<x:xmpmeta" in path.read_bytes(),
            **facts,
        }
    out = {
        "generated_by": f"fixtures/encrypted/facts.py (pypdf {PYPDF}, qpdf {passwords['qpdf']})",
        "source": {"file": "source.pdf", **source},
        "files": files,
    }
    (HERE / "expected.json").write_text(json.dumps(out, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"wrote facts for {len(files)} files")


if __name__ == "__main__":
    main()
