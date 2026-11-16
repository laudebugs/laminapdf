//! A contents page made from the bookmarks, in three calls, so a document open in the base
//! or extract module can have one: `WasmDocument.contentsListing` (every module) reads what it
//! lists, `layOutContentsPage` (text module, which has the fonts) lays it out as a document of its
//! own, and `WasmDocument.insertContentsPage` (every module) inserts it as one undo step. Fonts
//! and the page's bytes are read field by field (as `merge` reads its inputs), so they aren't
//! copied through JSON; the rest are records.

use laminapdf::{ContentsOptions, ContentsPage, PagePosition, PlacedEntry, RectPt};
#[cfg(feature = "text")]
use laminapdf::{Font, Standard14};
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

#[wasm_bindgen(typescript_custom_section)]
const CONTENTS_TS: &str = r#"
/**
 * A contents page laid out by `layOutContentsPage`: its document, each entry's box as six numbers
 * (entry, page, x, y, width, height in page space), and the listing it was laid out from.
 */
export interface ContentsPageJs {
  bytes: Uint8Array;
  boxes: Float64Array;
  listing: string;
}
"#;

#[wasm_bindgen]
impl WasmDocument {
    /// What a contents page made with `options` lists (every module), as JSON: the bookmarks
    /// that go to a page, under `entries`, each with its `level`, `title` and the `labels` its
    /// page will show when the contents page takes one page, two, and so on up to ten. Lay it out with
    /// `layOutContentsPage` (text module), then insert it with `insertContentsPage`.
    ///
    /// # Errors
    /// `no-bookmarks`, `unknown-page`; `invalid-argument`, `unknown-option`; reading errors.
    #[wasm_bindgen(js_name = contentsListing)]
    pub fn contents_listing(
        &self,
        #[wasm_bindgen(unchecked_param_type = "ContentsPageInit | undefined")] options: &JsValue,
    ) -> JsResult<String> {
        let options = options_of(options)?;
        let listing = self
            .shared()
            .borrow()
            .contents_listing(&options)
            .map_err(|e| lamina_error(&e))?;
        Ok(listing.to_json())
    }

    /// Inserts `page`, laid out from the listing `options` gives, with a link over each entry
    /// and, with `options.bookmark`, a bookmark for it, as one transaction (one undo step).
    ///
    /// # Errors
    /// `document-changed` when the document no longer gives the listing `page` was laid out
    /// from; `invalid-argument`; as [`WasmDocument::apply`].
    #[wasm_bindgen(js_name = insertContentsPage, unchecked_return_type = "EditedJs")]
    pub fn insert_contents_page(
        &mut self,
        #[wasm_bindgen(unchecked_param_type = "ContentsPageInit | undefined")] options: &JsValue,
        #[wasm_bindgen(unchecked_param_type = "ContentsPageJs")] page: &JsValue,
    ) -> JsResult<JsValue> {
        let options = options_of(options)?;
        let page = page_of(page)?;
        let edited = self
            .shared()
            .borrow_mut()
            .insert_contents_page(&options, page)
            .map_err(|e| lamina_error(&e))?;
        Ok(to_js(&EditedJs::from(edited)))
    }
}

/// The contents page of `listing` (from `contentsListing`) laid out as a document of its own
/// (text module). `fonts` are its font then its fallbacks, each `{ bytes, index? }` or
/// `{ standard }`; none means Helvetica, which a PDF/A or PDF/UA-1 file refuses.
///
/// # Errors
/// `font-not-embedded`, `missing-glyph`, `invalid-font`; `invalid-argument`.
#[cfg(feature = "text")]
#[wasm_bindgen(js_name = layOutContentsPage, unchecked_return_type = "ContentsPageJs")]
pub fn lay_out_contents_page(
    listing: &str,
    #[wasm_bindgen(
        unchecked_param_type = "({ bytes: Uint8Array; index?: number } | { standard: string })[] | undefined"
    )]
    fonts: &JsValue,
) -> JsResult<JsValue> {
    let listing: laminapdf::ContentsListing = serde_json::from_str(listing).map_err(|e| {
        invalid_argument(
            "listing",
            format!("not a listing from contentsListing: {e}"),
            Vec::new(),
        )
    })?;
    let mut set = laminapdf::ContentsFonts::new();
    for (n, font) in fonts_of(fonts)?.into_iter().enumerate() {
        set = if n == 0 {
            set.with_font(font)
        } else {
            set.with_fallback(font)
        };
    }
    let page = listing.lay_out(&set).map_err(|e| lamina_error(&e))?;
    #[expect(clippy::cast_precision_loss, reason = "entry and page indices are small")]
    let boxes: Vec<f64> = page
        .entries
        .iter()
        .flat_map(|p| {
            let r = p.rect;
            [
                p.entry as f64,
                p.page as f64,
                r.x0.get(),
                r.y0.get(),
                r.width().get(),
                r.height().get(),
            ]
        })
        .collect();
    let out = js_sys::Object::new();
    let fields: [(&str, JsValue); 3] = [
        ("bytes", js_sys::Uint8Array::from(page.bytes.as_slice()).into()),
        ("boxes", js_sys::Float64Array::from(boxes.as_slice()).into()),
        ("listing", JsValue::from_str(&page.listing)),
    ];
    for (name, value) in fields {
        js_sys::Reflect::set(&out, &JsValue::from_str(name), &value)
            .map_err(|_| invalid_argument("page", "can't build the result".to_owned(), Vec::new()))?;
    }
    Ok(out.into())
}

/// `options` as the façade's.
fn options_of(options: &JsValue) -> JsResult<ContentsOptions> {
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
    Ok(options)
}

/// A laid-out page from JavaScript, read field by field.
fn page_of(page: &JsValue) -> JsResult<ContentsPage> {
    let wrong = |what: &str| invalid_argument("page", what.to_owned(), Vec::new());
    let field = |name: &str| js_sys::Reflect::get(page, &JsValue::from_str(name)).ok();
    let bytes = field("bytes")
        .and_then(|b| b.dyn_into::<js_sys::Uint8Array>().ok())
        .ok_or_else(|| wrong("its bytes aren't a Uint8Array"))?
        .to_vec();
    let boxes = field("boxes")
        .and_then(|b| b.dyn_into::<js_sys::Float64Array>().ok())
        .ok_or_else(|| wrong("its boxes aren't a Float64Array"))?
        .to_vec();
    let listing = field("listing")
        .and_then(|l| l.as_string())
        .ok_or_else(|| wrong("its listing isn't a string"))?;
    let index = |v: f64| {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a whole number in usize's range, checked first"
        )]
        (v.fract() == 0.0 && v >= 0.0 && v <= f64::from(u32::MAX))
            .then_some(v as usize)
            .ok_or_else(|| wrong("an entry or page index in its boxes isn't a whole number"))
    };
    let mut entries = Vec::new();
    for b in boxes.chunks(6) {
        let [entry, page, x, y, w, h] =
            <[f64; 6]>::try_from(b).map_err(|_| wrong("its boxes aren't six numbers each"))?;
        entries.push(PlacedEntry::new(
            index(entry)?,
            index(page)?,
            RectPt::from_points(x, y, w, h),
        ));
    }
    Ok(ContentsPage::new(bytes, entries, listing))
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
#[cfg(feature = "text")]
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
