//! Moving through a document across the boundary: `WasmDocument.outline()`,
//! `namedDestination(name)`, `namedDestinations()` and `pageLinks(index)`, each target a page
//! index and a view in page space, and `destinationAt(page, view)`, a page-space view as the
//! destination the outline ops take (the extract and text modules, so the base module stays in
//! its size budget).

use laminapdf::{LinkTo, NamedDestination, OutlineNode, PageLink, PageTarget, PageTargetView};
use serde::{Deserialize, Serialize};
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use crate::convert::{from_js, to_js};
use crate::document::WasmDocument;
use crate::error::{JsResult, lamina_error};

/// One outline item.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OutlineNodeJs {
    /// Its ID (`o12.0`), which the outline ops take.
    pub id: String,
    /// Its title (document text).
    pub title: String,
    /// Whether it opens with its children shown.
    pub open: bool,
    /// Where it goes, when its destination names a page of this document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetJs>,
    /// The named destination it goes to, when it names one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_name: Option<String>,
    /// The web address it opens, when it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    /// Its action's type when that is neither `GoTo` nor `URI` (`JavaScript`, `Launch`, …).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other_action: Option<String>,
    /// Its colour as `"#rrggbb"`, when it isn't the default black.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Shown in bold.
    pub bold: bool,
    /// Shown in italic.
    pub italic: bool,
    /// Its children, in order.
    pub children: Vec<OutlineNodeJs>,
}

/// One named destination and where it goes.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NamedDestinationJs {
    /// The name.
    pub name: String,
    /// Where it goes, when it names a page of this document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<TargetJs>,
}

impl From<&NamedDestination> for NamedDestinationJs {
    fn from(d: &NamedDestination) -> Self {
        Self {
            name: d.name.clone(),
            target: d.target.map(TargetJs::from),
        }
    }
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Tsify)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
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
            id: node.id.to_string(),
            title: node.title.clone(),
            open: node.open,
            target: node.target.map(TargetJs::from),
            destination_name: node.destination_name.clone(),
            uri: node.uri.clone(),
            other_action: node.other_action.clone(),
            color: node.color.map(|c| c.to_string()),
            bold: node.bold,
            italic: node.italic,
            children: node.children.iter().map(Self::from).collect(),
        }
    }
}

impl From<TargetViewJs> for PageTargetView {
    fn from(view: TargetViewJs) -> Self {
        match view {
            TargetViewJs::At { x, y, zoom } => Self::At { x, y, zoom },
            TargetViewJs::Fit => Self::Fit,
            TargetViewJs::FitWidth { y } => Self::FitWidth { y },
            TargetViewJs::FitHeight { x } => Self::FitHeight { x },
            TargetViewJs::FitRect { x0, y0, x1, y1 } => Self::FitRect { x0, y0, x1, y1 },
        }
    }
}

