---
title: "path-outside-roots"
version: "0.2.0-alpha.6"
url: "https://docs.laminapdf.com/reference/codes/path-outside-roots/"
---

# path-outside-roots

- Type: error
- Kind: `security` (encryption, permissions or signatures block the call)

## What happened

A tool of `laminapdf mcp` was given a path the server may not use: one outside every `--root` folder it was started with, one that climbs out with `..`, or a symbolic link inside a root that leads out of it. A missing file outside the roots, and a link that leads nowhere, get this answer too, so a call can't tell whether a file outside the roots exists.

## Why

An MCP server acts for a model that may have read instructions planted in a document or a web page. So the server touches only the folders named on its command line by the person who started it. It resolves every path, links included, before it checks the path, so a link can't lead it elsewhere.

## How to fix

Use a path inside one of the server's `--root` folders; a relative path is read from the first root. To work on files elsewhere, start the server with another `--root`.

## Example

```json
{"schema":"laminapdf.error/1","error":{"code":"path-outside-roots","kind":"security","message":"the path is outside the server's --root folders","data":{"path":"../secret.pdf"},"exitCode":1,…}}
```

## Data fields

- `path`: the path the tool was given (untrusted: caller text, never in the message).
