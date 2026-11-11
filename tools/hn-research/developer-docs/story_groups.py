"""Group the CORE stories by title (first matching rule wins) and count threads and comments per group.
Usage: python3 -I story_groups.py <workdir>"""
import json, re, sys
from collections import defaultdict
from pathlib import Path
H = Path(sys.argv[1])
S = [json.loads(l) for l in open(H / "data/stories.jsonl") if json.loads(l)["set"] == "core"]
C = defaultdict(int)
for l in open(H / "data/threads.jsonl"):
    C[json.loads(l)["story_id"]] += 1
G = [
 ("AI, LLMs, agents, llms.txt", r"llm|\bai\b|gpt|copilot|agents?\.md|agent|claude|mcp|rag\b|doc bot|chatgpt|nia|sosumi|promptless|autodoc|machines"),
 ("MDN", r"mdn|mozilla|web docs|documenting the web|yari"),
 ("Apple developer docs", r"apple|visionos"),
 ("Man pages, tldr, info", r"man ?pages?|manpages|\bum\b|tldr|strftime|man page|unix by reading|hyphens"),
 ("Best/worst docs (Ask HN and rants)", r"best|worst|crappiest|good documentation|example of good|terrible|piss-poor|is fucked|missing|wrong|is not$|unclear"),
 ("Docs tooling and formats", r"docusaurus|sphinx|mkdocs|gitbook|mintlify|hugo|markdown|restructuredtext|asciidoc|docs[- ]as[- ]code|docs like code|writerside|zensical|scalar|theneo|developerhub|tract|zero-config|fastapi|fastopenapi|read the docs|devdocs|zeal|javadoc|jep|rustdoc|java packages|clojuredocs|livebook|notebook|playground|in the repo"),
 ("Writing and structuring docs", r"writing|write|technical|four kinds|di[aá]taxis|grand unified|framework|graph|tree-based|simple|simply|half|small docs|examples|screenshots|judge|must|continuous|first|driven|superstition|respect|organize|level up|upskill|books|courses|style guide|readme|programmers don|free software|api is only"),
]
groups = defaultdict(list)
for s in S:
    t = s["title"]
    g = next((n for n, rx in G if re.search(rx, t, re.I)), "Other (single projects, platforms)")
    groups[g].append(s)
tot = sum(C[s["objectID"]] for s in S)
for g, ss in sorted(groups.items(), key=lambda kv: -sum(C[s["objectID"]] for s in kv[1])):
    n = sum(C[s["objectID"]] for s in ss); late = sum(1 for s in ss if s["created_at"] >= "2023")
    print(f"{g:40} threads {len(ss):3} (2023+: {late:3})  comments {n:5} ({100*n/tot:4.1f}%)")
    if "--list" in sys.argv:
        for s in ss: print("     ", s["objectID"], s["created_at"][:10], C[s["objectID"]], s["title"][:80])
