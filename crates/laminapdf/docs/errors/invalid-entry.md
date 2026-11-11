---
title: "invalid-entry"
version: "0.2.0-alpha.6"
url: "https://docs.laminapdf.com/reference/codes/invalid-entry/"
---

# invalid-entry

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

An entry a view reads is unreadable or of the wrong type; it reads as absent or its default.

## Why

A writer bug: an entry of the wrong type, or a reference that does not resolve.

## How to fix

Nothing to do; the view uses the entry's default.

## Example

```text
1 0 obj << /Type /Catalog /Pages 2 0 R /Outlines (none) >>
```

`/Outlines` should be a dictionary; the outline reads as empty and `data` is `{ "key": "Outlines" }`.

## Data fields

`key` (from Lamina PDF's fixed list).
