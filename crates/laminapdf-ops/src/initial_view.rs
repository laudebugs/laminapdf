//! `SetInitialView`: how the document asks to be shown when it opens (ISO 32000-2 §12.2,
//! §7.7.2 Table 29): the catalog's `/OpenAction`, `/PageMode`, `/PageLayout` and the
//! yes-or-no window entries of `/ViewerPreferences`.

use laminapdf_cos::{CosMut, Dict, Name, ObjRef, Object};
use laminapdf_model::{OpenAction, PageId, PageLayout, PageMode, WindowPreferences};
use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct SetInitialView {
    /// The page and zoom to open at.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "wire::double_option::deserialize"
    )]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "OpenAt | null"))]
    pub open: Option<Option<OpenAt>>,
    /// The panel to show (`/PageMode`).
    #[serde(default, skip_serializing_if = "Option::is_none", with = "page_mode")]
    #[cfg_attr(
        feature = "tsify",
        tsify(
            optional,
            type = "\"none\" | \"outlines\" | \"thumbnails\" | \"fullScreen\" | \"optionalContent\" | \"attachments\" | null"
        )
    )]
    pub page_mode: Option<Option<PageMode>>,
    /// How to lay the pages out (`/PageLayout`).
    #[serde(default, skip_serializing_if = "Option::is_none", with = "page_layout")]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct OpenAt {
    /// The page.
    #[serde(with = "wire::page_id")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub page: PageId,
    /// The zoom.
    #[serde(default, with = "zoom")]
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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
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
    fn get(&self, key: &str) -> Option<bool> {
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
            Some(open) => Some(self.open_value(doc, open.as_ref())?),
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
        match catalog_ref {
            Some(r) => cos.set(r, Object::Dict(catalog))?,
            None => {
                let mut trailer = cos.trailer().clone();
                trailer.insert("Root", Object::Dict(catalog));
                cos.set_trailer(trailer);
            }
        }
        Ok(())
    }

    /// The `/OpenAction` value for `open` (`None`: remove it), refusing to replace an action
    /// that isn't a destination.
    fn open_value(&self, doc: &laminapdf_model::Document, open: Option<&OpenAt>) -> Result<Option<Object>> {
        if let OpenAction::Action(_) = doc.initial_view()?.open {
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
        /// The patch field's wire form: absent keeps it, `null` removes it, a camelCase name sets it.
        mod $module {
            use serde::{Deserialize, Deserializer, Serializer, de};

            use super::$ty;

            const NAMES: [&str; <$ty>::ALL.len()] = [$($name),*];

            fn name(value: $ty) -> &'static str {
                <$ty>::ALL
                    .iter()
                    .zip(NAMES)
                    .find_map(|(v, n)| (*v == value).then_some(n))
                    .unwrap_or_default()
            }

            #[expect(
                clippy::ref_option,
                clippy::option_option,
                reason = "serde's `with` passes the field by reference; absent, null and a value are three cases"
            )]
            pub(super) fn serialize<S: Serializer>(value: &Option<Option<$ty>>, s: S) -> Result<S::Ok, S::Error> {
                match value {
                    Some(Some(v)) => s.serialize_str(name(*v)),
                    Some(None) | None => s.serialize_none(),
                }
            }

            #[expect(clippy::option_option, reason = "absent, null and a value are three cases")]
            pub(super) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Option<$ty>>, D::Error> {
                let Some(s) = Option::<String>::deserialize(d)? else {
                    return Ok(Some(None));
                };
                NAMES
                    .iter()
                    .position(|n| *n == s)
                    .and_then(|i| <$ty>::ALL.get(i).copied())
                    .map(|v| Some(Some(v)))
                    .ok_or_else(|| de::Error::unknown_variant(&s, &NAMES))
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
    use serde::de::{self, Deserializer, Visitor};
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

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<OpenZoom, D::Error> {
        d.deserialize_any(ZoomVisitor)
    }

    struct ZoomVisitor;

    impl Visitor<'_> for ZoomVisitor {
        type Value = OpenZoom;

        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("\"default\", \"fitPage\", \"fitWidth\" or a percentage from 1 to 6400")
        }

        fn visit_str<E: de::Error>(self, s: &str) -> Result<OpenZoom, E> {
            match s {
                "default" => Ok(OpenZoom::ReaderDefault),
                "fitPage" => Ok(OpenZoom::FitPage),
                "fitWidth" => Ok(OpenZoom::FitWidth),
                _ => Err(E::unknown_variant(s, &["default", "fitPage", "fitWidth"])),
            }
        }

        fn visit_u64<E: de::Error>(self, n: u64) -> Result<OpenZoom, E> {
            u16::try_from(n)
                .ok()
                .filter(|p| (1..=OpenZoom::MAX_PERCENT).contains(p))
                .map(OpenZoom::Percent)
                .ok_or_else(|| E::invalid_value(de::Unexpected::Unsigned(n), &self))
        }

        fn visit_i64<E: de::Error>(self, n: i64) -> Result<OpenZoom, E> {
            u64::try_from(n)
                .map_err(|_| E::invalid_value(de::Unexpected::Signed(n), &self))
                .and_then(|n| self.visit_u64(n))
        }
    }
}
