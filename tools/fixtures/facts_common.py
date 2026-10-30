"""Helpers shared by the facts scripts in tools/fixtures/ (text_facts.py, build_facts.py):
the SHA-256 and `qpdf --check` of a file, pdftotext's lines, the qpdf and poppler versions,
and running a pdf.js script under Node with PDFJS_DIST. Tools only, never run by tests."""

import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def sha256(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def qpdf_check(path):
    """Whether `qpdf --check` finds no errors."""
    return subprocess.run(["qpdf", "--check", str(path)], capture_output=True, text=True).returncode == 0


def qpdf_version():
    out = subprocess.run(["qpdf", "--version"], capture_output=True, text=True, check=True)
    return out.stdout.split()[2]


def pdftotext(path, page=1):
    """pdftotext's lines for `page` (its default reading-order mode, UTF-8), without the blank
    lines it puts between blocks and the form feed that ends the page."""
    out = subprocess.run(
        ["pdftotext", "-f", str(page), "-l", str(page), "-enc", "UTF-8", str(path), "-"],
        capture_output=True,
        text=True,
        check=True,
    )
    return [line for line in out.stdout.replace("\f", "").split("\n") if line]


def poppler_version():
    # `pdftotext -v` prints "pdftotext version X.Y.Z" on stderr.
    out = subprocess.run(["pdftotext", "-v"], capture_output=True, text=True, check=True)
    return (out.stderr or out.stdout).split()[2]


def run_pdfjs(script, paths):
    """The JSON a tools/fixtures/<script> prints for `paths`, with pdf.js from $PDFJS_DIST."""
    dist = os.environ.get("PDFJS_DIST")
    if not dist:
        sys.exit("set PDFJS_DIST to node_modules/pdfjs-dist (pdfjs-dist@6.4.299)")
    script = ROOT / "tools" / "fixtures" / script
    out = subprocess.run(["node", str(script), dist, *map(str, paths)], capture_output=True, text=True, check=True)
    return json.loads(out.stdout)
