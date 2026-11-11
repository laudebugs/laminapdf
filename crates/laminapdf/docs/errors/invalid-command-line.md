---
title: "invalid-command-line"
version: "0.2.0-alpha.6"
url: "https://docs.laminapdf.com/reference/codes/invalid-command-line/"
---

# invalid-command-line

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
error[invalid-command-line]: unexpected argument '--pages' found

Usage: laminapdf info [OPTIONS] <INPUT>

For more information, try '--help'.
$ echo $?
2
```

With `--json`, the line on standard error has the message "the command line doesn't parse" and the parser's finding in `data.detail`.

## Data fields

Which field is there depends on what was wrong:

- `detail`: when the parser refused the line, the kind of problem it found, such as `UnknownArgument` or `MissingRequiredArgument`.
- `argument`: when a value parsed but means nothing, the value given: the angle of `pages rotate`, a `help` topic or a `--fields` key (untrusted: caller text, never in the message).
