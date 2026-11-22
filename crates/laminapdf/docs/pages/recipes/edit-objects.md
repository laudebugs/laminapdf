---
title: "Edit what a page draws"
description: "List the shapes, text and images a PDF page already draws, then move, resize, delete and reorder them in one edit that rewrites only the objects it names."
version: "0.7.0"
url: "https://docs.laminapdf.com/recipes/edit-objects/"
---

# Edit what a page draws

List the shapes, text and images a PDF page already draws, then move, resize, delete and reorder them in one edit that rewrites only the objects it names.

You'll change four things a page already draws: shrink a rectangle, delete a circle, move an image across the page and send a curve under everything else. Use it to tidy a generated page, move a logo, or remove a stray mark before you send a file on.

First, the program lists the page's objects in painting order, each with its box and, when it can't be moved, the reason. Each edit then names its object as the list gave it, and the four edits apply together:

**TypeScript**

`packages/sdk/examples/edit-objects.ts`:

```ts
// Move, resize, delete and reorder what a page already draws.
//
// Lists the first page's objects, then changes four of them in one edit: shrinks the first
// shape about its centre, deletes the second, moves the last image across the page and sends
// the third shape, a line across the page, under everything else. Each edit names its object as
// the list does, and only the objects it names are written again: the rest of the page keeps its
// bytes. An object that would draw differently in its new place in the stacking order (another
// clip or transparency there) is refused with `content-not-editable`.
import { Document } from "@laminapdf/sdk";

/** The document with four objects of its first page changed. Needs `init({ extract: true })`. */
export function editObjects(bytes: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const objects = doc.content.objects(0);
  for (const object of objects) {
    console.log(object.kind, object.bounds, object.readOnly ? `(read-only: ${object.readOnly})` : "");
  }
  const [square, second, line] = objects.filter((o) => o.kind === "path");
  const photo = objects.filter((o) => o.kind === "image").at(-1);
  if (!square?.bounds || !second || !line || !photo) {
    throw new Error("the page needs three shapes and an image");
  }
  // Page space: points from the top-left, y down. Half size, about the square's centre.
  const [x0, y0, x1, y1] = square.bounds;
  const [cx, cy] = [(x0 + x1) / 2, (y0 + y1) / 2];
  doc.content.edit(0, [
    { transform: { object: square.object, matrix: [0.5, 0, 0, 0.5, cx - cx * 0.5, cy - cy * 0.5] } },
    { delete: { object: second.object } },
    { transform: { object: photo.object, matrix: [1, 0, 0, 1, 300, 0] } },
    { reorder: { object: line.object, to: "back" } },
  ]);
  return doc.save();
}
```

**Rust**

`crates/laminapdf/examples/edit_objects.rs`:

````rust
//! Move, resize, delete and reorder what a page already draws.
//!
//! Lists the first page's objects, then changes four of them in one edit: shrinks the first
//! shape about its centre, deletes the second, moves the last image across the page and sends
//! the third shape, a line across the page, under everything else. Each edit names its object
//! as the list does, and only the objects it names are written again: the rest of the page
//! keeps its bytes. An object that would draw differently in its new place in the stacking order
//! (another clip or transparency there) is refused with `content-not-editable`.
//!
//! ```sh
//! cargo run -p laminapdf --example edit_objects -- in.pdf [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    ContentEdit, ContentObjectKind, Document, EditContent, Matrix, OpenOptions, PageObject, SaveOptions, StackPosition,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let input = PathBuf::from(args.next().ok_or("usage: edit_objects in.pdf [out-dir]")?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let edited = edit(std::fs::read(input)?)?;
    std::fs::write(out.join("edit_objects.pdf"), edited)?;
    Ok(())
}

/// The document with four objects of its first page changed.
pub(crate) fn edit(bytes: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let objects = doc.content_objects(0)?;
    list(&objects);
    let page = doc.page_info(0)?.id;
    doc.apply(vec![EditContent::new(page, edits(&objects)?).into()])?;
    Ok(doc.save(&SaveOptions::new())?.bytes)
}

