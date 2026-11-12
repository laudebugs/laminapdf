---
"@laminapdf/render": minor
---

The Rust crate `laminapdf-content` can now add content to a page without changing what the page already draws. `PageContentWriter` wraps a page's content in `q … Q`, adds streams before or after it, and registers Form XObjects, graphics states and fonts under names the page doesn't use yet. A page that shares its resources or content streams with other pages keeps them unchanged.
