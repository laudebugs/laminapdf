# `content-unbalanced-marked`

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

An `EMC` had no matching `BMC`/`BDC` (it was ignored), or a content stream ended with marked-content sequences open (they were closed).

## Why

A producer bug, or tagging split across streams.

## How to fix

Check tagged-content results; the sequences were closed for you.

## Example

```text
/P << /MCID 0 >> BDC (text) Tj
```

The stream ends with the marked-content sequence open; it is closed at the end, so MCID 0 covers the text that follows it.

## Data fields

- `offset`: the `EMC`'s byte offset, or the end of the content.

The `location` is `object` (the page for page content, the Form XObject for a form) with `streamOffset` in its decoded content; none for content interpreted on its own or a direct page.

## For contributors

The decisions and todos behind this code:

- [ADR 0023](../decisions/0023-content-stream-reading.md)