/// Prints each object: what it draws, its box and, when it can't be moved, why.
fn list(objects: &[PageObject]) {
    for object in objects {
        let read_only = match object.read_only {
            Some(reason) => format!(" (read-only: {})", reason.as_str()),
            None => String::new(),
        };
        println!("{:?} {:?}{read_only}", object.kind, object.bounds);
    }
}

/// The four edits, each naming its object as the list gave it.
fn edits(objects: &[PageObject]) -> Result<Vec<ContentEdit>, Box<dyn Error>> {
    let shapes: Vec<&PageObject> = objects.iter().filter(|o| o.kind == ContentObjectKind::Path).collect();
    let photo = objects
        .iter()
        .rev()
        .find(|o| o.kind == ContentObjectKind::Image)
        .ok_or("no image")?;
    let [square, second, line, ..] = shapes.as_slice() else {
        return Err("the page needs three shapes".into());
    };
    Ok(vec![
        ContentEdit::Transform {
            object: square.object,
            matrix: half_size(square)?,
        },
        ContentEdit::Delete { object: second.object },
        ContentEdit::Transform {
            object: photo.object,
            matrix: Matrix::translate(300.0, 0.0),
        },
        ContentEdit::Reorder {
            object: line.object,
            to: StackPosition::Back,
        },
    ])
}

/// Half size, about the object's centre. Page space: points from the top-left, y down.
fn half_size(object: &PageObject) -> Result<Matrix, Box<dyn Error>> {
    let bounds = object.bounds.ok_or("the object draws nothing")?;
    let (cx, cy) = (
        f64::midpoint(bounds.x0.get(), bounds.x1.get()),
        f64::midpoint(bounds.y0.get(), bounds.y1.get()),
    );
    Ok(Matrix::translate(-cx, -cy)
        .then(&Matrix::scale(0.5, 0.5))
        .then(&Matrix::translate(cx, cy)))
}
````

**Result:** [edit_objects.pdf](https://docs.laminapdf.com/examples/pdf/edit_objects.pdf)

In the result, the red rectangle is half its size about the same centre, the blue circle is gone and the gradient image sits 300 points to the right. The dashed curve now paints first, under everything else; nothing overlaps it, so it looks the same.

## How it works

- `content.objects(index)` (`content_objects` in Rust) lists what the page draws: paths, text, images, forms and shadings. Boxes are in page space, points from the top-left of the page as shown, with y down. `content.hitTest(index, [x, y])` gives the objects at a point, topmost first, and a rectangle `[x0, y0, x1, y1]` gives every object it meets.
- An edit names an object by its place in the list and a hash of its bytes. So once an edit has changed an object, a name taken before that edit fails with `content-object-stale`, and an edit never lands on the wrong object.
- A `transform` takes a matrix in page space: `[1, 0, 0, 1, 300, 0]` moves an object 300 points right, and `[0.5, 0, 0, 0.5, e, f]` halves it.
- Only the objects you name are written again. The rest of the page keeps its bytes, and when another page draws the same content, that page stays as it was.
- A resource nothing names any more, such as a deleted image's stored picture, leaves the page's resources.
- **Undo:** one `undo()` reverts the whole edit.

## What to change

- **Replace an image:** add the new JPEG or PNG with `addBlob(bytes)` (`add_blob`), then edit `{ replaceImage: { object, blob } }`. The new picture fills the same box.
- **Crop an image:** `{ crop: { object, rect } }` clips an image, a form or a shading to a rectangle in page space.
- **Stacking order:** `to` takes `"front"`, `"back"`, `{ before: object }` or `{ after: object }`. A move that would put the object under another clip or other transparency, where it would draw differently, is refused with `content-not-editable`, and nothing changes.
- **Clipping objects:** an object that clips what follows it can't be moved or deleted; its `readOnly` says `clipping-path`.
- **Private content:** a deleted object stays in the file's earlier revisions until a full save. The edit's `stillInEarlierRevisions` says so; save with `mode: "fullRewrite"` to drop it.

Next: [crop pages](../../pages/recipes/crop-pages.md), to hide part of every page instead.
