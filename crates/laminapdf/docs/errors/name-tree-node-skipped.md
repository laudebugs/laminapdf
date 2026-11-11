---
title: "name-tree-node-skipped"
version: "0.2.0-alpha.6"
url: "https://docs.laminapdf.com/reference/codes/name-tree-node-skipped/"
---

# name-tree-node-skipped

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

A name- or number-tree node was malformed, reached twice or too deep, and was skipped.

## Why

Damaged trees: a node that is not a dictionary, a kid reached twice (a cycle), or a tree deeper than `max_tree_depth`.

## How to fix

Nothing to do; the entries under the skipped node are missing from the tree.

## Example

```text
<< /Kids [20 0 R 21 0 R] >>   % 21 0 obj is (oops), not a dictionary
```

The walk of the `/Dests` name tree skips node 21 and keeps the entries under node 20.

## Data fields

None.
