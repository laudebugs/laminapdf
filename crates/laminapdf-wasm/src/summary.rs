//! `WasmDocument.commentSummary` (text module): a new PDF/UA-1 document listing the comments,
//! each page drawn beside its comments or the comments alone as a list. The options are a
//! record; the fonts are read field by field, as `layOutContentsPage` reads them.

use laminapdf::{PageSize, Pt, SummaryLayout, SummaryOptions, SummaryPaper};
use serde::Deserialize;
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use crate::contents_page::fonts_of;
use crate::convert::from_js_or_default;
use crate::document::WasmDocument;
use crate::error::{JsResult, invalid_argument, lamina_error};

/// How a comment summary sets out the comments.
#[derive(Debug, Clone, Copy, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) enum SummaryLayoutInit {
    /// Each page drawn small beside its comments.
    BesidePage,
    /// The comments alone, under each page's heading.
    List,
}

/// The paper: `"document"` (the first page's size) or `[width, height]` in points.
#[derive(Debug, Deserialize, Tsify)]
#[serde(untagged)]
pub(crate) enum SummaryPaperInit {
    /// `"document"`.
    Named(String),
    /// A width and height in points.
    Size([f64; 2]),
}

/// What `commentSummary` shows; every field has a default.
#[derive(Debug, Default, Deserialize, Tsify)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CommentSummaryInit {
    /// `besidePage` (the default) or `list`.
    #[tsify(optional)]
    pub layout: Option<SummaryLayoutInit>,
    /// The IDs of the comments to list; every comment when left out.
    #[tsify(optional)]
    pub comments: Option<Vec<String>>,
    /// Whether replies are listed (yes by default).
    #[tsify(optional)]
    pub replies: Option<bool>,
    /// Whether each comment's status is listed (yes by default).
    #[tsify(optional)]
    pub status: Option<bool>,
    /// Whether pages without comments are listed (no by default).
    #[tsify(optional)]
    pub empty_pages: Option<bool>,
    /// The paper (`"document"` by default).
    #[tsify(optional)]
    pub paper: Option<SummaryPaperInit>,
    /// The summary's title.
    #[tsify(optional)]
    pub title: Option<String>,
    /// The language most comments are in (`en` by default); the summary's own words are English.
    #[tsify(optional)]
    pub language: Option<String>,
}

#[wasm_bindgen]
impl WasmDocument {
    /// A new document listing the comments as `options` asks, PDF/UA-1, set in `fonts`: its
    /// font, then its fallbacks, each `{ bytes, index? }`. This document doesn't change.
    ///
    /// # Errors
    /// `invalid-argument` for no font or options of the wrong shape; `unknown-option`;
    /// `font-not-embedded` for a standard 14 font; `missing-glyph`; reading errors.
    #[wasm_bindgen(js_name = commentSummary)]
    pub fn comment_summary(
        &self,
        #[wasm_bindgen(unchecked_param_type = "CommentSummaryInit | undefined")] options: &JsValue,
        #[wasm_bindgen(unchecked_param_type = "({ bytes: Uint8Array; index?: number } | { standard: string })[]")]
        fonts: &JsValue,
    ) -> JsResult<WasmDocument> {
        let init: CommentSummaryInit = from_js_or_default(options, "options")?;
        let mut fonts = fonts_of(fonts)?.into_iter();
        let font = fonts
            .next()
            .ok_or_else(|| invalid_argument("fonts", "a summary needs a font".to_owned(), Vec::new()))?;
        let mut options = fonts.fold(SummaryOptions::new(font), SummaryOptions::with_fallback);
        if let Some(layout) = init.layout {
            options = options.with_layout(match layout {
                SummaryLayoutInit::BesidePage => SummaryLayout::BesidePage,
                SummaryLayoutInit::List => SummaryLayout::List,
            });
        }
        if let Some(ids) = init.comments {
            let ids = ids
                .iter()
                .map(|id| {
                    id.parse()
                        .map_err(|_| invalid_argument("options", format!("not an annotation ID: {id}"), Vec::new()))
                })
                .collect::<JsResult<Vec<_>>>()?;
            options = options.with_comments(ids);
        }
        if let Some(replies) = init.replies {
            options = options.with_replies(replies);
        }
        if let Some(status) = init.status {
            options = options.with_status(status);
        }
        if let Some(empty) = init.empty_pages {
            options = options.with_empty_pages(empty);
        }
        match init.paper {
            None => {}
            Some(SummaryPaperInit::Named(name)) if name == "document" => {}
            Some(SummaryPaperInit::Named(name)) => {
                return Err(invalid_argument(
                    "options",
                    format!("paper is \"document\" or [width, height], not {name:?}"),
                    Vec::new(),
                ));
            }
            Some(SummaryPaperInit::Size([width, height])) => {
                options = options.with_paper(SummaryPaper::Size(PageSize::new(Pt::new(width), Pt::new(height))));
            }
        }
        if let Some(title) = init.title {
            options = options.with_title(title);
        }
        if let Some(language) = init.language {
            options = options.with_language(language);
        }
        let summary = self
            .shared()
            .borrow()
            .comment_summary(&options)
            .map_err(|e| lamina_error(&e))?;
        Ok(WasmDocument::new(summary))
    }
}
