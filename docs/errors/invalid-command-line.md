# `invalid-command-line`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

The command line didn't parse: a command or flag `laminapdf` doesn't have, a missing argument, a value of the wrong form, or `-` (standard input) named as more than one input.

## Why

The CLI checks its whole command line before it reads any file, so a typo never runs half a command. Exit code 2 tells a script that the call itself was wrong, not the document.

## How to fix

Run the command with `--help` to see its arguments, and `laminapdf --help` for the commands. Read standard input at most once per command.

## Example

```console
$ laminapdf info --pages 1 report.pdf
error[invalid-command-line]: the command line doesn't parse
  = hint: run the command with --help to see its arguments
$ echo $?
2
```

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [ADR 0031](../decisions/0031-cli-and-mcp-contract.md) §4, §5 and §8
- [L4a](../todos/done/L4a-lamina-cli.md): the `laminapdf` CLI and its first commands
