# `unknown-code`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

`laminapdf explain` was given a code that isn't one of Lamina PDF's error or warning codes.

## Why

Every code Lamina PDF reports is listed once, with its own page. A code that isn't listed has no explanation to print.

## How to fix

Check the spelling: `didYouMean` lists the known codes closest to the one given. The tables in this folder's README list every code.

## Example

```console
$ laminapdf explain xref-repaird
error[unknown-code]: not a Lamina PDF code
  = did you mean: xref-repaired
$ echo $?
2
```

## Data fields

- `code`: the code given (untrusted: caller text, never in the message).

## For contributors

The decisions and todos behind this code:

- [ADR 0031](../decisions/0031-cli-and-mcp-contract.md) §4, §5 and §8
- [L4a](../todos/done/L4a-lamina-cli.md): the `laminapdf` CLI and its first commands
