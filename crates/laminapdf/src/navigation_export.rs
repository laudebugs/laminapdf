//! A document's links and outline as files (operation `H.08`): JSON in the formats
//! `laminapdf.links/1` and `laminapdf.outline/1` ([ADR 0031] §6: a `schema` id, camelCase keys,
//! pages by `pageIndex` and `pageNumber`, lengths in points rounded to 1/1000), and CSV for a
//! spreadsheet, one row a link or an outline item.
//!
//! [ADR 0031]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0031-cli-and-mcp-contract.md

use serde::Serialize;

use crate::document::Document;
use crate::error::Result;
use crate::links::{LinkTo, PageLink};
use crate::navigation::{OutlineNode, PageTarget, PageTargetView};

/// The links JSON's format.
///
/// Added in 0.4.0.
pub const LINKS_SCHEMA: &str = "laminapdf.links/1";

/// The outline JSON's format.
///
/// Added in 0.4.0.
pub const OUTLINE_SCHEMA: &str = "laminapdf.outline/1";

/// A link and the page it is on.
///
/// Added in 0.4.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct DocumentLink {
    /// The page's 0-based index.
    pub page: u32,
    /// The link.
    pub link: PageLink,
}

impl Document {
    /// Every page's visible links, page by page in each page's annotation order: what
    /// [`Document::page_links`] gives for each page.
    ///
    /// # Errors
    /// Reading errors (the annotations, named destinations, the page tree).
    ///
    /// Added in 0.4.0.
    pub fn links(&self) -> Result<Vec<DocumentLink>> {
        let count = self.page_count()?;
        let mut out = Vec::new();
        for page in 0..count {
            out.extend(
                self.page_links(page)?
                    .into_iter()
                    .map(|link| DocumentLink { page, link }),
            );
        }
        Ok(out)
    }

    /// [`Document::links`] as `laminapdf.links/1` JSON.
    ///
    /// # Errors
    /// As [`Document::links`].
    ///
    /// Added in 0.4.0.
    pub fn links_json(&self) -> Result<String> {
        let links: Vec<JsonLink> = self.links()?.iter().map(JsonLink::from).collect();
        Ok(to_json(&JsonLinks {
            schema: LINKS_SCHEMA,
            generator: GENERATOR,
            links,
        }))
    }

    /// [`Document::links`] as CSV: a header row, then a row a link (RFC 4180).
    ///
    /// # Errors
    /// As [`Document::links`].
    ///
    /// Added in 0.4.0.
    pub fn links_csv(&self) -> Result<String> {
        let mut csv = Csv::new(&[
            "pageNumber",
            "left",
            "top",
            "right",
            "bottom",
            "kind",
            "uri",
            "destination",
            "targetPage",
            "file",
            "action",
            "contents",
        ]);
        for link in self.links()?.iter().map(JsonLink::from) {
            csv.row(&[
                link.page_number.to_string(),
                number(link.rect[0]),
                number(link.rect[1]),
                number(link.rect[2]),
                number(link.rect[3]),
                link.kind.to_owned(),
                link.uri.unwrap_or_default(),
                link.destination.unwrap_or_default(),
                link.target.map(|t| t.page_number.to_string()).unwrap_or_default(),
                link.file.unwrap_or_default(),
                link.action.unwrap_or_default(),
                link.contents.unwrap_or_default(),
            ]);
        }
        Ok(csv.finish())
    }

    /// [`Document::outline`] as `laminapdf.outline/1` JSON: the items nested as the outline is.
    ///
    /// # Errors
    /// As [`Document::outline`].
    ///
    /// Added in 0.4.0.
    pub fn outline_json(&self) -> Result<String> {
        let items: Vec<JsonItem> = self.outline()?.iter().map(JsonItem::from).collect();
        Ok(to_json(&JsonOutline {
            schema: OUTLINE_SCHEMA,
            generator: GENERATOR,
            items,
        }))
    }

