# Hacker News research scripts

These scripts collected and counted the data behind [What PDF users ask for: ten years of Hacker News](../../docs/research/hacker-news/pdf-users-on-hacker-news.md). They use only the Python standard library (3.10 or later), call the public [HN Search API](https://hn.algolia.com/api) and are never run by tests or CI.

The data stays out of git: about 120 MB of other people's comments, which the API serves again on request. A rerun gives slightly different numbers, because the API's points and comment counts change and deleted comments drop out. The counts in the notes are from the collection made on 6 November 2026.

## Collect and prepare

Pick a folder for the data, outside the checkout, then run from the repository root:

```sh
python3 -I tools/hn-research/collect_stories.py "$HN/data"    # stories_raw.jsonl, stories.jsonl
python3 -I tools/hn-research/collect_comments.py "$HN/data"   # threads.jsonl, comment_hits.jsonl, comment_counts.json
python3 -I tools/hn-research/prep.py "$HN"                    # areas/<area>.jsonl, stats/tool_mentions_by_year.json, stats/area_volume_by_year.json
python3 -I tools/hn-research/story_stats.py "$HN"             # stats/story_volume.json and the story tables
```

Behind a proxy that re-signs TLS, set `SSL_CERT_FILE` to its CA bundle. [The method note](../../docs/research/hacker-news/notes/method.md) says what each step keeps and drops.

## Count

Each area's scripts read the data folder from `HN_DATA` (the generation scripts take it as their first argument instead) and print their tables. Most also print random matches for spot checks; the options are in each script's header.

| Area | Scripts | Note |
| --- | --- | --- |
| Readers | [`counts/readers/`](counts/readers/): `themes.py`, `tools.py`, `extras.py`, the hand labels in `sentiment_labels.json`, the quotes in `quotes.py` and `verify_quotes.py` | [readers](../../docs/research/hacker-news/notes/readers.md) |
| Editors | [`counts/editors/editors_counts.py`](counts/editors/editors_counts.py) | [editors](../../docs/research/hacker-news/notes/editors.md) |
| Extraction | [`counts/extraction/extraction_counts.py`](counts/extraction/extraction_counts.py); its output in `output.txt` | [extraction](../../docs/research/hacker-news/notes/extraction.md) |
| Generation | [`counts/generation/`](counts/generation/): `generation_counts.py` over the core set of `generation_core.py`, `generation_extra.py`, `generation_stories.py`, `generation_read_thread.py`; output in `output.txt` | [generation](../../docs/research/hacker-news/notes/generation.md) |
| Format, security and trust | [`counts/format-and-security/format_security_counts.py`](counts/format-and-security/format_security_counts.py) | [format and security](../../docs/research/hacker-news/notes/format-and-security.md) |

For example:

```sh
HN_DATA="$HN" python3 -I tools/hn-research/counts/extraction/extraction_counts.py
HN_DATA="$HN" python3 -I tools/hn-research/counts/extraction/extraction_counts.py --samples tables 12
python3 -I tools/hn-research/counts/generation/generation_counts.py "$HN"
```

## Check the quotes

Every quote line in the notes and the report cites a comment by ID. `verify_quotes.py` checks that each quote appears word for word in that comment, runs 40 words or fewer and carries the comment's date:

```sh
python3 -I tools/hn-research/verify_quotes.py "$HN" docs/research/hacker-news
```
