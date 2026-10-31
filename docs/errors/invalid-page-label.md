# `invalid-page-label`

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

A `/PageLabels` entry is malformed; that part is dropped.

## Why

A writer bug: a range that is not a dictionary, an unknown `/S` style, a non-string `/P` prefix or an `/St` below 1.

## How to fix

Nothing; affected pages fall back to decimal labels.

## Example

```text
/PageLabels << /Nums [0 << /S /Fancy >> 4 << /S /r >>] >>
```

`/Fancy` is not a numbering style, so pages 0–3 get decimal labels; the range starting at 4 is kept. `data` is `{ "start": 0 }`.

## Data fields

`start`.
