#!/usr/bin/env python3
"""Write fixtures/encrypted/written/expected.json: facts about the files lamina wrote, from pypdf and qpdf.

Usage (from the repository root, after `UPDATE_GOLDEN=1 cargo test -p laminapdf-save --test
set_encryption`; pypdf pinned to 6.21.3, qpdf >= 11.7 on PATH):

    .venv/bin/python fixtures/encrypted/written/facts.py

The files are lamina's own output (`SetEncryption`, E11b), so the facts never come from lamina:
page count, the Info title and the SHA-256 of the first page's decoded content come from pypdf
opened with the file's password (and refused with a wrong one); /R and /P come from
`qpdf --show-encryption`, and `qpdf --check` must pass. `crates/laminapdf-save/tests/set_encryption.rs`
checks that lamina reads each file the same way.
"""

from __future__ import annotations

import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

import pypdf

HERE = Path(__file__).resolve().parent
PYPDF = "6.21.3"

# The file name → the password that opens it (the file's user password, empty for none).
PASSWORDS = {
    "r6-user.pdf": "open-pw",
    "r6-owner-permissions.pdf": "",
    "r6-no-metadata-encryption.pdf": "open-pw",
}
OWNER = "owner-pw"


def qpdf(path: Path, password: str, *args: str) -> str:
    done = subprocess.run(
        ["qpdf", f"--password={password}", *args, str(path)], capture_output=True, text=True, check=True
    )
    return done.stdout


def facts(path: Path, password: str) -> dict:
    reader = pypdf.PdfReader(str(path))
    if reader.decrypt("not the password") != pypdf.PasswordType.NOT_DECRYPTED:
        raise SystemExit(f"{path.name}: a wrong password opened the file")
    kind = reader.decrypt(password)
    if kind == pypdf.PasswordType.NOT_DECRYPTED:
        raise SystemExit(f"{path.name}: the password does not open the file")
    owner = pypdf.PdfReader(str(path))
    if owner.decrypt(OWNER) != pypdf.PasswordType.OWNER_PASSWORD:
        raise SystemExit(f"{path.name}: the owner password is not the owner's")
    qpdf(path, password, "--check")
    shown = qpdf(path, password, "--show-encryption")
    revision = int(re.search(r"^R = (\d+)", shown, re.M).group(1))
    p = int(re.search(r"^P = (-?\d+)", shown, re.M).group(1))
    content = reader.pages[0].get_contents().get_data()
    return {
        "password": password,
        "pages": len(reader.pages),
        "title": reader.metadata.title,
        "content_sha256": hashlib.sha256(content).hexdigest(),
        "revision": revision,
        "p": p,
    }


def main() -> int:
    if pypdf.__version__ != PYPDF:
        raise SystemExit(f"pypdf {pypdf.__version__}, expected {PYPDF}")
    out = {name: facts(HERE / name, password) for name, password in PASSWORDS.items()}
    (HERE / "expected.json").write_text(json.dumps({"owner_password": OWNER, "files": out}, indent=2) + "\n")
    print(f"wrote {HERE / 'expected.json'} for {len(out)} files")
    return 0


if __name__ == "__main__":
    sys.exit(main())
