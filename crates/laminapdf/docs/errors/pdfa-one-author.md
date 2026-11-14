---
title: "pdfa-one-author"
version: "0.3.0"
url: "https://docs.laminapdf.com/reference/codes/pdfa-one-author/"
---

# pdfa-one-author

- Type: warning
- Kind: `usage` (the caller misused the API)

## What happened

The file says it conforms to PDF/A-1, which allows the metadata's author list (`dc:creator`) only one entry (ISO 19005-1 §6.7.3), and an edit gave it more than one. Saving it as it is writes a file that no longer meets PDF/A-1.

## Why

PDF/A-1 mirrors the Info dictionary's single `/Author` string in XMP, so it allows one name there. Later parts of PDF/A allow a list.

## How to fix

Keep one author, writing several names as one entry (`"Ada Lovelace and Charles Babbage"`), if the file must stay PDF/A-1. If it needn't, save anyway: the warning never blocks a save.

## Example

`doc.setMetadata({ authors: ["Ada Lovelace", "Charles Babbage"] })` on a PDF/A-1b file, then `doc.plan()`, lists `pdfa-one-author` with `authors` 2.

## Data fields

- `authors`: how many authors the metadata lists.
