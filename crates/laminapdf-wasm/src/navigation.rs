//! Moving through a document across the boundary: `WasmDocument.outline()`,
//! `namedDestination(name)`, `pageLinks(index)`, `initialView()` and `threads()`, each target a
//! page index and a view in page space (the extract and text modules, so the base module stays in
//! its size budget).

use laminapdf::{
    ArticleThread, LinkTo, OpenTarget, OpeningView, OutlineNode, PageLayout, PageLink, PageMode, PageTarget,
    PageTargetView,
};
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

/// How a document asks to be shown when it opens.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InitialViewJs {
    /// Where it opens.
    pub open: OpenTargetJs,
    /// The panel to show; left out when the file leaves it to the reader.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(
        optional,
        type = "\"none\" | \"outlines\" | \"thumbnails\" | \"fullScreen\" | \"optionalContent\" | \"attachments\""
    )]
    pub page_mode: Option<&'static str>,
    /// How to lay the pages out; left out when the file leaves it to the reader.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(
        optional,
        type = "\"singlePage\" | \"oneColumn\" | \"twoColumnLeft\" | \"twoColumnRight\" | \"twoPageLeft\" | \"twoPageRight\""
    )]
    pub page_layout: Option<&'static str>,
    /// The window's viewer preferences.
    pub preferences: WindowPreferencesJs,
}

/// Where a document opens, by `kind`.
#[derive(Debug, Serialize, Tsify)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(crate) enum OpenTargetJs {
    /// The first page at the reader's zoom.
    Default,
    /// A page and how to show it.
    Page {
        /// The page.
        target: TargetJs,
    },
    /// An action a reader runs, which Lamina PDF never does, by its type (`JavaScript`, `Named`, …).
    Action {
        /// Its type.
        action: String,
    },
}

/// The yes-or-no window entries of the viewer preferences.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the six booleans the file stores, one field each"
)]
pub(crate) struct WindowPreferencesJs {
    /// Hide the reader's toolbars.
    pub hide_toolbar: bool,
    /// Hide the reader's menu bar.
    pub hide_menubar: bool,
    /// Hide the scroll bars and navigation controls.
    #[serde(rename = "hideWindowUi")]
    pub hide_window_ui: bool,
    /// Resize the window to fit the first page.
    pub fit_window: bool,
    /// Put the window in the middle of the screen.
    pub center_window: bool,
    /// Show the document's title in the title bar.
    pub display_doc_title: bool,
}

/// An article thread: its title and its parts in reading order.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ThreadJs {
    /// Its title (document text), when it has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Its parts.
    pub parts: Vec<ThreadPartJs>,
}

/// One part of an article thread.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ThreadPartJs {
    /// The page's 0-based index; left out when the part names a page the document lacks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    /// Its rectangle, `[x0, y0, x1, y1]` in page space; left out when unreadable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rect: Option<[f64; 4]>,
}

/// A page mode's wire name, as `SetInitialView` takes it.
const fn mode_name(mode: PageMode) -> &'static str {
    match mode {
        PageMode::Outlines => "outlines",
        PageMode::Thumbnails => "thumbnails",
        PageMode::FullScreen => "fullScreen",
        PageMode::OptionalContent => "optionalContent",
        PageMode::Attachments => "attachments",
        _ => "none",
    }
}

/// A page layout's wire name, as `SetInitialView` takes it.
const fn layout_name(layout: PageLayout) -> &'static str {
    match layout {
        PageLayout::OneColumn => "oneColumn",
        PageLayout::TwoColumnLeft => "twoColumnLeft",
        PageLayout::TwoColumnRight => "twoColumnRight",
        PageLayout::TwoPageLeft => "twoPageLeft",
        PageLayout::TwoPageRight => "twoPageRight",
        _ => "singlePage",
    }
}

impl From<&OpeningView> for InitialViewJs {
    fn from(view: &OpeningView) -> Self {
        let p = view.preferences;
        Self {
            open: match &view.open {
                OpenTarget::Page(target) => OpenTargetJs::Page {
                    target: TargetJs::from(*target),
                },
                OpenTarget::Action(action) => OpenTargetJs::Action { action: action.clone() },
                _ => OpenTargetJs::Default,
            },
            page_mode: view.page_mode.map(mode_name),
            page_layout: view.page_layout.map(layout_name),
            preferences: WindowPreferencesJs {
                hide_toolbar: p.hide_toolbar,
                hide_menubar: p.hide_menubar,
                hide_window_ui: p.hide_window_ui,
                fit_window: p.fit_window,
                center_window: p.center_window,
                display_doc_title: p.display_doc_title,
            },
        }
    }
}

impl From<&ArticleThread> for ThreadJs {
    fn from(thread: &ArticleThread) -> Self {
        Self {
            title: thread.title.clone(),
            parts: thread
                .parts
                .iter()
                .map(|part| ThreadPartJs {
                    page: part.page,
                    rect: part.rect.map(|r| [r.x0.get(), r.y0.get(), r.x1.get(), r.y1.get()]),
                })
                .collect(),
        }
    }
}

#[wasm_bindgen]
impl WasmDocument {
    /// How the document asks to be shown when it opens (the extract and text modules).
    ///
    /// # Errors
    /// `data-pending` and the reading errors.
    #[wasm_bindgen(js_name = initialView, unchecked_return_type = "InitialViewJs")]
    pub fn initial_view(&self) -> JsResult<JsValue> {
        let view = self.shared().borrow().initial_view().map_err(|e| lamina_error(&e))?;
        Ok(to_js(&InitialViewJs::from(&view)))
    }

    /// The document's article threads; empty without any (the extract and text modules).
    ///
    /// # Errors
    /// `data-pending` and the reading errors.
    #[wasm_bindgen(unchecked_return_type = "ThreadJs[]")]
    pub fn threads(&self) -> JsResult<JsValue> {
        let threads = self.shared().borrow().threads().map_err(|e| lamina_error(&e))?;
        Ok(to_js(&threads.iter().map(ThreadJs::from).collect::<Vec<_>>()))
    }

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
