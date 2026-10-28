# X1 — Docs link checker and todo-board check in `cargo xtask`, run in CI

| Field | Value |
| --- | --- |
| ID | `X1` |
| Status | `ready` |
| Plan | [architecture §7 Cross-cutting](../../plans/architecture.md#cross-cutting) |
| Round / milestone | Cross-cutting (any time; recommended first) |
| Depends on | None |
| Blocks | None |
| ADRs | Follows [0015](../../decisions/0015-monorepo-layout-and-tooling.md) (xtask is the single entry point; CI runs what xtask runs) |
| Branch | `build/x1-docs-link-checker` |
| PR title | `build(xtask): check relative links and anchors in every Markdown file` |
| Size | M (≈ 250–450 lines incl. tests) |
| PR | — |

## Goal

Catch broken documentation links before merge. The todo board makes link rot likely: every todo moves `ready/` → `ongoing/` → `done/`, and plans, the board README and other todos link to it by path. `cargo xtask docs-links` checks every relative link and `#anchor` in every tracked `*.md` file (GitHub's anchor slug rules, including duplicate-heading suffixes), and runs in the CI fast lane.

## Context

- [Todo board](../README.md) and [git workflow §2 "Pick work from the todo board"](../../guidelines/git-workflow.md#pick-work-from-the-todo-board): when files move and which links must be updated.
- [ADR 0015](../../decisions/0015-monorepo-layout-and-tooling.md): xtask conventions; [code organisation §10 scripts](../../guidelines/code-organization.md#10-scripts).
- A throwaway Python checker was used to write the todos; this replaces it with a tested Rust implementation.

## Scope

**In scope**

- `tools/xtask/src/docs_links.rs`: walk tracked Markdown (`git ls-files '*.md'`, excluding `spikes/*/target`, `node_modules`), parse links with `pulldown-cmark` (inline links, reference links, images; skip code spans/blocks and autolinks/absolute URLs), resolve relative paths against the file, check the target exists (file or directory), and for `#anchor` on `.md` targets compute GitHub-style slugs (lowercase; remove punctuation except `-` and `_`; spaces → `-`; Unicode letters kept; inline code/links reduced to text; duplicate headings get `-1`, `-2`, …) and check the anchor exists; same-file `#anchor` links too; explicit HTML anchors (`<a id="…">`) supported.
- Output: `path:line: broken link <target> (missing file | missing anchor)`; non-zero exit on any error; `--json` for tooling.
- `cargo xtask check` runs it (fast lane); documented in `CLAUDE.md`.
- Optional (flag): warn on links to `docs/todos/<state>/X.md` whose file lives in another state folder, suggesting the right path.

**Out of scope**

- External URL checking (network; flaky): maybe a nightly job later ([X3a](./X3a-nightly-lane.md) can add `lychee` if wanted).
- The docs site's own checker: [B7a](./B7a-docs-site.md).

## Tasks

- [ ] Add `pulldown-cmark` to xtask deps (licence check)
- [ ] Implement `docs_links.rs` with a slugger module and unit tests (cases: `## 7. Encryption (§7.6)` → `7-encryption-76`, `Round A — Foundations` → `round-a--foundations`, `B6. WASM and @lamina/pdf` → `b6-wasm-and-laminapdf`, duplicates, emoji, inline code)
- [ ] Wire `docs-links` into `main.rs`, `check`, and `ci`
- [ ] Fix any existing broken links found on `main` in the same PR
- [ ] Document in `CLAUDE.md` commands and the todo README
- [ ] Add `cargo xtask todos` (same xtask crate, no new runtime deps beyond what's allowed): parse every `docs/todos/{ready,ongoing,done}/*.md` header table, check that **Status** matches the folder, that every **Depends on** ID exists, that the graph has no cycles, that **Blocks** is exactly the reverse of **Depends on**, and that no `done` todo depends on an unfinished one; regenerate `docs/todos/README.md` (the board: "Can start now", ongoing, one table per plan). `cargo xtask todos --check` fails CI when the board or any Blocks field is stale. Port the logic of the one-off script used to build the board in PR "docs(todos)…" (normalise IDs, reverse edges, cycle check).

## Acceptance criteria

- [ ] `cargo xtask docs-links` exits 0 on `main` after this PR's fixes, and non-zero with a precise message for a broken file link and a broken anchor (integration test on a temp tree)
- [ ] Slug unit tests cover every heading style used in `docs/` (agreement with GitHub verified by hand for ten real headings, listed in the PR)
- [ ] Runs in the fast lane in under 5 seconds on the repository
- [ ] `cargo xtask todos --check` passes on main and fails on: a todo in the wrong folder for its Status, an unknown dependency ID, a dependency cycle, a stale Blocks field, a stale board.

## Tests

Unit tests for the slugger and link extraction; an integration test building a temp directory of Markdown files.

## Docs to update

- [ ] [architecture §7 Cross-cutting](../../plans/architecture.md#cross-cutting): tick the link-checker item
- [ ] `CLAUDE.md` (commands), [todo board README](../README.md) (mention the check)

## Risks and open questions

- GitHub's slug algorithm has edge cases (emoji, some punctuation); keep the implementation matching observed behaviour and add cases as they appear.

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied

## Appendix: reference board generator

The one-off Python script that normalised dependencies and generated `docs/todos/README.md` when the board was created. Port its logic to `cargo xtask todos`; don't add the script to the repo (ADR 0015: xtask is the single entry point).

<details><summary><code>todo_graph.py</code></summary>

```python
"""Normalise todo dependencies, recompute Blocks, check the graph, write the board.

Usage: python3 -I todo_graph.py <repo-root> [--write]
"""
import os, re, sys
from collections import defaultdict

root = sys.argv[1]
write = "--write" in sys.argv
T = os.path.join(root, "docs", "todos")
STATES = ["ready", "ongoing", "done"]
ID_RE = r"(?<![A-Za-z0-9#/-])([A-EHPX]\d+[a-z]?)(?![A-Za-z0-9])"

todos = {}
for st in STATES:
    for f in sorted(os.listdir(os.path.join(T, st))):
        if not f.endswith(".md"):
            continue
        p = os.path.join(T, st, f)
        s = open(p).read()
        m = re.search(r"^\| ID \| `([^`]+)` \|", s, re.M)
        tid = m.group(1) if m else f.split("-")[0]
        title = re.search(r"^# (.+)$", s, re.M).group(1)
        title = re.sub(r"^\S+ — ", "", title)
        todos[tid] = dict(id=tid, state=st, file=f, path=p, text=s, title=title)

ids = set(todos)

def members(prefix):
    return sorted(i for i in ids if re.fullmatch(prefix + r"[a-z]", i))

# Round -> todos that complete it (default when a todo names a whole round).
GENERIC = {
    "B2": ["B2b", "B2c"], "B3": ["B3c"], "B4": ["B4b", "B4c"], "B5": ["B5c"],
    "B6": ["B6b"], "B7": ["B7b"], "B8": ["B8b"], "B9": ["B9c"],
    "C1": ["C1b"], "C2": ["C2c"], "C3": ["C3c"], "C4": ["C4c", "C4d"],
    "C5": ["C5b"], "C6": ["C6b", "C6c"], "C7": ["C7c"],
    "D1": ["D1a", "D1b"], "D2": ["D2e"], "D3": ["D3b"], "D4": ["D4c"],
    "D5": ["D5b"], "D6": ["D6c", "D6d"], "D7": ["D7c"], "X3": ["X3a"],
}
# Precise targets given by the plan owners, per depending todo.
OVERRIDE = {
    "C1b": {"B6": ["B6b"]}, "C2c": {"B6": ["B6b"]}, "C3a": {"B6": ["B6b"]},
    "C2b": {"B2": ["B2c"]}, "C6a": {"B2": ["B2c"], "B6": ["B6b"]},
    "C4c": {"B2": ["B2b"], "B6": ["B6b"]},
    "D1a": {"C3": ["C3b"]}, "D1b": {"C3": ["C3b"]},
    "D2a": {"C4": ["C4a"], "C2": ["C2a"]}, "D2b": {"C2": ["C2b"]},
    "D2d": {"C3": ["C3b"], "C4": ["C4a"]}, "D4b": {"C7": ["C7a"]},
    "D4c": {"C6": ["C6c"]}, "D5a": {"C4": ["C4d"], "C2": ["C2a"]},
    "D7b": {"C4": ["C4b", "C4d"]},
    "E1d": {"C4": ["C4a"]}, "E10e": {"C4": ["C4a"]},
    "E11b": {"B2": ["B2b"]},
    "H1": {"C4": ["C4d"], "C2": ["C2b"], "D2": ["D2e"]},
    "E3h": {"C2": ["C2b"]}, "E13a": {"C2": ["C2b"]},
}

# Todos whose Depends-on prose mentions IDs that are context, not prerequisites.
EXACT = {"B9a": ["B2c"]}

def field(s, name):
    m = re.search(r"^\| " + re.escape(name) + r" \| (.*) \|$", s, re.M)
    return m.group(1) if m else None

problems, notes_needed = [], {}
deps = {}
for tid, t in todos.items():
    raw = field(t["text"], "Depends on") or ""
    found, resolved_any = [], False
    # "can run in parallel with X" is not a dependency.
    scan = re.sub(r"[Rr]uns? in parallel with[^.|;]*|[Cc]an run in parallel with[^.|;]*|in parallel with[^.|;]*", "", raw)
    own_round = re.match(r"[A-Z]\d+", tid).group(0)
    for ref in ([] if tid in EXACT else re.findall(ID_RE, scan)) + EXACT.get(tid, []):
        if ref == tid or ref == own_round:
            continue
        if ref in ids:
            found.append(ref)
        elif ref in GENERIC or members(ref):
            tgt = OVERRIDE.get(tid, {}).get(ref) or GENERIC.get(ref) or members(ref)
            found += tgt
            resolved_any = True
        else:
            problems.append(f"{tid}: unknown dependency id {ref!r}")
    seen, out = set(), []
    for d in found:
        if d not in seen:
            seen.add(d); out.append(d)
    deps[tid] = out
    if resolved_any:
        notes_needed[tid] = raw

# cycle check
color = {}
def visit(n, stack):
    color[n] = 1
    for d in deps.get(n, []):
        if color.get(d) == 1:
            problems.append("cycle: " + " -> ".join(stack + [n, d]))
        elif not color.get(d):
            visit(d, stack + [n])
    color[n] = 2
for n in todos:
    if not color.get(n):
        visit(n, [])

# state sanity: done todos may only depend on done ones
for tid, t in todos.items():
    for d in deps[tid]:
        if t["state"] == "done" and todos[d]["state"] != "done":
            problems.append(f"{tid} is done but depends on {d} ({todos[d]['state']})")

blocks = defaultdict(list)
for tid, ds in deps.items():
    for d in ds:
        blocks[d].append(tid)

def order_key(i):
    m = re.match(r"([A-Z])(\d+)([a-z]?)", i)
    return ("ABCDEHPX".index(m.group(1)), int(m.group(2)), m.group(3))

def link(frm, to):
    a, b = todos[frm], todos[to]
    rel = f"./{b['file']}" if a["state"] == b["state"] else f"../{b['state']}/{b['file']}"
    return f"[{to}]({rel})"

for tid, t in todos.items():
    s = t["text"]
    dl = ", ".join(link(tid, d) for d in sorted(deps[tid], key=order_key)) or "None"
    bl = ", ".join(link(tid, b) for b in sorted(blocks[tid], key=order_key)) or "None"
    s = re.sub(r"^\| Depends on \| .* \|$", lambda _: f"| Depends on | {dl} |", s, count=1, flags=re.M)
    s = re.sub(r"^\| Blocks \| .* \|$", lambda _: f"| Blocks | {bl} |", s, count=1, flags=re.M)
    if tid in notes_needed and "| Dependency notes |" not in s:
        note = notes_needed[tid].replace("|", "\\|")
        s = re.sub(r"^(\| Blocks \| .* \|)$", lambda m: m.group(1) + f"\n| Dependency notes | {note} |", s, count=1, flags=re.M)
    t["new"] = s

# Board
plan_of = {"A": "architecture", "B": "pdf-library", "C": "web-app", "D": "desktop-app",
           "E": "full-editor", "H": "full-editor", "P": "architecture", "X": "architecture"}
section = {"A": "Architecture — Round A", "B": "Library — Round B", "C": "Web reader — Round C",
           "D": "Desktop reader — Round D", "E": "Editor — Rounds E1–E18", "H": "Hosted service",
           "P": "Publishing (later)", "X": "Cross-cutting"}
lines = ["# Todo board", "",
         "One file per pull request of work. A todo lives in exactly one state folder:", "",
         "| Folder | Meaning |", "| --- | --- |",
         "| [`ready/`](ready/) | Specified and waiting. Start one only when every todo in its **Depends on** is in `done/`. |",
         "| [`ongoing/`](ongoing/) | A PR is open or being written. Moved here in the PR's first commit. |",
         "| [`done/`](done/) | Merged. Moved here by the PR before it merges. |", "",
         "New todos start from [`TEMPLATE.md`](TEMPLATE.md). The workflow is in [git workflow §2](../guidelines/git-workflow.md#pick-work-from-the-todo-board); plans link to their todos.", "",
         "IDs: `A`/`P`/`X` [architecture](../plans/architecture.md) (foundations, publishing, cross-cutting), `B` [library](../plans/pdf-library.md), `C` [web reader](../plans/web-app.md), `D` [desktop reader](../plans/desktop-app.md), `E` [editor](../plans/full-editor.md), `H` hosted service ([full-editor](../plans/full-editor.md)). A letter suffix (`B2a`, `B2b`) splits a round into PRs.", ""]
counts = {st: sum(1 for t in todos.values() if t["state"] == st) for st in STATES}
lines += [f"**{counts['ongoing']} ongoing · {counts['ready']} ready · {counts['done']} done** ({len(todos)} total).", ""]
startable = sorted((i for i, t in todos.items() if t["state"] == "ready"
                    and all(todos[d]["state"] == "done" for d in deps[i])), key=order_key)
lines += ["## Can start now", "", "Ready todos whose dependencies are all done:", ""]
lines += [f"- [{i}]({todos[i]['state']}/{todos[i]['file']}): {todos[i]['title']}" for i in startable] + [""]
lines += ["## Ongoing", ""]
lines += [f"- [{i}](ongoing/{t['file']}): {t['title']}" for i, t in sorted(todos.items(), key=lambda kv: order_key(kv[0])) if t["state"] == "ongoing"] + [""]
for letter in "ABCDHEPX":
    group = sorted((i for i in todos if i[0] == letter), key=order_key)
    if not group:
        continue
    lines += [f"## {section[letter]}", "", "| ID | Todo | State | Depends on |", "| --- | --- | --- | --- |"]
    for i in group:
        t = todos[i]
        dl = ", ".join(f"[{d}]({todos[d]['state']}/{todos[d]['file']})" for d in sorted(deps[i], key=order_key)) or "—"
        lines.append(f"| {i} | [{t['title']}]({t['state']}/{t['file']}) | {t['state']} | {dl} |")
    lines.append("")
board = "\n".join(lines)

print(f"todos: {len(todos)} {counts}")
print(f"startable now: {', '.join(startable)}")
print(f"resolved round refs in {len(notes_needed)} todos")
print("problems:", len(problems))
for p in problems[:50]:
    print("  ", p)
if write and not problems:
    for t in todos.values():
        if t["new"] != t["text"]:
            open(t["path"], "w").write(t["new"])
    open(os.path.join(T, "README.md"), "w").write(board + "\n")
    print("written")
```

</details>
