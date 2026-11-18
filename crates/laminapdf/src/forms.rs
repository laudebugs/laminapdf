//! The interactive form (ISO 32000-2 §12.7) as records: each field with its value, options and
//! widgets in page space (ADR 0027), what a form panel and a filler read before
//! [`SetFieldValue`](crate::SetFieldValue) changes a value; and the order Tab visits a page's
//! widgets in.

use std::collections::BTreeMap;

use laminapdf_model::{ChoiceOption, FieldKind, FieldValue, PageFrame, PageId, Pages, Tabs};

use crate::document::Document;
use crate::error::{Error, Result};

/// The form's fields, and what its field tree and the pages' widgets disagree on.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct FormFields {
    /// The terminal fields, in the order the form lists them. Fields without a type are left
    /// out, and objects that share a fully qualified name are one field.
    pub fields: Vec<FormField>,
    /// The IDs of widgets a page shows that no field of the form reaches (`a12.0`).
    pub orphan_widgets: Vec<String>,
    /// The IDs of fields none of whose widgets a page shows.
    pub widgetless: Vec<String>,
    /// Whether the form asks viewers to draw every widget again (`/NeedAppearances`).
    pub need_appearances: bool,
    /// Whether the form has an XFA part, which Lamina PDF doesn't lay out.
    pub has_xfa: bool,
}

/// A form field: one the form lists with no field below it.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct FormField {
    /// Its ID (`f12.0`), as [`SetFieldValue`](crate::SetFieldValue) takes it.
    pub id: String,
    /// The IDs of other objects with the same name, which share its value.
    pub shared: Vec<String>,
    /// Its fully qualified name (`person.first`), document text.
    pub name: String,
    /// The name the interface shows (`/TU`), document text.
    pub tooltip: Option<String>,
    /// The kind of field.
    pub kind: FieldKind,
    /// Its flags (`/Ff`, ISO 32000-2 Tables 227–233).
    pub flags: u32,
    /// Whether the interface may change it (`ReadOnly`); the library's ops may.
    pub read_only: bool,
    /// Whether it needs a value before the form is submitted (`Required`).
    pub required: bool,
    /// Its value; `None` when it has none.
    pub value: Option<FieldValue>,
    /// The value resetting the form gives it.
    pub default_value: Option<FieldValue>,
    /// The longest text it takes, in characters (`/MaxLen`).
    pub max_len: Option<u32>,
    /// A list or combo box's options, or a button group's export values.
    pub options: Vec<ChoiceOption>,
    /// The states a check box's or radio group's widgets turn on to, each once.
    pub on_states: Vec<String>,
    /// Whether a signature field holds a signature.
    pub signed: bool,
    /// Its widgets.
    pub widgets: Vec<FormWidget>,
}

/// One widget of a form field.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct FormWidget {
    /// Its ID (`a12.0`).
    pub id: String,
    /// The index of the page that shows it; `None` when no page does.
    pub page: Option<u32>,
    /// Its rectangle, in that page's space.
    pub rect: Option<crate::RectPt>,
    /// Its annotation flags (`/F`, ISO 32000-2 Table 167).
    pub flags: u32,
    /// For a check box or radio button, the state it turns on to.
    pub on_state: Option<String>,
    /// The state it shows (`/AS`).
    pub state: Option<String>,
}

/// The order Tab visits a page's widgets in.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PageTabOrder {
    /// The order the page asks for (`/Tabs`); `None` when it asks for none this reader knows,
    /// and the widgets come in the order the page lists them.
    pub tabs: Option<Tabs>,
    /// The page's widgets in that order.
    pub widgets: Vec<TabStop>,
}

/// A widget in the tab order.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct TabStop {
    /// The widget's ID (`a12.0`).
    pub widget: String,
    /// The ID of the field it belongs to (`f12.0`).
    pub field: String,
}