impl From<PageTarget> for TargetJs {
    fn from(target: PageTarget) -> Self {
        let view = match target.view {
            PageTargetView::At { x, y, zoom } => TargetViewJs::At { x, y, zoom },
            PageTargetView::Fit => TargetViewJs::Fit,
            PageTargetView::FitWidth { y } => TargetViewJs::FitWidth { y },
            PageTargetView::FitHeight { x } => TargetViewJs::FitHeight { x },
            PageTargetView::FitRect { x0, y0, x1, y1 } => TargetViewJs::FitRect { x0, y0, x1, y1 },
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

/// One link on a page, in page space. Its TypeScript type is written below: tsify gives a
/// flattened enum as `interface … extends` a union, which TypeScript refuses.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PageLinkJs {
    /// Its rectangle: `[x0, y0, x1, y1]`, top-left and bottom-right.
    pub rect: [f64; 4],
    /// The areas that follow it, eight numbers per area (four corners, `x` then `y`); left out
    /// when the rectangle is the area.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub quads: Vec<f64>,
    /// Its description (`/Contents`, document text).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contents: Option<String>,
    /// What following it does.
    #[serde(flatten)]
    pub to: LinkToJs,
}

/// What following a link does, by `kind`.
#[derive(Debug, Serialize, Tsify)]
#[serde(tag = "kind", rename_all = "kebab-case", rename_all_fields = "camelCase")]
pub(crate) enum LinkToJs {
    /// A destination in this document.
    Dest {
        /// Where it goes; left out when it names no page here.
        #[serde(skip_serializing_if = "Option::is_none")]
        target: Option<TargetJs>,
    },
    /// A named destination.
    Named {
        /// Its name.
        name: String,
        /// Where it goes; left out when the name names no page here.
        #[serde(skip_serializing_if = "Option::is_none")]
        target: Option<TargetJs>,
    },
    /// A URI, as stored (not checked).
    Uri {
        /// The address.
        uri: String,
    },
    /// A place in another file.
    Remote {
        /// The file's name, when readable.
        #[serde(skip_serializing_if = "Option::is_none")]
        file: Option<String>,
    },
    /// A named action.
    NamedAction {
        /// `NextPage`, `PrevPage`, `FirstPage`, `LastPage` or another name.
        name: String,
    },
    /// An action a reader never runs.
    Unsupported {
        /// Its type: `Launch`, `JavaScript`, `SubmitForm`…
        action: String,
    },
}

#[wasm_bindgen(typescript_custom_section)]
const PAGE_LINK_TS: &str = r#"
/** One link on a page, in page space, and what following it does (by `kind`). */
export type PageLinkJs = {
  /** Its rectangle: `[x0, y0, x1, y1]`, top-left and bottom-right. */
  rect: [number, number, number, number];
  /** The areas that follow it, eight numbers per area; absent when the rectangle is the area. */
  quads?: number[];
  /** Its description (`/Contents`, document text). */
  contents?: string;
} & LinkToJs;
"#;

impl From<&PageLink> for PageLinkJs {
    fn from(link: &PageLink) -> Self {
        let r = &link.rect;
        let to = match &link.to {
            LinkTo::Destination {
                name: Some(name),
                target,
            } => LinkToJs::Named {
                name: name.clone(),
                target: target.map(TargetJs::from),
            },
            LinkTo::Destination { name: None, target } => LinkToJs::Dest {
                target: target.map(TargetJs::from),
            },
            LinkTo::Uri(uri) => LinkToJs::Uri { uri: uri.clone() },
            LinkTo::OtherFile(file) => LinkToJs::Remote { file: file.clone() },
            LinkTo::NamedAction(name) => LinkToJs::NamedAction { name: name.clone() },
            LinkTo::Unsupported(action) => LinkToJs::Unsupported { action: action.clone() },
            // A kind a later façade adds: never run.
            _ => LinkToJs::Unsupported {
                action: "Unknown".to_owned(),
            },
        };
        Self {
            rect: [r.x0.get(), r.y0.get(), r.x1.get(), r.y1.get()],
            quads: link
                .quads
                .iter()
                .flat_map(|q| q.iter().flat_map(|p| [p.x.get(), p.y.get()]))
                .collect(),
            contents: link.contents.clone(),
            to,
        }
    }
}

#[wasm_bindgen]
impl WasmDocument {
    /// The visible links on page `index`, in annotation order (the extract and text modules).
    ///
    /// # Errors
    /// `page-index-out-of-range`, `data-pending` and the reading errors.
    #[wasm_bindgen(js_name = pageLinks, unchecked_return_type = "PageLinkJs[]")]
    pub fn page_links(&self, index: u32) -> JsResult<JsValue> {
        let links = self.shared().borrow().page_links(index).map_err(|e| lamina_error(&e))?;
        Ok(to_js(&links.iter().map(PageLinkJs::from).collect::<Vec<_>>()))
    }

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

    /// Every named destination, sorted by name (the extract and text modules).
    ///
    /// # Errors
    /// `data-pending` and the reading errors.
    #[wasm_bindgen(js_name = namedDestinations, unchecked_return_type = "NamedDestinationJs[]")]
    pub fn named_destinations(&self) -> JsResult<JsValue> {
        let names = self
            .shared()
            .borrow()
            .named_destinations()
            .map_err(|e| lamina_error(&e))?;
        Ok(to_js(&names.iter().map(NamedDestinationJs::from).collect::<Vec<_>>()))
    }

    /// `view` (page space) on page `page` as the destination the outline and named-destination
    /// ops take, in the page's user space (the extract and text modules).
    ///
    /// # Errors
    /// `invalid-argument` for a view that isn't one; `page-index-out-of-range`; reading errors.
    #[wasm_bindgen(js_name = destinationAt, unchecked_return_type = "ExplicitDestination")]
    pub fn destination_at(
        &self,
        page: u32,
        #[wasm_bindgen(unchecked_param_type = "TargetViewJs")] view: &JsValue,
    ) -> JsResult<JsValue> {
        let view: TargetViewJs = from_js(view, "view")?;
        let dest = self
            .shared()
            .borrow()
            .destination_at(page, view.into())
            .map_err(|e| lamina_error(&e))?;
        Ok(to_js(&dest))
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
