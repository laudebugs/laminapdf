#!/usr/bin/env python3
"""Write fixtures/encrypted/expected.json: facts about the encrypted fixtures, from pypdf and qpdf.

Usage (from the repository root, after make.py; pypdf pinned to 6.21.3, qpdf >= 11.7 on PATH):

    .venv/bin/python fixtures/encrypted/facts.py

Facts never come from lamina: page count, outline titles (flattened, depth first), Info strings,
come from pypdf opened with the user password (and checked again with the owner password); the
SHA-256 of each page's decoded content and of the XMP bytes from pypdf over a copy that
`qpdf --decrypt` wrote; permissions, /P and R come from
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

HERE = Path(__file__).resolve().parent
PYPDF = "6.21.3"

# qpdf --show-encryption labels → lamina_cos::Permissions methods.
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


def facts_from_pypdf(path: Path, password: str | None) -> dict:
    reader = pypdf.PdfReader(path)
    if reader.is_encrypted and not reader.decrypt(password or ""):
        sys.exit(f"{path.name}: pypdf rejects the password")
    info = reader.trailer["/Info"].get_object()
    facts = {
        "page_count": len(reader.pages),
        "outline_titles": outline_titles(reader),
        "info": {key[1:]: str(info[key]) for key in sorted(info)},
    }
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
        facts = facts_from_pypdf(path, pw["user"])
        if facts_from_pypdf(path, pw["owner"]) != facts:
            sys.exit(f"{name}: the owner password reads differently from the user password")
        encrypt = pypdf.PdfReader(path).trailer["/Encrypt"].get_object()
        enc = qpdf_encryption(path, pw["owner"])
        files[name] = {
            "user_password": pw["user"],
            "owner_password": pw["owner"],
            "V": int(encrypt["/V"]),
            "R": enc["R"],
            "key_bits": 256 if enc["R"] >= 5 else int(encrypt.get("/Length", 40)),
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
