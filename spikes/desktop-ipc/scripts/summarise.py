#!/usr/bin/env python3
"""Median of each transport's runs per machine and combination, as Markdown tables.

    python3 scripts/summarise.py results/ > results/summary.md
    python3 scripts/summarise.py results/ --compact   # the README's one table per machine

Standard library only. Reads results/<label>-<source>-<tile>-dpr<n>.csv (the measured runs),
<label>-sweep.csv, <label>-headers-{off,on}-coi.csv and <label>-failure.csv.
"""

import csv
import statistics
import sys
from collections import defaultdict
from pathlib import Path

ORDER = ["channel", "response", "scheme-raw", "scheme-png", "scheme-webp", "scheme-qoi"]
COLUMNS = [
    ("burst tiles/s", "burst_tiles_per_s", 0),
    ("burst MB/s", "burst_mb_per_s", 0),
    ("ready p50 ms", "ready_p50_ms", 1),
    ("ready p95 ms", "ready_p95_ms", 1),
    ("dropped frames", "dropped_frames", 0),
    ("blank frames %", "blank_frame_pct", 0),
    ("400 % fill ms", "zoom400_fill_ms", 0),
    ("webview RSS MB", "web_rss_peak_mb", 0),
    ("CPU %", "cpu_avg_pct", 0),
    ("encode ms", "encode_ms_avg", 2),
]


def num(value: str) -> float | None:
    try:
        f = float(value)
    except ValueError:
        return None
    return None if f != f else f  # NaN: the zoom never filled


def median(rows: list[dict], key: str) -> str:
    values = [v for r in rows if (v := num(r.get(key, ""))) is not None]
    return "" if not values else f"{statistics.median(values):.{dict((k, d) for _, k, d in COLUMNS)[key]}f}"


def combos(folder: Path) -> dict[str, dict[str, list[dict]]]:
    """label → combination → rows."""
    out: dict[str, dict[str, list[dict]]] = defaultdict(dict)
    for path in sorted(folder.glob("*-dpr[12].csv")):
        rows = list(csv.DictReader(path.open()))
        if not rows:
            continue
        label = rows[0]["label"]
        combo = path.stem[len(label) + 1 :]
        out[label][combo] = rows
    return out


def table(rows: list[dict]) -> list[str]:
    lines = [
        "| transport | n | runs | " + " | ".join(name for name, _, _ in COLUMNS) + " |",
        "| --- | --: | --: | " + " | ".join("--:" for _ in COLUMNS) + " |",
    ]
    by_transport = defaultdict(list)
    for r in rows:
        by_transport[r["transport"]].append(r)
    for t in ORDER:
        rs = by_transport.get(t)
        if rs:
            cells = [median(rs, key) for _, key, _ in COLUMNS]
            lines.append(f"| {t} | {rs[0]['inflight']} | {len(rs)} | " + " | ".join(cells) + " |")
    return lines


def isolation(folder: Path, label: str) -> list[str]:
    lines = []
    for headers in ("off", "on"):
        path = folder / f"{label}-headers-{headers}-coi.csv"
        if path.exists():
            r = next(csv.DictReader(path.open()))
            lines.append(
                f"| {headers} | {r['cross_origin_isolated']} | {r['shared_array_buffer']} | {r['ipc_invoke']} | "
                f"{r['fetch_corp_same-origin']} | {r['img_corp_same-origin']} | {r['img_corp_cross-origin']} | {r['origin']} |"
            )
    if not lines:
        return []
    return [
        "| headers | crossOriginIsolated | SharedArrayBuffer | invoke | fetch tile (CORP same-origin) "
        "| `<img>` tile (CORP same-origin) | `<img>` tile (CORP cross-origin) | page origin |",
        "| --- | --- | --- | --- | --- | --- | --- | --- |",
        *lines,
    ]


COMPACT = ["pdf-256-dpr1", "pdf-256-dpr2", "pdf-512-dpr1", "pdf-512-dpr2"]


def compact(labels: dict[str, dict[str, list[dict]]]) -> None:
    """Per machine: transport × combination, each cell `tiles/s · MB/s · ready p50/p95 ms · dropped`."""
    for label in sorted(labels):
        print(f"**{label}**\n")
        print("| transport | " + " | ".join(c.removeprefix("pdf-").replace("-dpr", " px, DPR ") for c in COMPACT) + " |")
        print("| --- | " + " | ".join("---" for _ in COMPACT) + " |")
        for t in ORDER:
            cells = []
            for c in COMPACT:
                rs = [r for r in labels[label].get(c, []) if r["transport"] == t]
                cells.append(
                    f"{median(rs, 'burst_tiles_per_s')} · {median(rs, 'burst_mb_per_s')} · "
                    f"{median(rs, 'ready_p50_ms')}/{median(rs, 'ready_p95_ms')} · {median(rs, 'dropped_frames')}"
                    if rs
                    else ""
                )
            print(f"| {t} | " + " | ".join(cells) + " |")
        print()


def main() -> None:
    folder = Path(sys.argv[1] if len(sys.argv) > 1 else "results")
    labels = combos(folder)
    if "--compact" in sys.argv:
        compact(labels)
        return
    extra = {p.name.removesuffix("-headers-off-coi.csv") for p in folder.glob("*-headers-off-coi.csv")}
    for label in sorted(set(labels) | extra):
        print(f"## {label}\n")
        machine = folder / f"{label}-machine.txt"
        if machine.exists():
            print("```\n" + machine.read_text().strip() + "\n```\n")
        iso = isolation(folder, label)
        if iso:
            print("\n".join(iso) + "\n")
        failure = folder / f"{label}-failure.csv"
        if failure.exists():
            for r in csv.DictReader(failure.open()):
                print(f"**Run stopped:** {r['error']}\n")
        sweep = folder / f"{label}-sweep.csv"
        if sweep.exists():
            rows = list(csv.DictReader(sweep.open()))
            print("In-flight sweep (512 px, DPR 2, one run each), burst tiles/s:\n")
            print("| transport | n = 4 | n = 8 | n = 16 |\n| --- | --: | --: | --: |")
            for t in ORDER:
                cells = {r["inflight"]: r["burst_tiles_per_s"] for r in rows if r["transport"] == t}
                if cells:
                    print(f"| {t} | {cells.get('4', '')} | {cells.get('8', '')} | {cells.get('16', '')} |")
            print()
        for combo, rows in sorted(labels.get(label, {}).items()):
            print(f"### {label}: {combo}\n")
            print("\n".join(table(rows)) + "\n")


if __name__ == "__main__":
    main()
