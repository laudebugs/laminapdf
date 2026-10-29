#!/usr/bin/env python3
"""Writes the UAX conformance subsets in fixtures/unicode/ (todo B4b) from the Unicode
Character Database test files: every Nth test line of BidiCharacterTest.txt (UAX #9) and
LineBreakTest.txt (UAX #14), with the file's own header kept (its copyright and the Unicode
licence pointer). Comments after a line break test are dropped to keep the subset small.

    curl -fLO https://www.unicode.org/Public/17.0.0/ucd/BidiCharacterTest.txt
    curl -fLO https://www.unicode.org/Public/17.0.0/ucd/auxiliary/LineBreakTest.txt
    python3 tools/fixtures/unicode_subsets.py <download-dir>

The downloads are checked against the SHA-256 values below; the script never touches the
network. Unicode 17.0.0 is the version of ICU4X 2.4's compiled data. The files are under the
Unicode License v3 (https://www.unicode.org/license.txt), which allows copying and
redistribution with the copyright notice, kept in each subset's header.
Standard library only.
"""

import hashlib
import sys
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "unicode"

# name → (SHA-256 of the 17.0.0 file, keep every Nth test line)
SOURCES = {
    "BidiCharacterTest.txt": ("a3e6e905ab5afbe318a96df5401d0372a04cd73ef139ab5e3cf0ae241c255488", 100),
    "LineBreakTest.txt": ("e69884e0dde6a8724873f885d68c52dc14518abf9ae4ca9e2283b8773db3b752", 10),
}


def subset(text, step, strip_comments):
    lines = text.splitlines()
    header = []
    for line in lines:
        if not line.startswith("#"):
            break
        header.append(line)
    tests = [l for l in lines if l.strip() and not l.startswith("#")]
    kept = tests[::step]
    if strip_comments:
        kept = [l.split("#", 1)[0].rstrip() for l in kept]
    note = [
        "#",
        f"# lamina subset (tools/fixtures/unicode_subsets.py): every {step}th of the {len(tests)} test",
        f"# lines above, {len(kept)} lines" + (", comments dropped." if strip_comments else "."),
        "#",
    ]
    return "\n".join(header + note + kept) + "\n"


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    downloads = Path(sys.argv[1])
    OUT.mkdir(parents=True, exist_ok=True)
    for name, (digest, step) in SOURCES.items():
        path = downloads / name
        if not path.is_file() or hashlib.sha256(path.read_bytes()).hexdigest() != digest:
            sys.exit(f"{path}: missing or wrong SHA-256 (expected Unicode 17.0.0)")
        out = OUT / name
        out.write_text(subset(path.read_text(encoding="utf-8"), step, name == "LineBreakTest.txt"), encoding="utf-8")
        print(f"{out}  {out.stat().st_size}")


if __name__ == "__main__":
    main()
