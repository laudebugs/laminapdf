---
title: "Error and warning codes"
description: "Every error and warning code Lamina PDF raises, with a page for each that says when it happens and how to fix it."
version: "0.2.0"
url: "https://docs.laminapdf.com/reference/codes/"
---

# Error and warning codes

Every error and warning code Lamina PDF raises, with a page for each that says when it happens and how to fix it.

Every error and warning Lamina PDF raises has a code, such as `wrong-password`, and the code is the same in Rust, WebAssembly and `@laminapdf/sdk`. Match on its `code`, not its `message`: a message can change, but a code keeps its meaning in every version. [Errors and warnings](../../pages/concepts/errors-and-warnings.md) shows how to catch them in your code.

## React to a code

Your app catches an error, reads its code, and picks what to do next. This example opens a file. It asks for a password when the file needs one, asks again when the password is wrong, and tells the person when the file isn't a PDF:

**TypeScript**

`packages/sdk/examples/react-to-error-codes.ts`:

```ts
// Turn an error's code into what your app does next: ask for a password, ask again, or tell the
// person the file isn't a PDF. Any other code throws, so your app passes on the codes it doesn't
// handle.
import { Document, isLaminaPdfError } from "@laminapdf/sdk";

/** What the app does after trying to open a file. */
export type NextStep =
  | { readonly step: "show"; readonly pages: number; readonly warnings: readonly string[] }
  | { readonly step: "ask-for-password" }
  | { readonly step: "ask-again" }
  | { readonly step: "not-a-pdf" };

/** Opens `bytes`, with `password` if one was given, and says what to do next. Needs `init()`. */
export function openOrAsk(bytes: Uint8Array, password?: string): NextStep {
  try {
    using doc = Document.open(bytes, password === undefined ? {} : { password });
    // Warnings mean the file opened but not exactly as written: a repair, or a value dropped at a
    // limit. Their codes say which. `pageCount` comes first, since reading the pages can add some.
    return { step: "show", pages: doc.pageCount, warnings: doc.warnings.map((w) => w.code) };
  } catch (e) {
    if (isLaminaPdfError(e, "password-required")) return { step: "ask-for-password" };
    if (isLaminaPdfError(e, "wrong-password")) return { step: "ask-again" };
    if (isLaminaPdfError(e, "not-pdf")) return { step: "not-a-pdf" };
    throw e;
  }
}
```

**Rust**

`crates/laminapdf/examples/react_to_error_codes.rs`:

````rust
//! Turn an error's code into what your app does next: ask for a password, ask again, or tell the
//! person the file isn't a PDF. Any other code is returned as the error, so your app passes on
//! the codes it doesn't handle.
//!
//! ```sh
//! cargo run -p laminapdf --example react_to_error_codes -- in.pdf [password]
//! ```

use std::error::Error;

use laminapdf::{Document, ErrorCode, OpenOptions, Warning, WarningCode};

/// What the app does after trying to open a file.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum NextStep {
    Show { pages: u32, warnings: Vec<WarningCode> },
    AskForPassword,
    AskAgain,
    NotAPdf,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let input = args.next().ok_or("usage: react_to_error_codes in.pdf [password]")?;
    let password = args.next();
    let next = open_or_ask(std::fs::read(input)?, password.as_deref())?;
    println!("{next:?}");
    Ok(())
}

/// Opens `bytes`, with `password` if one was given, and says what to do next.
pub(crate) fn open_or_ask(bytes: Vec<u8>, password: Option<&str>) -> Result<NextStep, laminapdf::Error> {
    let options = match password {
        Some(password) => OpenOptions::new().with_password(password),
        None => OpenOptions::new(),
    };
    match Document::open_bytes(bytes, options) {
        Ok(doc) => {
            // Count the pages first: reading the page tree can add warnings of its own.
            let pages = doc.page_count()?;
            // Warnings mean the file opened but not exactly as written: a repair, or a value
            // dropped at a limit. Their codes say which.
            let warnings = doc
                .warnings()
                .next_batch(u32::MAX)
                .iter()
                .map(Warning::warning_code)
                .collect();
            Ok(NextStep::Show { pages, warnings })
        }
        Err(e) => match e.error_code() {
            ErrorCode::PasswordRequired => Ok(NextStep::AskForPassword),
            ErrorCode::WrongPassword => Ok(NextStep::AskAgain),
            ErrorCode::NotPdf => Ok(NextStep::NotAPdf),
            _ => Err(e),
        },
    }
}
````

Any other code goes back to the caller as the error. New codes can appear in any release, so pass on any code you don't handle; your app then acts only on the codes it knows.

## Every code

The tables group the codes by the part of Lamina PDF that raises them. Each row gives the code's `kind`, one of nine broad groups such as `input`, `limit` or `security`, which you can match on when you don't need the exact code. Then it says what happened and what to do. Each code links to its own page, which says when the code happens, why, how to fix it, and what its `data` holds.

See the [error and warning codes](../../errors/README.md).
