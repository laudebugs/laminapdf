---
title: "text-in-earlier-revision"
version: "0.4.0"
url: "https://docs.laminapdf.com/reference/codes/text-in-earlier-revision/"
---

# text-in-earlier-revision

- Type: warning
- Kind: `security` (encryption, permissions or signatures block the call, or look tampered with)

## What happened

The file was saved with incremental updates, and an earlier revision holds words that the latest revision has nowhere. Anyone can cut the file back to that revision and read them.

## Why

An incremental save appends the changes to the end of the file and keeps everything before them (ISO 32000-2 §7.5.6). Deleting a paragraph and saving that way leaves the paragraph in the bytes of the earlier revision. Signing, filling a form and some editors' plain "Save" all save incrementally.

Not every removed word is a leak: fixing a typo removes the misspelt word too. The warning states that the earlier text is still in the file; whether that matters depends on what was removed.

## How to fix

Save the file as a full rewrite, which keeps only the latest revision. A signed file loses its signatures that way, so sign it again afterwards.

## Example

`fixtures/corpus/hello-incremental.pdf` first draws "Hello, World", and its update replaces that with "Hello, Update". `laminapdf check` reports `text-in-earlier-revision` with `revision` 0, `words` 1 and `pages` `[0]`.

## Data fields

- `revision`: the earlier revision's index, 0 for the original file.
- `words`: how many words that revision has more often than the latest one (only words of two or more letters or digits count).
- `pages`: the indices of that revision's pages where those words are.

The warning has no `location`, and it never holds the words themselves.
