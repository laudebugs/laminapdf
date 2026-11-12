//! `SetInitialView`: how the document asks to be shown when it opens (ISO 32000-2 §12.2,
//! §7.7.2 Table 29): the catalog's `/OpenAction`, `/PageMode`, `/PageLayout` and the
//! yes-or-no window entries of `/ViewerPreferences`.

use laminapdf_cos::{CosMut, Dict, Name, ObjRef, Object};
use laminapdf_model::{PageId, PageLayout, PageMode, WindowPreferences};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::error::{Error, Result};
use crate::wire;

/// Changes how the document asks to be shown when it opens. Each field is a patch: left out,
/// it keeps what the file has; `null` removes the entry, so readers use their default; a value
/// sets it.
///
/// * `open`: the page and zoom to open at, written as an explicit destination in
///   `/OpenAction` (§12.3.2.2). An `/OpenAction` that is an action other than `GoTo`
///   (JavaScript, a named action, a link to another file) is never replaced or removed: the op
///   fails with [`Error::OpenActionNotDestination`].
/// * `page_mode`, `page_layout`: the panel to show and how to lay the pages out.
/// * `preferences`: the window entries of `/ViewerPreferences` (§12.2, Table 147); `true` sets
///   one, `false` removes it (its default), and other entries (`/Direction`, `/PrintScaling`, …)
///   are kept.
///
/// When a value needs a later PDF version than the file declares (`UseAttachments` 1.6,
/// `UseOC` and the two-page layouts 1.5, `/DisplayDocTitle` 1.4), the catalog's `/Version` is
/// raised to it. The op is class [`Document`](crate::ModificationClass::Document) and touches no
/// page.
///
/// On the wire:
/// `{ "op": "setInitialView", "args": { "open": { "page": "p12.0", "zoom": "fitWidth" },
/// "pageMode": "outlines", "pageLayout": null, "preferences": { "displayDocTitle": true } } }`.
///
/// ```
/// use laminapdf_ops::{OpenAt, OpenZoom, SetInitialView, WindowPreferencesPatch};
/// use laminapdf_model::PageMode;
///
/// let op = SetInitialView::default()
///     .with_open(Some(OpenAt::new("p3.0".parse()?, OpenZoom::FitWidth)))
///     .with_page_mode(Some(PageMode::Outlines))
///     .with_preferences(WindowPreferencesPatch::default().with_display_doc_title(true));
/// assert_eq!(
///     serde_json::to_string(&op)?,
///     r#"{"open":{"page":"p3.0","zoom":"fitWidth"},"pageMode":"outlines","preferences":{"displayDocTitle":true}}"#
/// );
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct SetInitialView {
    /// The page and zoom to open at.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "OpenAt | null"))]
    pub open: Option<Option<OpenAt>>,
    /// The panel to show (`/PageMode`).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "page_mode::serialize"
    )]
    #[cfg_attr(
        feature = "tsify",
        tsify(
            optional,
            type = "\"none\" | \"outlines\" | \"thumbnails\" | \"fullScreen\" | \"optionalContent\" | \"attachments\" | null"
        )
    )]
    pub page_mode: Option<Option<PageMode>>,
    /// How to lay the pages out (`/PageLayout`).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "page_layout::serialize"
    )]
    #[cfg_attr(
        feature = "tsify",
        tsify(
            optional,
            type = "\"singlePage\" | \"oneColumn\" | \"twoColumnLeft\" | \"twoColumnRight\" | \"twoPageLeft\" | \"twoPageRight\" | null"
        )
    )]
    pub page_layout: Option<Option<PageLayout>>,
    /// The window entries of `/ViewerPreferences`.
    #[serde(default, skip_serializing_if = "WindowPreferencesPatch::is_empty")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub preferences: WindowPreferencesPatch,
}

/// The page and zoom a document opens at.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct OpenAt {
    /// The page.
    #[serde(serialize_with = "wire::page_id::serialize")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub page: PageId,
    /// The zoom.
    #[serde(serialize_with = "zoom::serialize")]
    #[cfg_attr(
        feature = "tsify",
        tsify(optional, type = "\"default\" | \"fitPage\" | \"fitWidth\" | number")
    )]
    pub zoom: OpenZoom,
}

impl OpenAt {
    /// Open at `page` with `zoom`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(page: PageId, zoom: OpenZoom) -> Self {
        Self { page, zoom }
    }
}

