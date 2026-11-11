---
title: "source-changed"
version: "0.2.0-alpha.6"
url: "https://docs.laminapdf.com/reference/codes/source-changed/"
---

# source-changed

- Type: error
- Kind: `io` (the host failed to supply bytes; often transient)

## What happened

A remote file changed while `@laminapdf/sdk` was reading it range by range: a response's `ETag` or total length differs from the first one (or the server ignored `If-Range` and sent a different file).

## Why

A range document is assembled from many responses. If the file changed between them, the parts no longer belong to one file, and the document would read a mix of two versions. `httpRangeSource` sends `If-Range` with the first `ETag` and compares each response's `ETag` and `Content-Range` length with the first.

## How to fix

Open the file again (a new source, a new document). If the file changes often, download it whole with `Document.open(bytes)` instead.

## Example

```js
const doc = await Document.openRange(httpRangeSource(url));
await doc.ensure(() => doc.pages); // throws { code: "source-changed", … } if the file was replaced meanwhile
```

## Data fields

None.
