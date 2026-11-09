//! Moving through a document across the boundary: `WasmDocument.outline()` and
//! `namedDestination(name)`, each target a page index and a view in page space (the extract
//! and text modules, so the base module stays in its size budget).

use laminapdf::{OutlineNode, Target, TargetView};
use serde::Serialize;
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use crate::convert::to_js;
use crate::document::WasmDocument;
use crate::error::{JsResult, lamina_error};

/// One outline item.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OutlineNodeJs {
    /// Its title (document text).
    pub title: String,
    /// Whether it opens with its children shown.
    pub open: bool,
    /// Where it goes, when its destination names a page of this document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetJs>,
    /// Its children, in order.
    pub children: Vec<OutlineNodeJs>,
}

/// A page and how to show it.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TargetJs {
    /// The page's 0-based index.
    pub page: u32,
    /// How to show it, in page space.
    pub view: TargetViewJs,
}

/// How a target shows its page, in page space; an unset coordinate is left out.
#[derive(Debug, Serialize, Tsify)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(crate) enum TargetViewJs {
    /// The point (`x`, `y`) at the viewport's top-left, at `zoom` when given.
    At {
        /// Left edge.
        #[serde(skip_serializing_if = "Option::is_none")]
        x: Option<f64>,
        /// Top edge.
        #[serde(skip_serializing_if = "Option::is_none")]
        y: Option<f64>,
        /// Zoom factor (1 is 100 %).
        #[serde(skip_serializing_if = "Option::is_none")]
        zoom: Option<f64>,
    },
    /// The whole page.
    Fit,
    /// The page's width, with `y` at the top.
    FitWidth {
        /// Top edge.
        #[serde(skip_serializing_if = "Option::is_none")]
        y: Option<f64>,
    },
    /// The page's height, with `x` at the left.
    FitHeight {
        /// Left edge.
        #[serde(skip_serializing_if = "Option::is_none")]
        x: Option<f64>,
    },
    /// The rectangle.
    FitRect {
        /// Left.
        x0: f64,
        /// Top.
        y0: f64,
        /// Right.
        x1: f64,
        /// Bottom.
        y1: f64,
    },
}

impl From<&OutlineNode> for OutlineNodeJs {
    fn from(node: &OutlineNode) -> Self {
        Self {
            title: node.title.clone(),
            open: node.open,
            target: node.target.map(TargetJs::from),
            children: node.children.iter().map(Self::from).collect(),
        }
    }
}

impl From<Target> for TargetJs {
    fn from(target: Target) -> Self {
        let view = match target.view {
            TargetView::At { x, y, zoom } => TargetViewJs::At { x, y, zoom },
            TargetView::Fit => TargetViewJs::Fit,
            TargetView::FitWidth { y } => TargetViewJs::FitWidth { y },
            TargetView::FitHeight { x } => TargetViewJs::FitHeight { x },
            TargetView::FitRect { x0, y0, x1, y1 } => TargetViewJs::FitRect { x0, y0, x1, y1 },
            // A view a later façade adds: the page's top.
            _ => TargetViewJs::At {
                x: None,
                y: None,
                zoom: None,
            },
        };
        Self {
            page: target.page,
            view,
        }
    }
}

#[wasm_bindgen]
impl WasmDocument {
    /// The outline's top-level items, each with its children; empty without an outline (the
    /// extract and text modules).
    ///
    /// # Errors
    /// `data-pending` and the reading errors.
    #[wasm_bindgen(unchecked_return_type = "OutlineNodeJs[]")]
    pub fn outline(&self) -> JsResult<JsValue> {
        let outline = self.shared().borrow().outline().map_err(|e| lamina_error(&e))?;
        Ok(to_js(&outline.iter().map(OutlineNodeJs::from).collect::<Vec<_>>()))
    }

    /// The named destination `name`, or `undefined` when the document has none by that name or
    /// it names no page here (the extract and text modules).
    ///
    /// # Errors
    /// `data-pending` and the reading errors.
    #[wasm_bindgen(js_name = namedDestination, unchecked_return_type = "TargetJs | undefined")]
    pub fn named_destination(&self, name: &str) -> JsResult<JsValue> {
        let target = self
            .shared()
            .borrow()
            .named_destination(name)
            .map_err(|e| lamina_error(&e))?;
        match target {
            Some(target) => Ok(to_js(&TargetJs::from(target))),
            None => Ok(JsValue::UNDEFINED),
        }
    }
}
