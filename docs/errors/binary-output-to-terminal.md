# `binary-output-to-terminal`

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

## For contributors

The decisions and todos behind this code:

- [ADR 0031](../decisions/0031-cli-and-mcp-contract.md) §4, §5 and §8
- [L4a](../todos/done/L4a-lamina-cli.md): the `laminapdf` CLI and its first commands