impl Document {
    /// The form's fields with their values and widgets ([`FormFields`]): empty for a document
    /// without a form.
    ///
    /// ```
    /// # #[cfg(feature = "build")] {
    /// use laminapdf::{BuildOptions, Builder, PageSize};
    ///
    /// let mut b = Builder::new(BuildOptions::new().with_id(*b"form-fields-0001"));
    /// b.page(PageSize::A4).finish()?;
    /// let doc = b.finish()?;
    /// assert!(doc.form()?.fields.is_empty());
    /// # }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// Reading errors (the catalog, the field tree, the pages).
    ///
    /// Added in 0.6.0.
    pub fn form(&self) -> Result<FormFields> {
        let doc = self.ops();
        let form = doc.form()?;
        let pages = doc.pages()?;
        let mut frames = BTreeMap::new();
        let mut index_of: BTreeMap<PageId, u32> = BTreeMap::new();
        for page in pages.iter() {
            index_of.insert(page.id().clone(), u32::try_from(page.index()).unwrap_or(u32::MAX));
        }
        let mut fields = Vec::with_capacity(form.fields.len());
        for field in form.fields {
            let mut widgets = Vec::with_capacity(field.widgets.len());
            for w in &field.widgets {
                let page = w.page.as_ref().and_then(|p| index_of.get(p).copied());
                let rect = match (&w.page, w.rect) {
                    (Some(id), Some(rect)) => frame(&mut frames, &pages, id)?.map(|f| f.rect_to_page(&rect)),
                    _ => None,
                };
                widgets.push(FormWidget {
                    id: w.id.to_string(),
                    page,
                    rect,
                    flags: w.flags.0,
                    on_state: w.on_state.clone(),
                    state: w.state.clone(),
                });
            }
            fields.push(FormField {
                id: field.id.to_string(),
                shared: field.shared.iter().map(ToString::to_string).collect(),
                tooltip: field.tooltip.clone(),
                kind: field.kind,
                flags: field.flags.0,
                read_only: field.read_only(),
                required: field.required(),
                on_states: field.on_states().into_iter().map(str::to_owned).collect(),
                max_len: field.max_len,
                signed: field.signed,
                name: field.name,
                value: field.value,
                default_value: field.default_value,
                options: field.options,
                widgets,
            });
        }
        Ok(FormFields {
            fields,
            orphan_widgets: form.orphan_widgets.iter().map(|o| o.widget.to_string()).collect(),
            widgetless: form.widgetless.iter().map(ToString::to_string).collect(),
            need_appearances: form.need_appearances,
            has_xfa: form.has_xfa,
        })
    }

    /// The order Tab visits page `index`'s widgets in ([`PageTabOrder`]): row, column or
    /// structure order as the page's `/Tabs` asks, else the order the page lists them.
    ///
    /// # Errors
    /// `page-index-out-of-range`; reading errors (the page, its widgets, the structure tree).
    ///
    /// Added in 0.6.0.
    pub fn tab_order(&self, index: u32) -> Result<PageTabOrder> {
        let doc = self.ops();
        let pages = doc.pages()?;
        let page = usize::try_from(index)
            .ok()
            .and_then(|i| pages.get(i))
            .ok_or(Error::PageIndexOutOfRange {
                index,
                count: u32::try_from(pages.len()).unwrap_or(u32::MAX),
            })?;
        let order = doc.tab_order(page.id())?;
        Ok(PageTabOrder {
            tabs: order.tabs,
            widgets: order
                .widgets
                .iter()
                .map(|w| TabStop {
                    widget: laminapdf_model::AnnotId::object(w.widget).to_string(),
                    field: laminapdf_model::FieldId::object(w.field).to_string(),
                })
                .collect(),
        })
    }
}

/// Page `id`'s frame, read once per page.
fn frame(frames: &mut BTreeMap<PageId, PageFrame>, pages: &Pages<'_>, id: &PageId) -> Result<Option<PageFrame>> {
    if let Some(frame) = frames.get(id) {
        return Ok(Some(*frame));
    }
    let Some(page) = pages.by_id(id) else { return Ok(None) };
    let frame = page.frame()?;
    frames.insert(id.clone(), frame);
    Ok(Some(frame))
}
