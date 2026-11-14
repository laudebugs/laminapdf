---
title: "input-unreadable"
version: "0.3.0"
url: "https://docs.laminapdf.com/reference/codes/input-unreadable/"
---

# input-unreadable

- Type: error
- Kind: `io` (the host failed to supply or take bytes)

## What happened

An input couldn't be read: the PDF named on the command line, standard input, or the file given to `--password-file`.

## Why

The file may not exist, may be a folder, or may not be readable by the user running the command.

## How to fix

Check that the file exists and that you can read it. `data.input` names the file, and `data.reason` says what the system reported.

## Example

```console
$ laminapdf info missing.pdf
error[input-unreadable]: couldn't read an input
  = hint: check that the file exists and that you can read it; data.input names it
$ echo $?
1
```

## Data fields

- `input`: the file that couldn't be read, or `-` for standard input (untrusted: caller text, never in the message).
- `reason`: the system's error kind, such as `NotFound` or `PermissionDenied`.
