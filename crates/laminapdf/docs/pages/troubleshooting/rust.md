---
title: "Troubleshooting Rust"
description: "Fixes for the laminapdf crate when Cargo picks the wrong version, a feature is missing, or your Rust is too old."
version: "0.4.0"
url: "https://docs.laminapdf.com/troubleshooting/rust/"
---

# Troubleshooting Rust

Fixes for the laminapdf crate when Cargo picks the wrong version, a feature is missing, or your Rust is too old.

These entries cover the `laminapdf` crate: the version Cargo picks, the features that bring each part of the API, and the Rust version it needs. Each entry starts from what you see, then gives the cause and the fix.

## The `laminapdf` crate has nothing in it

**Cause:** your project is on version `0.0.0`, a placeholder that only reserves the name. Every release before `0.2.0` was an alpha, and Cargo never picks an alpha unless you name it, so a plain `cargo add laminapdf` picked the placeholder.

**Fix:** add the version these docs describe: `cargo add laminapdf@0.4.0`. See [Install](../../pages/install.md#rust-laminapdf).

## Cargo finds no version that matches your requirement

**Cause:** Lamina PDF's releases before `0.2.0` were alphas, whose versions end in `-alpha` and a number, and Cargo treats them as pre-releases. A requirement such as `"0.1"` matches only full releases, so Cargo's error lists the alphas as "candidate versions found which didn't match".

**Fix:** require `0.2.0` or later, as `cargo add` writes it: `laminapdf = "0.4.0"`. Cargo then takes later releases in the same `0.x` line, which the [stability policy](../../pages/reference/stability.md) promises need no change to your code. Read the [changelog](../../pages/reference/changelog.md) before you move to a new `0.x` line.

## `Builder` or `page_text` is not found in `laminapdf`

**Cause:** the crate compiles only the parts of the API its features turn on, so a missing feature gives `unresolved import` or `no method named`. With `--no-default-features`, it only opens, reads, edits and saves documents.

**Fix:** turn on the feature that brings what you use: `build` for the builder without text, `extract` for text extraction, or `text` for fonts and text, which brings both. The default features include `text`. [Install](../../pages/install.md#rust-laminapdf) lists every feature.

## `laminapdf::explain` is not found

**Cause:** `explain` returns each code's page from these docs as text, which adds about 80 KB to your program. The function is behind the `explain` feature, which is off by default.

**Fix:** turn the feature on: `cargo add laminapdf@0.4.0 --features explain`. Or link to the page instead: every error's `docs_url()` gives its address.

## Cargo says the crate requires a newer rustc

**Cause:** your Rust is older than the oldest release Lamina PDF supports, two behind the newest stable one. The error reads "cannot be built because it requires rustc 1.97 or newer".

**Fix:** update Rust with `rustup update stable`, or pin the project to a newer toolchain in its `rust-toolchain.toml`. See the [stability policy](../../pages/reference/stability.md).

Next: [opening and saving files](../../pages/troubleshooting/files.md), for passwords, signed files and limits.
