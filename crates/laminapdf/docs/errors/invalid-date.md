---
title: "invalid-date"
version: "0.2.0-alpha.5"
url: "https://docs.laminapdf.com/reference/codes/invalid-date/"
---

# invalid-date

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

An Info date is not a valid PDF date (ISO 32000-2 §7.9.4); the raw string is kept.

## Why

Writers that write free-form dates.

## How to fix

The `Info` view keeps the raw string; read it there if you need it.

## Example

```text
<< /CreationDate (last Tuesday) >>
```

The Info view's `creation_date` is `None` and its raw string is kept; `data` is `{ "key": "CreationDate" }`.

## Data fields

`key` (which entry).
