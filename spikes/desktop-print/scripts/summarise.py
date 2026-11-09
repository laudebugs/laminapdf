#!/usr/bin/env python3
"""The README's tables from results/<label>-jobs.csv and <label>-fidelity.csv.

    python3 scripts/summarise.py results/ > results/summary.md

Standard library only. Per machine: one row per path and fixture with the pages printed at the
right size, the pages whose content moved more than 2 pt, SSIM (min / median, after aligning),
whether any page gained text (a header or footer), and for the 50-page fixture the time to
spool, the peak memory and the file size.
"""

import csv
import statistics
import sys
from collections import defaultdict
from pathlib import Path

PATHS = {"webview": "A webview", "raster": "B native raster", "vector": "C native vector"}
FIXTURES = ["mixed-50", "mixed-sizes", "vectors", "mixed-50@3-7"]


def rows(path: Path) -> list[dict]:
    return list(csv.DictReader(path.open())) if path.exists() else []


def main() -> None:
    folder = Path(sys.argv[1] if len(sys.argv) > 1 else "results")
    labels = sorted(p.name.removesuffix("-jobs.csv") for p in folder.glob("*-jobs.csv"))
    for label in labels:
        jobs = {(r["path"], r["fixture"]): r for r in rows(folder / f"{label}-jobs.csv")}
        pages = defaultdict(list)
        notes = defaultdict(list)
        for r in rows(folder / f"{label}-fidelity.csv"):
            if r["page"]:
                pages[(r["path"], r["fixture"])].append(r)
            elif r["note"]:
                notes[(r["path"], r["fixture"])].append(r["note"])
        print(f"### {label}\n")
        machine = folder / f"{label}-machine.txt"
        if machine.exists():
            print("```\n" + machine.read_text().strip() + "\n```\n")
        print("| path | fixture | right size | moved > 2 pt | SSIM min / median | added text | spool s | peak MB (app / webview) | file MB |")
        print("| --- | --- | --: | --: | --: | --- | --: | --: | --: |")
        for path in ["webview", "raster", "vector"]:
            for fixture in FIXTURES:
                job = jobs.get((path, fixture))
                ps = pages.get((path, fixture), [])
                if not job and not ps:
                    continue
                if job and job["ok"] != "true":
                    print(f"| {PATHS[path]} | {fixture} | failed: {job['error']} | | | | | | |")
                    continue
                n = len(ps)
                right = sum(r["size_ok"] == "true" for r in ps)
                moved = sum(
                    abs(float(r["shift_x_pt"] or 0)) > 2 or abs(float(r["shift_y_pt"] or 0)) > 2 for r in ps
                )
                ss = [float(r["ssim"]) for r in ps if r["ssim"]]
                ssim = f"{min(ss):.3f} / {statistics.median(ss):.3f}" if ss else ""
                added = "yes" if any(r["added_text"] for r in ps) else "no"
                note = "; ".join(notes.get((path, fixture), []))
                spool = mem = size = ""
                if job and fixture.startswith("mixed-50"):
                    spool = f"{int(job['spool_ms']) / 1000:.1f}"
                    mem = f"{job['main_rss_peak_mb']} / {job['web_rss_peak_mb']}"
                    size = f"{int(job['out_bytes']) / 1048576:.1f}"
                sized = f"{right}/{n}" + (f" ({note})" if note else "")
                print(f"| {PATHS[path]} | {fixture} | {sized} | {moved} | {ssim} | {added} | {spool} | {mem} | {size} |")
        print()


if __name__ == "__main__":
    main()
