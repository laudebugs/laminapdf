# Hacker News on developer documentation

The scripts behind [the developer docs research](../../../docs/research/docs-site/developer-docs.md) and its [Hacker News note](../../../docs/research/docs-site/developer-docs/hacker_news.md). They collect stories about developer documentation from the HN Search API, keep the ones about docs, fetch their comments, count comments per theme and check every quoted comment against the API again. They are stdlib Python, run by hand, never by tests.

The order: `collect_stories.py`, `select.py`, `collect_comments.py`, then `themes.py`, `strict.py`, `story_groups.py` and `nominations.py` for the counts, and `verify.py` for the quotes. `grep.py` searches the comments for a phrase.

`data/` holds the selected stories (`stories.jsonl`) and the printed outputs the notes cite. The raw search results and the 13 MB of comment threads stay out of git, as in the [earlier study](../README.md); the scripts were written to run from a scratch folder, so set their input and output paths before running them again.