/// The zoom a document opens at (§12.3.2.2, Table 149).
///
/// On the wire: `"default"`, `"fitPage"`, `"fitWidth"`, or a percentage as a number (`125`).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OpenZoom {
    /// The reader's own zoom: `[page /XYZ null null null]`.
    #[default]
    ReaderDefault,
    /// The whole page: `[page /Fit]`.
    FitPage,
    /// The page's width: `[page /FitH null]`.
    FitWidth,
    /// A percentage, 1 to 6400: `[page /XYZ null null 1.25]` for 125.
    Percent(u16),
}

impl OpenZoom {
    /// The largest percentage: Acrobat's maximum zoom.
    ///
    /// Added in 0.2.0.
    pub const MAX_PERCENT: u16 = 6400;

    /// The destination array opening `page` at this zoom.
    fn destination(self, page: ObjRef) -> Object {
        let mut items = vec![Object::Ref(page)];
        match self {
            Self::ReaderDefault => items.extend([Object::name("XYZ"), Object::Null, Object::Null, Object::Null]),
            Self::FitPage => items.push(Object::name("Fit")),
            Self::FitWidth => items.extend([Object::name("FitH"), Object::Null]),
            Self::Percent(p) => items.extend([
                Object::name("XYZ"),
                Object::Null,
                Object::Null,
                Object::Real(f64::from(p) / 100.0),
            ]),
        }
        Object::Array(items)
    }
}

/// A patch of the window entries of `/ViewerPreferences` ([`WindowPreferences`]): each field
/// left out keeps what the file has, `true` sets the entry, `false` removes it.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct WindowPreferencesPatch {
    /// `/HideToolbar`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub hide_toolbar: Option<bool>,
    /// `/HideMenubar`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub hide_menubar: Option<bool>,
    /// `/HideWindowUI`.
    #[serde(default, rename = "hideWindowUi", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub hide_window_ui: Option<bool>,
    /// `/FitWindow`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub fit_window: Option<bool>,
    /// `/CenterWindow`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub center_window: Option<bool>,
    /// `/DisplayDocTitle`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub display_doc_title: Option<bool>,
}

impl WindowPreferencesPatch {
    /// Sets `/DisplayDocTitle`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_display_doc_title(mut self, value: bool) -> Self {
        self.display_doc_title = Some(value);
        self
    }

    /// Sets `/FitWindow`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_fit_window(mut self, value: bool) -> Self {
        self.fit_window = Some(value);
        self
    }

    /// Sets `/CenterWindow`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_center_window(mut self, value: bool) -> Self {
        self.center_window = Some(value);
        self
    }

    /// Sets `/HideToolbar`, `/HideMenubar` and `/HideWindowUI` together.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_hide_reader_controls(mut self, value: bool) -> Self {
        self.hide_toolbar = Some(value);
        self.hide_menubar = Some(value);
        self.hide_window_ui = Some(value);
        self
    }

    /// Whether the patch changes nothing.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// The value it gives the entry stored under `key`, one of [`WindowPreferences::KEYS`].
    fn get(self, key: &str) -> Option<bool> {
        match key {
            "HideToolbar" => self.hide_toolbar,
            "HideMenubar" => self.hide_menubar,
            "HideWindowUI" => self.hide_window_ui,
            "FitWindow" => self.fit_window,
            "CenterWindow" => self.center_window,
            "DisplayDocTitle" => self.display_doc_title,
            _ => None,
        }
    }
}

