---
title: "open-action-not-destination"
version: "0.4.0"
url: "https://docs.laminapdf.com/reference/codes/open-action-not-destination/"
---

# open-action-not-destination

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

An edit to the initial view tried to set or remove the page the document opens at, but the document's open action isn't a page to open at. It runs something besides opening a page: a script, a named action, a link to another file, a page change followed by further actions, or an action whose type can't be read. Lamina PDF never replaces or removes such an action as a side effect, so it refused the whole edit and changed nothing.

## Why

A PDF's catalog holds one open action. It's either a destination, meaning a page and a zoom, or an action that a reader runs when the file opens. Setting the opening page writes a destination over that action, and removing it would delete the action. Both would lose what the file's author put there without saying so.

## How to fix

Leave the opening page out of the edit: in Rust, don't call `with_open` on `SetInitialView`; in TypeScript, leave `open` out of `setInitialView`. The page mode, page layout and window preferences still change, and the open action stays as it is. `initial_view()` (Rust) and `initialView()` (TypeScript) report the open action's type, so a dialog can say what the file runs and turn the page and zoom controls off.

## Example

```rust
use laminapdf::{OpenAt, OpenZoom, Op, SetInitialView};

// The file opens with JavaScript, so this edit is refused:
let op = SetInitialView::default().with_open(Some(OpenAt::new(page, OpenZoom::FitPage)));
doc.apply(vec![Op::from(op)])?; // open-action-not-destination
```

## Data fields

None.
