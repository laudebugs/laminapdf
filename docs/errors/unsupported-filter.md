# `unsupported-filter`

- Type: error
- Kind: `unsupported` (valid input that lamina doesn't implement)

## What happened

A stream uses a filter lamina doesn't implement; only decoding that stream fails.

## Why

Rare or proprietary filters.

## How to fix

Use a tool that supports the filter.

## Example

```text
5 0 obj << /Filter /JBIG2Decode /Length 2048 >> stream …
```

Decoding this stream fails; the rest of the document reads normally. `data.filter` is the filter's name as read from the file.

## Data fields

`filter` (**untrusted**: read from the file). The message never includes it.