impl SetInitialView {
    /// Sets the page and zoom to open at; `None` removes an open destination.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn with_open(mut self, open: Option<OpenAt>) -> Self {
        self.open = Some(open);
        self
    }

    /// Sets the page mode; `None` removes it.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_page_mode(mut self, mode: Option<PageMode>) -> Self {
        self.page_mode = Some(mode);
        self
    }

    /// Sets the page layout; `None` removes it.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_page_layout(mut self, layout: Option<PageLayout>) -> Self {
        self.page_layout = Some(layout);
        self
    }

    /// Sets window preferences.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_preferences(mut self, preferences: WindowPreferencesPatch) -> Self {
        self.preferences = preferences;
        self
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<()> {
        let open = match &self.open {
            Some(open) => Some(Self::open_value(doc, open.as_ref())?),
            None => None,
        };
        let version = self.needed_version(doc)?;
        let mut cos = doc.cos_mut().edit();
        let (catalog_ref, mut catalog) = catalog(&cos)?;
        match open {
            Some(Some(dest)) => catalog.insert("OpenAction", dest),
            Some(None) => {
                catalog.remove("OpenAction");
            }
            None => {}
        }
        set_name(
            &mut catalog,
            "PageMode",
            self.page_mode.map(|m| m.map(PageMode::pdf_name)),
        );
        set_name(
            &mut catalog,
            "PageLayout",
            self.page_layout.map(|l| l.map(PageLayout::pdf_name)),
        );
        self.write_preferences(&mut cos, &mut catalog)?;
        if let Some((major, minor)) = version {
            catalog.insert("Version", Name::from(format!("{major}.{minor}").as_str()));
        }
        if let Some(r) = catalog_ref {
            cos.set(r, Object::Dict(catalog))?;
        } else {
            let mut trailer = cos.trailer().clone();
            trailer.insert("Root", Object::Dict(catalog));
            cos.set_trailer(trailer);
        }
        Ok(())
    }

    /// The `/OpenAction` value for `open` (`None`: remove it), refusing to replace an action
    /// that isn't a destination.
    fn open_value(doc: &laminapdf_model::Document, open: Option<&OpenAt>) -> Result<Option<Object>> {
        if doc.open_action_kind()?.is_some() {
            return Err(Error::OpenActionNotDestination);
        }
        let Some(open) = open else {
            return Ok(None);
        };
        let pages = doc.pages()?;
        let page = pages
            .by_id(&open.page)
            .ok_or_else(|| Error::UnknownPage(open.page.clone()))?;
        let r = page.obj_ref().ok_or_else(|| Error::DirectPage(open.page.clone()))?;
        Ok(Some(open.zoom.destination(r)))
    }

    /// The PDF version the values written need, when the file declares an earlier one.
    fn needed_version(&self, doc: &laminapdf_model::Document) -> Result<Option<(u8, u8)>> {
        let mut needed = (1, 0);
        if let Some(Some(mode)) = self.page_mode {
            needed = needed.max(mode.since());
        }
        if let Some(Some(layout)) = self.page_layout {
            needed = needed.max(layout.since());
        }
        if self.preferences.display_doc_title == Some(true) {
            needed = needed.max((1, 4));
        }
        let declared = doc.version()?.and_then(|v| {
            let (major, minor) = v.split_once('.')?;
            Some((major.parse::<u8>().ok()?, minor.parse::<u8>().ok()?))
        });
        Ok(match declared {
            Some(declared) if declared >= needed => None,
            None if needed == (1, 0) => None,
            _ => Some(needed),
        })
    }

    /// Applies the preferences patch to `/ViewerPreferences`: in its own object when it is
    /// indirect, else in the catalog, which drops the entry once it is empty.
    fn write_preferences(&self, cos: &mut CosMut<'_>, catalog: &mut Dict) -> Result<()> {
        if self.preferences.is_empty() {
            return Ok(());
        }
        let existing = catalog.get("ViewerPreferences").cloned();
        let indirect = existing.as_ref().and_then(Object::as_ref);
        let current = match (&existing, indirect) {
            (_, Some(r)) => cos.get(r)?.and_then(|o| o.as_dict().cloned()),
            (Some(Object::Dict(d)), None) => Some(d.clone()),
            _ => None,
        };
        let mut prefs = current.unwrap_or_default();
        for key in WindowPreferences::KEYS {
            match self.preferences.get(key) {
                Some(true) => prefs.insert(key, true),
                Some(false) => {
                    prefs.remove(key);
                }
                None => {}
            }
        }
        match indirect {
            Some(r) => cos.set(r, Object::Dict(prefs))?,
            None if prefs.is_empty() => {
                catalog.remove("ViewerPreferences");
            }
            None => catalog.insert("ViewerPreferences", prefs),
        }
        Ok(())
    }
}

/// The catalog's object (when indirect) and dictionary.
fn catalog(cos: &CosMut<'_>) -> Result<(Option<ObjRef>, Dict)> {
    match cos.trailer().get("Root").cloned() {
        Some(Object::Ref(r)) => match cos.get(r)?.as_deref() {
            Some(Object::Dict(d)) => Ok((Some(r), d.clone())),
            _ => Err(Error::Cos(laminapdf_cos::Error::Type {
                expected: "dictionary",
                context: "/Root",
            })),
        },
        Some(Object::Dict(d)) => Ok((None, d)),
        _ => Err(Error::Cos(laminapdf_cos::Error::MissingRoot)),
    }
}

