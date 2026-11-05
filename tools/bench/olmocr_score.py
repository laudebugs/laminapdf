"""Scores Markdown files against olmOCR-bench's unit tests, with the benchmark's own test code.

`cargo xtask extract-bench` writes one Markdown file per PDF (`<candidate>/<pdf stem>_pg<page>_repeat1.md`)
and runs this script, which loads each subset's tests (`olmocr.bench.tests.load_tests`, pinned in
requirements.txt) and runs every test on its file, as olmOCR's `olmocr.bench.benchmark` does: a
missing or empty file fails its tests. The report has, per subset, the pass rate and the count per
test type, and the overall score as the benchmark reports it: the mean of the subsets' pass rates.

    python -I tools/bench/olmocr_score.py <bench_data> <candidate> <report.json> <subset>...

Never run by tests (it needs the dataset and the pinned packages).
"""

import json
import os
import sys
from collections import defaultdict


def score(bench_data, candidate, subsets):
    from olmocr.bench.tests import load_tests

    report = {"subsets": {}, "failures": []}
    for subset in subsets:
        tests = load_tests(os.path.join(bench_data, f"{subset}.jsonl"))
        by_type = defaultdict(lambda: [0, 0])
        passed_all = 0
        for test in tests:
            stem = os.path.splitext(test.pdf)[0]
            path = os.path.join(candidate, f"{stem}_pg{test.page}_repeat1.md")
            try:
                with open(path, encoding="utf-8") as f:
                    markdown = f.read()
            except FileNotFoundError:
                markdown = ""
            try:
                passed, why = test.run(markdown)
            except Exception as e:  # a test that crashes counts as failed, as in the benchmark
                passed, why = False, f"test raised {type(e).__name__}"
            kind = str(getattr(test.type, "value", test.type))
            by_type[kind][1] += 1
            if passed:
                by_type[kind][0] += 1
                passed_all += 1
            elif len(report["failures"]) < 2000:
                report["failures"].append({"subset": subset, "id": test.id, "type": kind, "why": why[:300]})
        report["subsets"][subset] = {
            "tests": len(tests),
            "passed": passed_all,
            "score": passed_all / len(tests) if tests else 0.0,
            "byType": {k: {"passed": p, "tests": n} for k, (p, n) in sorted(by_type.items())},
        }
    scores = [s["score"] for s in report["subsets"].values()]
    report["overall"] = sum(scores) / len(scores) if scores else 0.0
    return report


def main():
    if len(sys.argv) < 5:
        sys.exit("usage: olmocr_score.py <bench_data> <candidate> <report.json> <subset>...")
    bench_data, candidate, out, *subsets = sys.argv[1:]
    report = score(bench_data, candidate, subsets)
    with open(out, "w", encoding="utf-8") as f:
        json.dump(report, f, indent=2)
        f.write("\n")


if __name__ == "__main__":
    main()
