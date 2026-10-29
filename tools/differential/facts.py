#!/usr/bin/env python3
"""Differential facts from pypdf, PDFium (pypdfium2) and qpdf (ADR 0018 tier 6, nightly lane).

    facts.py <pypdf|pdfium|qpdf> <list.tsv> <out.json>

<list.tsv> has one `<name>\\t<path>` line per input. <out.json> is
{"tool", "version", "reports_repair", "facts": {name: {outcome, page_count, outline_titles}}},
the shape `cargo xtask differential --live` compares with lamina's (outcome is opened,
repaired, failed or timeout). Never computed by lamina. Inputs are data: they are only read,
never executed. Requires the pinned versions in requirements.txt (qpdf: the version the
nightly workflow installs, recorded in the report).
"""

import json
import signal
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "corpus-facts"))
import facts as pypdf_facts  # noqa: E402 - tools/corpus-facts/facts.py, the PR lane's pypdf facts

PER_FILE_SECONDS = 60
FAILED = {"outcome": "failed", "page_count": None, "outline_titles": None}
TIMEOUT = {"outcome": "timeout", "page_count": None, "outline_titles": None}


class Timeout(BaseException):
    """Raised by SIGALRM. A BaseException, so a reader's own `except Exception` (pypdf's or
    ours) can't swallow it and turn a hang into a "failed" outcome. SIGALRM can't interrupt
    native code (PDFium) either; `cargo xtask differential --live` also kills a reader process
    that overruns and reruns its files one per process (tools/xtask/src/runner.rs)."""


def on_alarm(_signum, _frame):
    raise Timeout


def pypdf_tool():
    import pypdf

    return pypdf.__version__, True, pypdf_facts.outcome_facts


def pdfium_tool():
    import pypdfium2
    import pypdfium2.version

    def facts(data):
        try:
            pdf = pypdfium2.PdfDocument(data)
        except Exception:  # noqa: BLE001 - any failure to open is the "failed" outcome
            return dict(FAILED)
        try:
            pages = len(pdf)
            try:
                titles = [bookmark.get_title() for bookmark in pdf.get_toc()]
            except Exception:  # noqa: BLE001 - an unreadable outline is its own fact
                titles = None
        finally:
            pdf.close()
        # PDFium repairs silently: it can't tell "opened" from "repaired".
        return {"outcome": "opened", "page_count": pages, "outline_titles": titles}

    version = f"pypdfium2 {pypdfium2.version.PYPDFIUM_INFO}, PDFium {pypdfium2.version.PDFIUM_INFO}"
    return version, False, facts


QPDF_REPAIR_HINTS = ("damaged", "reconstruct", "xref", "startxref")


def qpdf_outline_titles(items):
    out = []
    for item in items:
        out.append(item.get("title", ""))
        out.extend(qpdf_outline_titles(item.get("kids", [])))
    return out


def qpdf_tool():
    version = subprocess.run(["qpdf", "--version"], capture_output=True, text=True, check=True).stdout
    version = version.splitlines()[0].strip()

    # qpdf needs a seekable file, so it gets the path rather than the bytes.
    def facts_of_path(path):
        args = ["qpdf", "--json=2", "--json-key=pages", "--json-key=outlines", str(path)]
        try:
            run = subprocess.run(args, capture_output=True, timeout=PER_FILE_SECONDS, check=False)
        except subprocess.TimeoutExpired:
            return dict(TIMEOUT)
        # Exit codes: 0 ok, 3 ok with warnings, 2 errors (qpdf --help=exit-status).
        if run.returncode not in (0, 3):
            return dict(FAILED)
        try:
            doc = json.loads(run.stdout)
        except ValueError:
            return dict(FAILED)
        stderr = run.stderr.decode("utf-8", "replace").lower()
        repaired = run.returncode == 3 and any(hint in stderr for hint in QPDF_REPAIR_HINTS)
        outlines = doc.get("outlines")
        return {
            "outcome": "repaired" if repaired else "opened",
            "page_count": len(doc.get("pages", [])),
            "outline_titles": qpdf_outline_titles(outlines) if isinstance(outlines, list) else None,
        }

    return version, True, facts_of_path


def main(argv):
    if len(argv) != 3 or argv[0] not in ("pypdf", "pdfium", "qpdf"):
        sys.exit(__doc__)
    tool, list_path, out_path = argv
    if tool == "pypdf" and __import__("pypdf").__version__ != pypdf_facts.PINNED:
        sys.exit(f"pypdf {__import__('pypdf').__version__} found; requirements.txt pins {pypdf_facts.PINNED}")
    version, reports_repair, facts = {"pypdf": pypdf_tool, "pdfium": pdfium_tool, "qpdf": qpdf_tool}[tool]()
    signal.signal(signal.SIGALRM, on_alarm)
    results = {}
    for line in Path(list_path).read_text(encoding="utf-8").splitlines():
        if not line:
            continue
        name, path = line.split("\t", 1)
        if tool == "qpdf":
            results[name] = facts(path)
            continue
        data = Path(path).read_bytes()
        signal.alarm(PER_FILE_SECONDS)
        try:
            results[name] = facts(data)
        except Timeout:
            results[name] = dict(TIMEOUT)
        except Exception:  # noqa: BLE001 - a crash inside the tool counts as failing to open
            results[name] = dict(FAILED)
        finally:
            signal.alarm(0)
    report = {"tool": tool, "version": version, "reports_repair": reports_repair, "facts": results}
    Path(out_path).write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"{tool} ({version}): facts for {len(results)} files in {out_path}")


if __name__ == "__main__":
    import logging

    logging.basicConfig(level=logging.ERROR)
    main(sys.argv[1:])
