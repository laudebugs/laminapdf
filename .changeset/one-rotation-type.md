---
"@laminapdf/render": minor
---

Editing a page whose `/Rotate` is stored as an integral real, such as `90.0`, now sees the rotation the page displays: before, the `RotatePage` op read it as 0. The Rust crates share one rotation type, `Rotation`, which `laminapdf-syntax` defines and the model, the façade and `laminapdf-render` re-export. `laminapdf-render`'s `ViewRotation` is gone; `RenderRequest::with_rotation` takes a `Rotation`, and `RenderPage::rotate` is a `Rotation` instead of degrees. `laminapdf_cos::Page` no longer has `media_box`, `crop_box` and `rotate`: read them from `laminapdf-model`'s page view.
