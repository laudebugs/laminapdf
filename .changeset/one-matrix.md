---
"@laminapdf/sdk": minor
---

Breaking (Rust, while alpha): `Matrix`, `FillRule` and `Dash` are now defined once, in `laminapdf-model`. `laminapdf`, `laminapdf-build` and `laminapdf-content` re-export them under their old paths. Their API changes as follows:

- `Matrix::translate` takes two `f64`s instead of two `Pt`s.
- `Dash` lengths are `f64`s in the drawing's own units, so the builder takes points: `Dash::new(vec![18.0, 6.0], 0.0)`.
- The content reader's `Matrix::new` takes an array, `multiply` is `then` (the receiver applies first, with `after` for the other order), and `transform_point` is `apply`.
- `PageFrame::to_user_space_matrix` and `TilePlan::page_to_tile` return a `Matrix` instead of a `[f64; 6]`. Read its fields, or call `to_array()` for the six numbers. `laminapdf-render` re-exports `Matrix`.
- `Dash` is `#[non_exhaustive]`: build one with `Dash::new`. The content reader's `Matrix::from_object` is removed.
- The content reader's current colour, `laminapdf_content::Color`, is now `CurrentColor`, so `Color` means only the builder's colour.

The JavaScript API and every output are unchanged.
