"""Scores Markdown files against olmOCR-bench's unit tests, with the benchmark's own test code.

`cargo xtask extract-bench` writes one Markdown file per PDF (`<candidate>/<pdf stem>_pg<page>_repeat1.md`)
and runs this script, which loads each subset's tests (`olmocr.bench.tests.load_tests`, pinned in
requirements.txt) and runs every test on its file. Like olmOCR's `olmocr.bench.benchmark`, it adds a
`baseline` test for every PDF that has none (the page gave some text, and no long run of repeats)
as a group of its own, and the overall score is the mean of the groups' pass rates, the baseline
group included. Unlike it, a missing Markdown file fails its tests rather than the whole run, since
xtask writes an empty file for a PDF that failed; a run of the subsets scored here is the
benchmark's `--dir` run over the same subsets.

    python -I tools/bench/olmocr_score.py <bench_data> <candidate> <report.json> <subset>...

Never run by tests (it needs the dataset and the pinned packages).
"""

import json
import os
import sys
from collections import defaultdict


def score(bench_data, candidate, subsets):
    from olmocr.bench.tests import BaselineTest, load_tests

    groups = {subset: load_tests(os.path.join(bench_data, f"{subset}.jsonl")) for subset in subsets}
    has_baseline = {t.pdf for tests in groups.values() for t in tests if str(getattr(t.type, "value", t.type)) == "baseline"}
    pdfs = sorted({t.pdf for tests in groups.values() for t in tests} - has_baseline)
    groups["baseline"] = [BaselineTest(id=f"{pdf}_baseline", pdf=pdf, page=1, type="baseline") for pdf in pdfs]

    report = {"subsets": {}, "failures": []}
    for subset, tests in groups.items():
        by_type = defaultdict(lambda: [0, 0])
        by_pdf = defaultdict(lambda: [0, 0])
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
            by_pdf[test.pdf][1] += 1
            if passed:
                by_type[kind][0] += 1
                by_pdf[test.pdf][0] += 1
                passed_all += 1
            elif len(report["failures"]) < 2000:
                report["failures"].append({"subset": subset, "id": test.id, "type": kind, "why": why[:300]})
        report["subsets"][subset] = {
            "tests": len(tests),
            "passed": passed_all,
            "score": passed_all / len(tests) if tests else 0.0,
            "byType": {k: {"passed": p, "tests": n} for k, (p, n) in sorted(by_type.items())},
            "byPdf": {k: {"passed": p, "tests": n} for k, (p, n) in sorted(by_pdf.items())},
        }
    scores = [s["score"] for s in report["subsets"].values()]
    report["overall"] = sum(scores) / len(scores) if scores else 0.0
    without = [s["score"] for name, s in report["subsets"].items() if name != "baseline"]
    report["overallWithoutBaseline"] = sum(without) / len(without) if without else 0.0
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
