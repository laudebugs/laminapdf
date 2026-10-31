# `outline-cycle`

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

An outline item was reached twice; that branch stopped.

## Why

Damaged or hostile outlines.

## How to fix

Nothing; the outline is shown without the loop.

## Example

```text
11 0 obj << /Title (Chapter 1) /Parent 10 0 R /Next 11 0 R >>
```

Item 11 is its own `/Next`; the outline walk stops that branch the second time it reaches it.

## Data fields

None.