    /// [`Document::outline`] as CSV: a header row, then a row an item in reading order, its
    /// depth in `level` (1 for a top-level item).
    ///
    /// # Errors
    /// As [`Document::outline`].
    ///
    /// Added in 0.4.0.
    pub fn outline_csv(&self) -> Result<String> {
        let mut csv = Csv::new(&[
            "level",
            "title",
            "targetPage",
            "destination",
            "uri",
            "action",
            "open",
            "bold",
            "italic",
            "color",
        ]);
        let items: Vec<JsonItem> = self.outline()?.iter().map(JsonItem::from).collect();
        let mut stack: Vec<(usize, &JsonItem)> = items.iter().rev().map(|item| (1, item)).collect();
        while let Some((level, item)) = stack.pop() {
            csv.row(&[
                level.to_string(),
                item.title.clone(),
                item.target
                    .as_ref()
                    .map(|t| t.page_number.to_string())
                    .unwrap_or_default(),
                item.destination.clone().unwrap_or_default(),
                item.uri.clone().unwrap_or_default(),
                item.action.clone().unwrap_or_default(),
                item.open.to_string(),
                item.bold.to_string(),
                item.italic.to_string(),
                item.color.clone().unwrap_or_default(),
            ]);
            stack.extend(item.children.iter().rev().map(|child| (level + 1, child)));
        }
        Ok(csv.finish())
    }
}

/// Who wrote the file.
#[derive(Debug, Clone, Copy, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
struct Generator {
    name: &'static str,
    version: &'static str,
}

const GENERATOR: Generator = Generator {
    name: "laminapdf",
    version: env!("CARGO_PKG_VERSION"),
};

/// `laminapdf.links/1`.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
struct JsonLinks {
    /// `laminapdf.links/1`.
    schema: &'static str,
    generator: Generator,
    /// Every page's links, page by page.
    links: Vec<JsonLink>,
}

/// One link.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
struct JsonLink {
    page_index: u32,
    page_number: u32,
    /// Left, top, right and bottom, in points from the displayed page's top-left, y down.
    rect: [f64; 4],
    /// `destination`, `uri`, `otherFile`, `namedAction` or `unsupported`.
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    uri: Option<String>,
    /// The named destination it goes to.
    #[serde(skip_serializing_if = "Option::is_none")]
    destination: Option<String>,
    /// Where it goes, when that is a page of this document.
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<JsonTarget>,
    /// The other file a `GoToR` link opens.
    #[serde(skip_serializing_if = "Option::is_none")]
    file: Option<String>,
    /// A named action (`NextPage`…), or the type of an action a reader never runs.
    #[serde(skip_serializing_if = "Option::is_none")]
    action: Option<String>,
    /// Its description (`/Contents`).
    #[serde(skip_serializing_if = "Option::is_none")]
    contents: Option<String>,
}

impl From<&DocumentLink> for JsonLink {
    fn from(DocumentLink { page, link }: &DocumentLink) -> Self {
        let mut out = Self {
            page_index: *page,
            page_number: page + 1,
            rect: [
                link.rect.x0.get(),
                link.rect.y0.get(),
                link.rect.x1.get(),
                link.rect.y1.get(),
            ]
            .map(round),
            kind: "",
            uri: None,
            destination: None,
            target: None,
            file: None,
            action: None,
            contents: link.contents.clone(),
        };
        match &link.to {
            LinkTo::Destination { name, target } => {
                out.kind = "destination";
                out.destination.clone_from(name);
                out.target = target.as_ref().map(JsonTarget::from);
            }
            LinkTo::Uri(uri) => {
                out.kind = "uri";
                out.uri = Some(uri.clone());
            }
            LinkTo::OtherFile(file) => {
                out.kind = "otherFile";
                out.file.clone_from(file);
            }
            LinkTo::NamedAction(name) => {
                out.kind = "namedAction";
                out.action = Some(name.clone());
            }
            LinkTo::Unsupported(kind) => {
                out.kind = "unsupported";
                out.action = Some(kind.clone());
            }
        }
        out
    }
}

/// A page of this document and how to show it.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
struct JsonTarget {
    page_index: u32,
    page_number: u32,
    /// `at`, `fit`, `fitWidth`, `fitHeight` or `fitRect`.
    view: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    left: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    right: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bottom: Option<f64>,
    /// 1 is 100 %.
    #[serde(skip_serializing_if = "Option::is_none")]
    zoom: Option<f64>,
}

impl From<&PageTarget> for JsonTarget {
    fn from(target: &PageTarget) -> Self {
        let mut out = Self {
            page_index: target.page,
            page_number: target.page + 1,
            view: "at",
            left: None,
            top: None,
            right: None,
            bottom: None,
            zoom: None,
        };
        match target.view {
            PageTargetView::At { x, y, zoom } => {
                (out.left, out.top, out.zoom) = (x.map(round), y.map(round), zoom);
            }
            PageTargetView::Fit => out.view = "fit",
            PageTargetView::FitWidth { y } => (out.view, out.top) = ("fitWidth", y.map(round)),
            PageTargetView::FitHeight { x } => (out.view, out.left) = ("fitHeight", x.map(round)),
            PageTargetView::FitRect { x0, y0, x1, y1 } => {
                out.view = "fitRect";
                (out.left, out.top, out.right, out.bottom) =
                    (Some(round(x0)), Some(round(y0)), Some(round(x1)), Some(round(y1)));
            }
        }
        out
    }
}

