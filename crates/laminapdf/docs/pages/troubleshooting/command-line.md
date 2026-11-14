---
title: "Troubleshooting the command line"
description: "Fixes for the laminapdf tool when a script stops on exit 3, a file already exists, a password is needed or a PDF would go to the terminal."
version: "0.3.0"
url: "https://docs.laminapdf.com/troubleshooting/command-line/"
---

# Troubleshooting the command line

Fixes for the laminapdf tool when a script stops on exit 3, a file already exists, a password is needed or a PDF would go to the terminal.

These entries cover the `laminapdf` tool. Most start from an exit code, because that is what a script sees first. Each entry starts from what you see, then gives the cause and the fix. `laminapdf help exit-codes` prints every exit code, and [Use laminapdf in scripts](../../pages/cli/scripts.md#exit-codes) explains them.

## A script stops on exit 3, though the command worked

**Cause:** exit 3 means the command succeeded but raised warnings, such as a file that needed repair. A shell with `set -e` stops on any exit code other than 0, so it treats the warnings as a failure.

**Fix:** read the warnings on standard error, or in the report's `warnings` with `--json`. If they don't matter to your script, pass `--warnings-ok`, which makes the same run exit 0.

## The command exits 4 with `overwrite-refused`

**Cause:** the output file already exists, and `laminapdf` replaces a file only when you pass `--force`. It checks every output before it writes any, so nothing was written.

**Fix:** pass `--force` to replace the file, or write to a new name. `--dry-run` shows what a command would write without writing it. See [`overwrite-refused`](../../errors/overwrite-refused.md).

## The command exits 5 for an encrypted file

**Cause:** the file needs a password, and the command had none, or the wrong one. The error is `password-required` or `wrong-password`. `laminapdf` never takes a password as an argument, because other users of the machine can see a command's arguments.

**Fix:** put the password in a file and pass `--password-file FILE`, or set the `LAMINA_PDF_PASSWORD` environment variable. See [Passwords and limits](../../pages/cli/scripts.md#passwords-and-limits) and [`password-required`](../../errors/password-required.md).

## The command exits 2 with `binary-output-to-terminal`

**Cause:** the command would write a PDF or an image to standard output, and standard output is your terminal. `laminapdf` refuses rather than fill the screen with binary.

**Fix:** name an output file with `-o`, or send standard output to a file or another command: `laminapdf render in.pdf --pages 1 -o - > page-1.png`. See [`binary-output-to-terminal`](../../errors/binary-output-to-terminal.md).

## The command exits 2 with `invalid-page-range`

**Cause:** the range you gave doesn't parse, or it names a page the document lacks, such as page 9 of a 5-page file. Page ranges count from 1.

**Fix:** check the range against the [page-range grammar](../../pages/cli/scripts.md#page-ranges): `3-7`, `z` for the last page, `r2` for the second from the end, and `x3` for every page but page 3. See [`invalid-page-range`](../../errors/invalid-page-range.md).

Next: [opening and saving files](../../pages/troubleshooting/files.md), for signed files and files over a limit.
