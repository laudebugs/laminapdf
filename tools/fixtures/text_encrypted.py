#!/usr/bin/env python3
"""Encrypted copies of text goldens for the extraction tests (todo B9d).

Usage (from the repository root; needs qpdf >= 11.7 on PATH, no Python packages):

    python3 tools/fixtures/text_encrypted.py

Writes fixtures/text/encrypted/:

* text-shaped-aes-256.pdf: fixtures/golden/text-shaped.pdf (five scripts) encrypted with the
  standard security handler R6 (AES-256), user password `user`, owner password `owner`;
* build-text-multiscript-rc4-128.pdf: fixtures/golden/build-text-multiscript.pdf with R3
  (RC4, 128-bit key), the same passwords.

crates/laminapdf-text/tests/extract_encrypted.rs opens each with the user password and requires the
same glyphs, text and quads as its plaintext golden. The files are not byte-reproducible (qpdf
uses random IVs and salts); only what they decrypt to matters. Rerun after the goldens change.
"""

from __future__ import annotations

import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
GOLDEN = ROOT / "fixtures" / "golden"
OUT = ROOT / "fixtures" / "text" / "encrypted"

# (output name, golden, qpdf key length)
CASES = [
    ("text-shaped-aes-256.pdf", "text-shaped.pdf", "256"),
    ("build-text-multiscript-rc4-128.pdf", "build-text-multiscript.pdf", "128"),
]


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    for name, golden, bits in CASES:
        out = OUT / name
        # RC4 on purpose (readers must still decrypt it): qpdf writes it only when allowed.
        weak = ["--allow-weak-crypto"] if bits == "128" else []
        cipher = ["--use-aes=n"] if bits == "128" else []
        args = ["qpdf", *weak, "--encrypt", "--user-password=user", "--owner-password=owner", f"--bits={bits}", *cipher]
        subprocess.run([*args, "--", str(GOLDEN / golden), str(out)], check=True)
        print(out.relative_to(ROOT))


if __name__ == "__main__":
    main()