/// `laminapdf.outline/1`.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
struct JsonOutline {
    /// `laminapdf.outline/1`.
    schema: &'static str,
    generator: Generator,
    /// The top-level items, each with its children.
    items: Vec<JsonItem>,
}

/// One outline item.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
struct JsonItem {
    title: String,
    /// Whether it opens with its children shown.
    open: bool,
    bold: bool,
    italic: bool,
    /// `#rrggbb`, when it isn't the default black.
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    destination: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<JsonTarget>,
    #[serde(skip_serializing_if = "Option::is_none")]
    uri: Option<String>,
    /// The type of an action that is neither a `GoTo` nor a `URI`.
    #[serde(skip_serializing_if = "Option::is_none")]
    action: Option<String>,
    children: Vec<Self>,
}

impl From<&OutlineNode> for JsonItem {
    fn from(node: &OutlineNode) -> Self {
        Self {
            title: node.title.clone(),
            open: node.open,
            bold: node.bold,
            italic: node.italic,
            color: node.color.map(|c| format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)),
            destination: node.destination_name.clone(),
            target: node.target.as_ref().map(JsonTarget::from),
            uri: node.uri.clone(),
            action: node.other_action.clone(),
            children: node.children.iter().map(Self::from).collect(),
        }
    }
}

/// A length rounded to 1/1000 of a point (ADR 0030 §7, which ADR 0031 follows).
fn round(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

/// A CSV cell's number: as JSON writes it.
fn number(value: f64) -> String {
    serde_json::Number::from_f64(value).map_or_else(String::new, |n| n.to_string())
}

fn to_json(value: &impl Serialize) -> String {
    // The types here are plain data: serializing them can't fail.
    serde_json::to_string_pretty(value).unwrap_or_default() + "\n"
}

/// RFC 4180 CSV: comma-separated, CRLF lines, a cell quoted when it holds a comma, a quote or
/// a line break, its quotes doubled.
struct Csv(String);

impl Csv {
    fn new(header: &[&str]) -> Self {
        let mut csv = Self(String::new());
        csv.row(&header.iter().map(|h| (*h).to_owned()).collect::<Vec<_>>());
        csv
    }

    fn row(&mut self, cells: &[String]) {
        for (i, cell) in cells.iter().enumerate() {
            if i > 0 {
                self.0.push(',');
            }
            if cell.contains([',', '"', '\n', '\r']) {
                self.0.push('"');
                self.0.push_str(&cell.replace('"', "\"\""));
                self.0.push('"');
            } else {
                self.0.push_str(cell);
            }
        }
        self.0.push_str("\r\n");
    }

    fn finish(self) -> String {
        self.0
    }
}

/// The JSON Schemas (2020-12) of [`Document::links_json`] and [`Document::outline_json`], as
/// `tests/navigation_export.rs` commits them: `schemas/links.schema.json` and
/// `schemas/outline.schema.json`.
///
/// Added in 0.4.0.
#[cfg(feature = "schema")]
#[must_use]
pub fn navigation_schemas() -> [(&'static str, serde_json::Value); 2] {
    let with_id = |mut schema: serde_json::Value, file: &str, title: &str| {
        if let Some(object) = schema.as_object_mut() {
            object.insert(
                "$id".to_owned(),
                serde_json::Value::String(format!(
                    "https://github.com/laudebugs/laminapdf/blob/main/crates/laminapdf/schemas/{file}"
                )),
            );
            object.insert("title".to_owned(), serde_json::Value::String(title.to_owned()));
        }
        schema
    };
    [
        (
            "links.schema.json",
            with_id(
                schemars::schema_for!(JsonLinks).to_value(),
                "links.schema.json",
                LINKS_SCHEMA,
            ),
        ),
        (
            "outline.schema.json",
            with_id(
                schemars::schema_for!(JsonOutline).to_value(),
                "outline.schema.json",
                OUTLINE_SCHEMA,
            ),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_quotes_only_the_cells_that_need_it() {
        let mut csv = Csv::new(&["a", "b"]);
        csv.row(&["plain".to_owned(), "say \"hi\", twice".to_owned()]);
        assert_eq!(csv.finish(), "a,b\r\nplain,\"say \"\"hi\"\", twice\"\r\n");
    }
}
