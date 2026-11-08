"""Story-level figures for the report: PDF stories per year, per area and as Show HN launches.

Usage: python3 -I tools/hn-research/story_stats.py <data-root>
Reads <data-root>/data/stories.jsonl (from collect_stories.py); writes
<data-root>/stats/story_volume.json and prints the tables. A story counts in an area when its
title matches the area's pattern, so a story can count in several areas or in none.
"""

import json
import re
import statistics
import sys
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(sys.argv[1])
YEARS = [str(y) for y in range(2017, 2027)]
AREAS = {
    "readers": r"reader|viewer|preview|sumatra|okular|zathura|evince|foxit|skim|pdf\.js|pdfjs|scholar",
    "editors": r"edit|annotat|sign|redact|fill|form|merge|split|stirling|compress|ocrmypdf|pdftk|qpdf|scan|watermark|rotate",
    "parsing": r"extract|pars|text|table|ocr|tesseract|markdown|llm|rag|gemini|mistral|marker|docling|chat with|q&a|"
               r"ingest|convert pdf|pdf to|search",
    "generation": r"generat|html|create|typst|latex|invoice|report|resume|print|paged|weasyprint|wkhtmltopdf|prince|"
                  r"book|typeset",
    "format_security": r"security|vulnerab|exploit|malware|javascript|spec|format|polyglot|tetris|doom|metadata|"
                       r"forensic|accessib|pdf/a|archiv|unfit|font",
}
AREAS = {k: re.compile(v, re.I) for k, v in AREAS.items()}


def main() -> None:
    stories = [json.loads(line) for line in open(ROOT / "data" / "stories.jsonl")]
    volume: dict[str, Counter] = defaultdict(Counter)
    comments: dict[str, Counter] = defaultdict(Counter)
    every: Counter = Counter()
    show_hn: dict[str, list] = defaultdict(list)
    for s in stories:
        year, title = s["created_at"][:4], s.get("title") or ""
        every[year] += 1
        if title.lower().startswith("show hn"):
            show_hn[year].append(s.get("points") or 0)
        for area, rx in AREAS.items():
            if rx.search(title):
                volume[area][year] += 1
                comments[area][year] += s.get("num_comments") or 0

    print("year  stories  show-hn  share  median-points")
    for y in YEARS:
        pts = show_hn[y]
        median = statistics.median(pts) if pts else 0
        print(f"{y}  {every[y]:7}  {len(pts):7}  {100 * len(pts) / every[y]:4.1f}%  {median}")
    print("\nstories per area per year,", YEARS[0], "to", YEARS[-1])
    for area in AREAS:
        print(f"{area:16}", [volume[area][y] for y in YEARS])

    (ROOT / "stats").mkdir(exist_ok=True)
    out = {
        "areas_story_volume": {a: {y: volume[a][y] for y in YEARS} for a in AREAS},
        "areas_thread_comments": {a: {y: comments[a][y] for y in YEARS} for a in AREAS},
        "stories_per_year": {y: every[y] for y in YEARS},
        "show_hn_per_year": {y: len(show_hn[y]) for y in YEARS},
        "show_hn_median_points": {y: statistics.median(show_hn[y]) if show_hn[y] else 0 for y in YEARS},
    }
    (ROOT / "stats" / "story_volume.json").write_text(json.dumps(out, indent=1))


if __name__ == "__main__":
    main()
