# Signing the .rpm failed on the space in the package's name

[Resolution](resolution.md).

## What we saw

On 2026-11-15, a staging release ([run 43771064877](https://github.com/laudebugs/laminapdf/actions/runs/43771064877)) failed on both Linux legs in the step "Sign the .deb and the .rpm":

```
gpg: can't open 'PDF-0.0.133-1.x86_64.rpm.sig': No such file or directory
gpg: signing failed: No such file or directory
error: gpg exec failed (2)
```

The `.deb` was signed and the AppImage's updater signature written; only the `.rpm` failed. Nightlies build no `.rpm` (an rpm's version can't carry a nightly's), so this was the first run to sign one: the first staging run had stopped earlier, on the updater key.

## How to reproduce

In an `ubuntu:22.04` container (the release's Linux runners), with `rpm` and `gnupg` installed and a test key, build any `.rpm`, copy it to `Lamina PDF-0.0.1-1.x86_64.rpm` and run `rpmsign --addsign --define "_gpg_name <fingerprint>"` on it. rpm 4.17.0 fails with `gpg exec failed (2)`. On `ubuntu:24.04` (rpm 4.18) the same command signs.

## Cause

Tauri names the package after the product, "Lamina PDF", so the file name has a space. rpm 4.17's `__gpg_sign_cmd` macro passes the signature's path to gpg unquoted, so gpg reads `/…/Lamina` and `PDF-0.0.133-1.x86_64.rpm.sig` as two arguments. rpm 4.18 quotes it.
