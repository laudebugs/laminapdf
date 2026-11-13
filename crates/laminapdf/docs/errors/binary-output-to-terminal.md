---
title: "binary-output-to-terminal"
version: "0.2.0"
url: "https://docs.laminapdf.com/reference/codes/binary-output-to-terminal/"
---

# binary-output-to-terminal

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

A command that writes a PDF was told to write it to standard output (`-o -`), and standard output is a terminal.

## Why

A PDF's bytes would fill the terminal with noise and can leave it in a strange state. The CLI writes binary output only to a file or a pipe.

## How to fix

Redirect standard output to a file or a pipe, or write to a file with `-o`.

## Example

```console
$ laminapdf pages rotate scan.pdf +90 -o -
error[binary-output-to-terminal]: standard output is a terminal
  = hint: redirect standard output to a file or a pipe, or write to a file with -o
$ laminapdf pages rotate scan.pdf +90 -o - > rotated.pdf
```

## Data fields

None.