/// Applies a name patch: `None` keeps the entry, `Some(None)` removes it, `Some(Some(_))` sets it.
#[expect(clippy::option_option, reason = "absent, null and a value are three cases")]
fn set_name(dict: &mut Dict, key: &'static str, value: Option<Option<&'static str>>) {
    match value {
        Some(Some(name)) => dict.insert(key, Name::from(name)),
        Some(None) => {
            dict.remove(key);
        }
        None => {}
    }
}

/// A name table for a patch field: the wire's camelCase names, in the type's `ALL` order.
macro_rules! named_patch {
    ($module:ident, $ty:ident, [$($name:literal),* $(,)?]) => {
        /// The patch field's wire names: absent keeps it, `null` removes it, a camelCase name sets it.
        mod $module {
            use serde::Serializer;

            use super::$ty;

            pub(super) const NAMES: [&str; <$ty>::ALL.len()] = [$($name),*];

            pub(super) fn name(value: $ty) -> &'static str {
                <$ty>::ALL
                    .iter()
                    .zip(NAMES)
                    .find_map(|(v, n)| (*v == value).then_some(n))
                    .unwrap_or_default()
            }

            #[expect(clippy::ref_option, reason = "serde's `with` passes the field by reference")]
            pub(super) fn serialize<S: Serializer>(value: &Option<Option<$ty>>, s: S) -> Result<S::Ok, S::Error> {
                match value {
                    Some(Some(v)) => s.serialize_str(name(*v)),
                    Some(None) | None => s.serialize_none(),
                }
            }
        }
    };
}

named_patch!(
    page_mode,
    PageMode,
    [
        "none",
        "outlines",
        "thumbnails",
        "fullScreen",
        "optionalContent",
        "attachments"
    ]
);
named_patch!(
    page_layout,
    PageLayout,
    [
        "singlePage",
        "oneColumn",
        "twoColumnLeft",
        "twoColumnRight",
        "twoPageLeft",
        "twoPageRight"
    ]
);

/// An [`OpenZoom`] as `"default"`, `"fitPage"`, `"fitWidth"` or a percentage, 1 to
/// [`OpenZoom::MAX_PERCENT`].
mod zoom {
    use serde::ser::Serializer;

    use super::OpenZoom;

    #[expect(
        clippy::trivially_copy_pass_by_ref,
        reason = "serde's `with` passes the field by reference"
    )]
    pub(super) fn serialize<S: Serializer>(zoom: &OpenZoom, s: S) -> Result<S::Ok, S::Error> {
        match zoom {
            OpenZoom::ReaderDefault => s.serialize_str("default"),
            OpenZoom::FitPage => s.serialize_str("fitPage"),
            OpenZoom::FitWidth => s.serialize_str("fitWidth"),
            OpenZoom::Percent(p) => s.serialize_u16(*p),
        }
    }
}

// Reading the wire form. Each type reads its JSON value through one non-generic function, so the
// WebAssembly modules hold one copy of the parsing code rather than one per serde deserializer
// that reaches an op (the JSON reader, a `Value`, serde's buffered content).

