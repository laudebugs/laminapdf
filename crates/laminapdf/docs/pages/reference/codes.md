---
title: "Error and warning codes"
description: "Every code Lamina PDF raises, generated from fixtures/codes.json when the site is built."
version: "0.2.0-alpha.5"
url: "https://docs.laminapdf.com/reference/codes/"
---

# Error and warning codes

Every code Lamina PDF raises, generated from fixtures/codes.json when the site is built.

Every error and warning Lamina PDF raises, in Rust, in WebAssembly and in `@laminapdf/sdk`, has one of these codes. The tables below are read from `fixtures/codes.json` when this site is built: the same file `cargo xtask gen codes` generates the Rust enums and the TypeScript unions from. Each code links to its page on this site, which explains when it happens, why, how to fix it and what its `data` holds. See [Errors and warnings](../../pages/concepts/errors-and-warnings.md) for how to handle them.

See the [error and warning codes](../../errors/README.md).
