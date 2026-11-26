//! The navigation reads' answers in `@laminapdf/sdk`'s JSON shapes: the outline, the named
//! destinations, how a file opens, the files it carries and its XMP packet. The wasm crate writes
//! the same shapes for the web reader (`laminapdf-wasm`'s `navigation.rs`, `attachments.rs` and
//! `dto.rs`); the engine can't link that crate, so these mirror them field for field, and the
//! tests below pin the names the webview reads.

use laminapdf::{
    AfRelationship, AttachedFile, NamedDestination, OpenTarget, OpeningView, OutlineNode, PageId, PageLayout, PageMode,
    PageTarget, PageTargetView, XmpText,
};
use serde::Serialize;

/// One outline item (`OutlineNodeJs`).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OutlineData {
    id: String,
    title: String,
    open: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<TargetData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    destination_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    other_action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    bold: bool,
    italic: bool,
    children: Vec<Self>,
}

impl From<&OutlineNode> for OutlineData {
    fn from(node: &OutlineNode) -> Self {
        Self {
            id: node.id.to_string(),
            title: node.title.clone(),
            open: node.open,
            target: node.target.map(TargetData::from),
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

/// One named destination (`NamedDestinationJs`).
#[derive(Debug, Serialize)]
pub(crate) struct NamedDestinationData {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<TargetData>,
}

impl From<&NamedDestination> for NamedDestinationData {
    fn from(d: &NamedDestination) -> Self {
        Self {
            name: d.name.clone(),
            target: d.target.map(TargetData::from),
        }
    }
}

/// A page and how to show it (`TargetJs`).
#[derive(Debug, Serialize)]
pub(crate) struct TargetData {
    page: u32,
    view: ViewData,
}

/// How a target shows its page, by `kind` (`TargetViewJs`).
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum ViewData {
    At {
        #[serde(skip_serializing_if = "Option::is_none")]
        x: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        y: Option<f64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        zoom: Option<f64>,
    },
    Fit,
    FitWidth {
        #[serde(skip_serializing_if = "Option::is_none")]
        y: Option<f64>,
    },
    FitHeight {
        #[serde(skip_serializing_if = "Option::is_none")]
        x: Option<f64>,
    },
    FitRect {
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
    },
}

impl From<PageTarget> for TargetData {
    fn from(target: PageTarget) -> Self {
        let view = match target.view {
            PageTargetView::At { x, y, zoom } => ViewData::At { x, y, zoom },
            PageTargetView::Fit => ViewData::Fit,
            PageTargetView::FitWidth { y } => ViewData::FitWidth { y },
            PageTargetView::FitHeight { x } => ViewData::FitHeight { x },
            PageTargetView::FitRect { x0, y0, x1, y1 } => ViewData::FitRect { x0, y0, x1, y1 },
            // A view a later façade adds: the page's top.
            _ => ViewData::At {
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

/// How a file asks to be shown when it opens (`InitialViewJs`).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InitialViewData {
    open: OpenData,
    #[serde(skip_serializing_if = "Option::is_none")]
    page_mode: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    page_layout: Option<&'static str>,
    preferences: PreferencesData,
}

/// Where a file opens, by `kind` (`OpenTargetJs`).
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum OpenData {
    Default,
    Page { target: TargetData },
    Action { action: String },
}

/// The viewer preferences' yes-or-no entries (`WindowPreferencesJs`).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the six booleans the file stores, one field each"
)]
struct PreferencesData {
    hide_toolbar: bool,
    hide_menubar: bool,
    hide_window_ui: bool,
    fit_window: bool,
    center_window: bool,
    display_doc_title: bool,
}

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

impl From<&OpeningView> for InitialViewData {
    fn from(view: &OpeningView) -> Self {
        let p = view.preferences;
        Self {
            open: match &view.open {
                OpenTarget::Page(target) => OpenData::Page {
                    target: TargetData::from(*target),
                },
                OpenTarget::Action(action) => OpenData::Action { action: action.clone() },
                _ => OpenData::Default,
            },
            page_mode: view.page_mode.map(mode_name),
            page_layout: view.page_layout.map(layout_name),
            preferences: PreferencesData {
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

/// One attached file (`AttachmentJs`).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AttachmentData {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mime: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    created: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    modified: Option<String>,
    has_checksum: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    relationship: Option<&'static str>,
    embedded: bool,
    holders: Vec<HolderData>,
}

/// What holds an attached file (`HolderJs`).
#[derive(Debug, Serialize)]
struct HolderData {
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    page: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    annotation: Option<String>,
}

impl AttachmentData {
    /// `file` across the boundary; `pages` are the document's page IDs in order, which a holder
    /// names its page by.
    pub(crate) fn new(pages: &[PageId], file: AttachedFile) -> Self {
        let page = |id: &PageId| pages.iter().position(|p| p == id).and_then(|i| u32::try_from(i).ok());
        let holders = file
            .holders
            .iter()
            .map(|holder| HolderData {
                kind: holder.kind_name(),
                page: holder.page().and_then(page),
                annotation: holder.annotation().map(ToString::to_string),
            })
            .collect();
        Self {
            id: file.id.to_string(),
            key: file.key,
            name: file.name,
            description: file.description,
            mime: file.mime,
            size: file.size,
            created: file.created.map(|d| d.to_string()),
            modified: file.modified.map(|d| d.to_string()),
            has_checksum: file.checksum.is_some(),
            relationship: file.relationship.map(AfRelationship::camel_name),
            embedded: file.embedded,
            holders,
        }
    }
}

/// The XMP packet as text (`XmpTextJs`).
#[derive(Debug, Serialize)]
pub(crate) struct XmpData {
    status: laminapdf::PacketStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    xml: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pdfa: Option<String>,
}

impl From<XmpText> for XmpData {
    fn from(p: XmpText) -> Self {
        Self {
            status: p.status,
            xml: p.xml,
            pdfa: p.pdfa,
        }
    }
}