impl<'de> Deserialize<'de> for SetInitialView {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        Self::from_value(Value::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}

impl<'de> Deserialize<'de> for OpenAt {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        Self::from_value(Value::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}

impl<'de> Deserialize<'de> for WindowPreferencesPatch {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        Self::from_value(Value::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}

/// An object's entries, refusing any key outside `fields` (ADR 0006 §1: unknown fields are
/// rejected), as serde's `deny_unknown_fields` words it.
fn entries(value: Value, what: &str, fields: &[&str]) -> std::result::Result<serde_json::Map<String, Value>, String> {
    let Value::Object(map) = value else {
        return Err(format!("invalid type: {}, expected {what}", kind_of(&value)));
    };
    if let Some(key) = map.keys().find(|k| !fields.contains(&k.as_str())) {
        return Err(format!("unknown field `{key}`, expected one of {}", quoted(fields)));
    }
    Ok(map)
}

/// A JSON value's kind, in serde's words.
const fn kind_of(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "sequence",
        Value::Object(_) => "map",
    }
}

/// `names` as serde lists expected names: `` `a`, `b` ``.
fn quoted(names: &[&str]) -> String {
    names.iter().map(|n| format!("`{n}`")).collect::<Vec<_>>().join(", ")
}

/// A patch field naming one of `names`: `None` when absent, `Some(None)` for `null`.
#[expect(clippy::option_option, reason = "absent, null and a value are three cases")]
fn named<T: Copy>(value: Option<Value>, all: &[T], names: &[&str]) -> std::result::Result<Option<Option<T>>, String> {
    match value {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(s)) => names
            .iter()
            .position(|n| *n == s)
            .and_then(|i| all.get(i).copied())
            .map(|v| Some(Some(v)))
            .ok_or_else(|| format!("unknown variant `{s}`, expected one of {}", quoted(names))),
        Some(other) => Err(format!("invalid type: {}, expected a string or null", kind_of(&other))),
    }
}

impl SetInitialView {
    const FIELDS: [&'static str; 4] = ["open", "pageMode", "pageLayout", "preferences"];

    fn from_value(value: Value) -> std::result::Result<Self, String> {
        let mut map = entries(value, "the arguments of setInitialView", &Self::FIELDS)?;
        let open = match map.remove("open") {
            None => None,
            Some(Value::Null) => Some(None),
            Some(open) => Some(Some(OpenAt::from_value(open)?)),
        };
        Ok(Self {
            open,
            page_mode: named(map.remove("pageMode"), &PageMode::ALL, &page_mode::NAMES)?,
            page_layout: named(map.remove("pageLayout"), &PageLayout::ALL, &page_layout::NAMES)?,
            preferences: match map.remove("preferences") {
                Some(p) => WindowPreferencesPatch::from_value(p)?,
                None => WindowPreferencesPatch::default(),
            },
        })
    }
}

impl OpenAt {
    const FIELDS: [&'static str; 2] = ["page", "zoom"];

    fn from_value(value: Value) -> std::result::Result<Self, String> {
        let mut map = entries(value, "an open page and zoom", &Self::FIELDS)?;
        let page = match map.remove("page") {
            Some(Value::String(id)) => id.parse::<PageId>().map_err(|e| e.to_string())?,
            Some(other) => return Err(format!("invalid type: {}, expected a page ID", kind_of(&other))),
            None => return Err("missing field `page`".to_owned()),
        };
        let zoom = match map.remove("zoom") {
            None => OpenZoom::default(),
            Some(zoom) => OpenZoom::from_value(&zoom)?,
        };
        Ok(Self { page, zoom })
    }
}

impl OpenZoom {
    fn from_value(value: &Value) -> std::result::Result<Self, String> {
        const EXPECTED: &str = "\"default\", \"fitPage\", \"fitWidth\" or a percentage from 1 to 6400";
        match value {
            Value::String(s) => match s.as_str() {
                "default" => Ok(Self::ReaderDefault),
                "fitPage" => Ok(Self::FitPage),
                "fitWidth" => Ok(Self::FitWidth),
                _ => Err(format!("unknown variant `{s}`, expected {EXPECTED}")),
            },
            Value::Number(n) => n
                .as_u64()
                .and_then(|n| u16::try_from(n).ok())
                .filter(|p| (1..=Self::MAX_PERCENT).contains(p))
                .map(Self::Percent)
                .ok_or_else(|| format!("invalid value: {n}, expected {EXPECTED}")),
            other => Err(format!("invalid type: {}, expected {EXPECTED}", kind_of(other))),
        }
    }
}

impl WindowPreferencesPatch {
    /// The wire names, in [`WindowPreferences::KEYS`]' order.
    const FIELDS: [&'static str; 6] = [
        "hideToolbar",
        "hideMenubar",
        "hideWindowUi",
        "fitWindow",
        "centerWindow",
        "displayDocTitle",
    ];

    fn from_value(value: Value) -> std::result::Result<Self, String> {
        let mut map = entries(value, "window preferences", &Self::FIELDS)?;
        let mut flag = |key: &str| match map.remove(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Bool(b)) => Ok(Some(b)),
            Some(other) => Err(format!("invalid type: {}, expected a boolean", kind_of(&other))),
        };
        Ok(Self {
            hide_toolbar: flag("hideToolbar")?,
            hide_menubar: flag("hideMenubar")?,
            hide_window_ui: flag("hideWindowUi")?,
            fit_window: flag("fitWindow")?,
            center_window: flag("centerWindow")?,
            display_doc_title: flag("displayDocTitle")?,
        })
    }
}
