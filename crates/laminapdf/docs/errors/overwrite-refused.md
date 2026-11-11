---
title: "overwrite-refused"
version: "0.2.0-alpha.6"
url: "https://docs.laminapdf.com/reference/codes/overwrite-refused/"
---

# overwrite-refused

- Type: error
- Kind: `state` (the call isn't valid in the object's current state)

## What happened

The output file already exists, or the command was asked to change its input in place with `--in-place`, and `--force` wasn't given.

## Why

The CLI never replaces a file without being told to, so a mistyped `-o` can't destroy a document. It never prompts either, because scripts and agents can't answer a prompt. Exit code 4 tells a script the command needs `--force`.

## How to fix

Pass `--force` to replace the file, or write to a new file with `-o`. The CLI writes to a temporary file and renames it, so a replaced file is never left half written.

## Example

```console
$ laminapdf pages rotate scan.pdf 90:z --in-place
error[overwrite-refused]: refusing to replace a file without --force
  = hint: pass --force, or write to a new file with -o
$ echo $?
4
```

## Data fields

- `output`: the file that would have been replaced (untrusted: caller text, never in the message).
