---
title: "sink"
version: "0.2.0-alpha.4"
url: "https://docs.laminapdf.com/reference/codes/sink/"
---

# sink

- Type: error
- Kind: `io` (the host failed to supply or take bytes)

## What happened

The `WriteSink` failed to take the bytes, or could not give them back. (A sink that already held bytes is [`sink-not-empty`](sink-not-empty.md).)

## Why

The destination failed: a full disk, a closed upload, a refused read-back.

## How to fix

Check the destination and retry the save. The document is unchanged; the sink may hold a prefix of the output.

## Example

An upload sink whose connection closed halfway through: the save fails with `sink`, the document is unchanged, and `data.kind` is the I/O error kind (`BrokenPipe`).

## Data fields

- `kind`: the I/O error kind, for a writer that failed.
- `message`: the sink's or writer's own text (**untrusted**: it comes from the host and may name a path; never in the message).
