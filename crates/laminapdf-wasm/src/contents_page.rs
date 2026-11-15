//! `WasmDocument.createContentsPage` (E3l): a contents page made from the bookmarks, in the text
//! module, since it lays text out. Its fonts are bytes read field by field (as `merge` reads its
//! inputs), so they aren't copied through JSON; the rest is a record.

use laminapdf::{ContentsOptions, Font, PagePosition, Standard14};
use serde::Deserialize;
use tsify::Tsify;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

use crate::convert::{from_js_or_default, to_js};
use crate::document::WasmDocument;
use crate::dto::EditedJs;
use crate::error::{JsResult, invalid_argument, lamina_error};

/// Where a contents page goes.
#[derive(Debug, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum ContentsAtInit {
    /// Before the first page.
    Start,
    /// After the last page.
    End,
    /// Before the page with this ID.
    Before(String),
    /// After the page with this ID.
    After(String),
}

/// How `createContentsPage` makes the page; every field has a default.
#[derive(Debug, Default, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ContentsPageInit {
    /// Where it goes: before the first page when left out.
    #[tsify(optional)]
    pub at: Option<ContentsAtInit>,
    /// The levels of bookmarks it lists, 1 or 2 (the default).
    #[tsify(optional)]
    pub levels: Option<u8>,
    /// The heading at its top: "Contents" when left out.
    #[tsify(optional)]
    pub title: Option<String>,
    /// Whether a bookmark for it goes first in the outline (no by default).
    #[tsify(optional)]
    pub bookmark: Option<bool>,
    /// The entries' size in points (11 by default); the heading is half again as large.
    #[tsify(optional)]
    pub size: Option<f64>,
}

#[wasm_bindgen]
impl WasmDocument {
    /// Inserts a contents page listing the bookmarks that go to a page, each entry with the label
    /// its page shows and a link there, in one transaction (one undo step). `fonts` are the
    /// page's font then its fallbacks, each `{ bytes, index? }` or `{ standard }`; none means
    /// Helvetica, which a PDF/A or PDF/UA-1 file refuses.
    ///
    /// # Errors
    /// `no-bookmarks`; `font-not-embedded`, `missing-glyph`, `invalid-font`; `unknown-page`;
    /// `invalid-argument`, `unknown-option`; as [`WasmDocument::apply`].
    #[wasm_bindgen(js_name = createContentsPage, unchecked_return_type = "EditedJs")]
    pub fn create_contents_page(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "ContentsPageInit | undefined")] options: &JsValue,
        #[wasm_bindgen(
            unchecked_param_type = "({ bytes: Uint8Array; index?: number } | { standard: string })[] | undefined"
        )]
        fonts: &JsValue,
    ) -> JsResult<JsValue> {
        let init: ContentsPageInit = from_js_or_default(options, "options")?;
        let mut options = ContentsOptions::new();
        if let Some(at) = init.at {
            options = options.with_at(position(at)?);
        }
        if let Some(levels) = init.levels {
            options = options.with_levels(levels);
        }
        if let Some(title) = init.title {
            options = options.with_title(title);
        }
        if let Some(bookmark) = init.bookmark {
            options = options.with_bookmark(bookmark);
        }
        if let Some(size) = init.size {
            options = options.with_size(size);
        }
        for (n, font) in fonts_of(fonts)?.into_iter().enumerate() {
            options = if n == 0 {
                options.with_font(font)
            } else {
                options.with_fallback(font)
            };
        }
        let edited = self
            .shared()
            .borrow_mut()
            .create_contents_page(&options)
            .map_err(|e| lamina_error(&e))?;
        Ok(to_js(&EditedJs::from(edited)))
    }
}

/// `at` as the façade's position.
fn position(at: ContentsAtInit) -> JsResult<PagePosition> {
    let page = |id: &str| {
        id.parse()
            .map_err(|_| invalid_argument("options", format!("not a page ID: {id}"), Vec::new()))
    };
    Ok(match at {
        ContentsAtInit::Start => PagePosition::Start,
        ContentsAtInit::End => PagePosition::End,
        ContentsAtInit::Before(id) => PagePosition::Before(page(&id)?),
        ContentsAtInit::After(id) => PagePosition::After(page(&id)?),
    })
}

/// The fonts, read field by field.
fn fonts_of(fonts: &JsValue) -> JsResult<Vec<Font>> {
    if fonts.is_undefined() || fonts.is_null() {
        return Ok(Vec::new());
    }
    let wrong = |what: &str| invalid_argument("fonts", what.to_owned(), Vec::new());
    let list = fonts
        .dyn_ref::<js_sys::Array>()
        .ok_or_else(|| wrong("not a list of { bytes, index? } or { standard }"))?;
    list.iter()
        .map(|item| {
            let field = |name: &str| {
                js_sys::Reflect::get(&item, &JsValue::from_str(name))
                    .ok()
                    .filter(|v| !v.is_undefined())
            };
            if let Some(name) = field("standard") {
                let name = name
                    .as_string()
                    .ok_or_else(|| wrong("a font's standard isn't a string"))?;
                let font = Standard14::from_base_font(&name).ok_or_else(|| {
                    let known: Vec<String> = Standard14::ALL.iter().map(|f| f.base_font().to_owned()).collect();
                    invalid_argument(
                        "fonts",
                        "not a standard 14 font name".to_owned(),
                        laminapdf::did_you_mean(&name, &known),
                    )
                })?;
                return Ok(Font::standard(font));
            }
            let bytes = field("bytes")
                .and_then(|b| b.dyn_into::<js_sys::Uint8Array>().ok())
                .ok_or_else(|| wrong("a font's bytes aren't a Uint8Array"))?
                .to_vec();
            let font = match field("index") {
                None => Font::from_bytes(bytes),
                Some(index) => {
                    let index = index
                        .as_f64()
                        .filter(|i| i.fract() == 0.0 && (0.0..=f64::from(u32::MAX)).contains(i))
                        .ok_or_else(|| wrong("a font's index isn't a whole number"))?;
                    #[expect(
                        clippy::cast_possible_truncation,
                        clippy::cast_sign_loss,
                        reason = "a whole number in u32's range, checked above"
                    )]
                    Font::from_collection(bytes, index as u32)
                }
            };
            font.map_err(|e| lamina_error(&e.into()))
        })
        .collect()
}
