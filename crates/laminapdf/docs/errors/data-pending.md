---
title: "data-pending"
version: "0.3.0"
url: "https://docs.laminapdf.com/reference/codes/data-pending/"
---

# data-pending

- Type: error
- Kind: `state` (the call isn't valid in the object's current state)

## What happened

The `ByteSource` hasn't fetched the listed byte ranges yet.

## Why

Range loading: the call needs more of the file.

## How to fix

Fetch the ranges and retry the call; the document stays usable.

## Example

```ts
const doc = await Document.openRange(httpRangeSource(url));
// pages() needs bytes that haven't arrived: ensure() fetches e.ranges and retries
const pages = await doc.ensure(() => doc.pages);
```

## Data fields

`ranges` (offset, length pairs).
