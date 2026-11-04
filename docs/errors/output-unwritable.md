# `output-unwritable`

- Type: error
- Kind: `io` (the host failed to supply or take bytes)

## What happened

The output couldn't be written: its folder is missing or read-only, the disk is full, or standard output was closed before the command finished.

## Why

The CLI writes to a temporary file beside the output and renames it into place, so a failed write leaves the old file, if there was one, as it was.

## How to fix

Check that the output's folder exists, that you can write to it and that the disk has room. When piping, make sure the reading command doesn't exit early.

## Example

```console
$ laminapdf pages rotate scan.pdf +90 -o /no/such/folder/out.pdf
error[output-unwritable]: couldn't write the output
  = hint: check that the output's folder exists, …
$ echo $?
1
```

## Data fields

- `output`: the file that couldn't be written, or `-` for standard output (untrusted: caller text, never in the message).
- `reason`: the system's error kind, such as `NotFound` or `StorageFull`.

## For contributors

The decisions and todos behind this code:

- [ADR 0031](../decisions/0031-cli-and-mcp-contract.md) §4, §5 and §8
- [L4a](../todos/done/L4a-lamina-cli.md): the `laminapdf` CLI and its first commands
