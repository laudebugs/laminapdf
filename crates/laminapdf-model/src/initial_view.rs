//! How a document asks to be shown when it opens (ISO 32000-2 §12.2, §7.7.2 Table 29): the
//! catalog's `/OpenAction`, `/PageMode`, `/PageLayout` and `/ViewerPreferences`.

use laminapdf_cos::{Dict, Object, Result, Warning};

use crate::document::Document;
use crate::outline::{ActionKind, Destination};

/// Which panel a viewer shows when the document opens (`/PageMode`, Table 29).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PageMode {
    /// No panel (`UseNone`, the default).
    None,
    /// The outline (`UseOutlines`).
    Outlines,
    /// Page thumbnails (`UseThumbs`).
    Thumbnails,
    /// Full screen (`FullScreen`).
    FullScreen,
    /// The optional content panel (`UseOC`, PDF 1.5).
    OptionalContent,
    /// The attachments panel (`UseAttachments`, PDF 1.6).
    Attachments,
}

impl PageMode {
    /// Every page mode, in Table 29's order.
    ///
    /// Added in 0.2.0.
    pub const ALL: [Self; 6] = [
        Self::None,
        Self::Outlines,
        Self::Thumbnails,
        Self::FullScreen,
        Self::OptionalContent,
        Self::Attachments,
    ];

    /// The name the catalog stores (`UseOutlines`).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn pdf_name(self) -> &'static str {
        match self {
            Self::None => "UseNone",
            Self::Outlines => "UseOutlines",
            Self::Thumbnails => "UseThumbs",
            Self::FullScreen => "FullScreen",
            Self::OptionalContent => "UseOC",
            Self::Attachments => "UseAttachments",
        }
    }

    /// The page mode a catalog name stands for, if it is one of Table 29's.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn from_pdf_name(name: &[u8]) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.pdf_name().as_bytes() == name)
    }

    /// The PDF version that introduced this mode, as `(major, minor)`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn since(self) -> (u8, u8) {
        match self {
            Self::OptionalContent => (1, 5),
            Self::Attachments => (1, 6),
            _ => (1, 0),
        }
    }
}

/// How a viewer lays pages out when the document opens (`/PageLayout`, Table 29).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PageLayout {
    /// One page at a time (the default).
    SinglePage,
    /// One continuous column.
    OneColumn,
    /// Two columns, odd pages on the left.
    TwoColumnLeft,
    /// Two columns, odd pages on the right.
    TwoColumnRight,
    /// Two pages at a time, odd pages on the left (PDF 1.5).
    TwoPageLeft,
    /// Two pages at a time, odd pages on the right (PDF 1.5).
    TwoPageRight,
}

impl PageLayout {
    /// Every page layout, in Table 29's order.
    ///
    /// Added in 0.2.0.
    pub const ALL: [Self; 6] = [
        Self::SinglePage,
        Self::OneColumn,
        Self::TwoColumnLeft,
        Self::TwoColumnRight,
        Self::TwoPageLeft,
        Self::TwoPageRight,
    ];

    /// The name the catalog stores (`OneColumn`).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn pdf_name(self) -> &'static str {
        match self {
            Self::SinglePage => "SinglePage",
            Self::OneColumn => "OneColumn",
            Self::TwoColumnLeft => "TwoColumnLeft",
            Self::TwoColumnRight => "TwoColumnRight",
            Self::TwoPageLeft => "TwoPageLeft",
            Self::TwoPageRight => "TwoPageRight",
        }
    }

    /// The page layout a catalog name stands for, if it is one of Table 29's.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn from_pdf_name(name: &[u8]) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|layout| layout.pdf_name().as_bytes() == name)
    }

    /// The PDF version that introduced this layout, as `(major, minor)`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn since(self) -> (u8, u8) {
        match self {
            Self::TwoPageLeft | Self::TwoPageRight => (1, 5),
            _ => (1, 0),
        }
    }
}

/// The yes-or-no entries of the viewer preferences dictionary that say how a viewer's window
/// shows the document (§12.2, Table 147). Each is `false` when the file leaves it out, as the
/// specification's defaults are.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the six booleans of Table 147, one field each"
)]
pub struct WindowPreferences {
    /// `/HideToolbar`: hide the viewer's toolbars.
    pub hide_toolbar: bool,
    /// `/HideMenubar`: hide the viewer's menu bar.
    pub hide_menubar: bool,
    /// `/HideWindowUI`: hide the scroll bars and navigation controls, leaving the page.
    pub hide_window_ui: bool,
    /// `/FitWindow`: resize the window to fit the first page.
    pub fit_window: bool,
    /// `/CenterWindow`: put the window in the middle of the screen.
    pub center_window: bool,
    /// `/DisplayDocTitle`: show the document's title, not its file name, in the title bar
    /// (PDF 1.4).
    pub display_doc_title: bool,
}

impl WindowPreferences {
    /// Each entry's key in the viewer preferences dictionary, with a getter and a setter.
    ///
    /// Added in 0.2.0.
    pub const KEYS: [&'static str; 6] = [
        "HideToolbar",
        "HideMenubar",
        "HideWindowUI",
        "FitWindow",
        "CenterWindow",
        "DisplayDocTitle",
    ];

    /// The value of the entry stored under `key`, one of [`WindowPreferences::KEYS`].
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<bool> {
        Some(match key {
            "HideToolbar" => self.hide_toolbar,
            "HideMenubar" => self.hide_menubar,
            "HideWindowUI" => self.hide_window_ui,
            "FitWindow" => self.fit_window,
            "CenterWindow" => self.center_window,
            "DisplayDocTitle" => self.display_doc_title,
            _ => return None,
        })
    }

    fn set(&mut self, key: &str, value: bool) {
        let slot = match key {
            "HideToolbar" => &mut self.hide_toolbar,
            "HideMenubar" => &mut self.hide_menubar,
            "HideWindowUI" => &mut self.hide_window_ui,
            "FitWindow" => &mut self.fit_window,
            "CenterWindow" => &mut self.center_window,
            "DisplayDocTitle" => &mut self.display_doc_title,
            _ => return,
        };
        *slot = value;
    }
}

/// What the catalog's `/OpenAction` asks for (§12.3.2, §12.6.4).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum OpenAction {
    /// No `/OpenAction`, or one that can't be read: the document opens on its first page at the
    /// viewer's default zoom.
    None,
    /// A destination, given directly or through a `GoTo` action's `/D`.
    Destination(Destination),
    /// Another action (JavaScript, a named action, a link to another file). Lamina PDF never
    /// runs it and only reports its type.
    Action(ActionKind),
}

/// How a document asks to be shown when it opens: [`Document::initial_view`].
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct InitialView {
    /// The page and zoom to open at, or the action to run.
    pub open: OpenAction,
    /// The panel to show; `None` when the catalog has no `/PageMode` or an unknown one.
    pub page_mode: Option<PageMode>,
    /// How to lay the pages out; `None` when the catalog has no `/PageLayout` or an unknown one.
    pub page_layout: Option<PageLayout>,
    /// The window's viewer preferences.
    pub preferences: WindowPreferences,
}

impl Document {
    /// How the document asks to be shown when it opens: the catalog's `/OpenAction`,
    /// `/PageMode`, `/PageLayout` and the yes-or-no entries of `/ViewerPreferences`
    /// (ISO 32000-2 §12.2, §7.7.2). A value of the wrong type counts as absent, with
    /// [`Warning::InvalidEntry`]; an unknown page mode or layout name counts as absent too.
    ///
    /// # Errors
    /// As [`Document::outline`]: strict mode, when a value raises a warning; reading errors
    /// of the catalog.
    ///
    /// Added in 0.2.0.
    pub fn initial_view(&self) -> Result<InitialView> {
        let catalog = self.catalog()?;
        Ok(InitialView {
            open: self.open_action(&catalog)?,
            page_mode: self
                .catalog_name(&catalog, "PageMode")?
                .and_then(|n| PageMode::from_pdf_name(&n)),
            page_layout: self
                .catalog_name(&catalog, "PageLayout")?
                .and_then(|n| PageLayout::from_pdf_name(&n)),
            preferences: self.window_preferences(&catalog)?,
        })
    }

    fn open_action(&self, catalog: &Dict) -> Result<OpenAction> {
        let Some(resolved) = self.open_action_value(catalog)? else {
            return Ok(OpenAction::None);
        };
        let outline = crate::outline::Outline::destinations_only(self);
        let destination = match &*resolved {
            Object::Array(_) => &*resolved,
            Object::Dict(action) => match self.action_kind(action)? {
                Some(ActionKind::GoTo) => match action.get("D") {
                    Some(d) => d,
                    None => return Ok(OpenAction::None),
                },
                Some(kind) => return Ok(OpenAction::Action(kind)),
                None => return Ok(OpenAction::None),
            },
            _ => {
                self.warn_catalog("OpenAction")?;
                return Ok(OpenAction::None);
            }
        };
        Ok(outline
            .destination(destination)?
            .map_or(OpenAction::None, OpenAction::Destination))
    }

    /// The kind of the action the document runs when it opens, when `/OpenAction` is an action
    /// other than going to a page; `None` for a destination, a `GoTo` action or no open action.
    /// It reads only the catalog's entry, never a destination or a name tree, so an edit can
    /// check it without the reading code [`Document::initial_view`] needs.
    ///
    /// # Errors
    /// Reading errors (the catalog).
    ///
    /// Added in 0.2.0.
    pub fn open_action_kind(&self) -> Result<Option<ActionKind>> {
        let catalog = self.catalog()?;
        let Some(resolved) = self.open_action_value(&catalog)? else {
            return Ok(None);
        };
        Ok(match &*resolved {
            Object::Dict(action) => self.action_kind(action)?.filter(|k| *k != ActionKind::GoTo),
            _ => None,
        })
    }

    /// The catalog's `/OpenAction`, resolved.
    fn open_action_value(&self, catalog: &Dict) -> Result<Option<std::sync::Arc<Object>>> {
        match catalog.get("OpenAction") {
            Some(value) => self.read(value),
            None => Ok(None),
        }
    }

    /// An action dictionary's `/S`, when it is a name.
    fn action_kind(&self, action: &Dict) -> Result<Option<ActionKind>> {
        Ok(match action.get("S") {
            Some(s) => match self.read(s)?.as_deref() {
                Some(Object::Name(n)) => Some(ActionKind::from_name(n.as_bytes())),
                _ => None,
            },
            None => None,
        })
    }

    /// A catalog entry that must be a name, as its bytes.
    fn catalog_name(&self, catalog: &Dict, key: &'static str) -> Result<Option<Vec<u8>>> {
        let Some(value) = catalog.get(key) else {
            return Ok(None);
        };
        match self.read(value)?.as_deref() {
            Some(Object::Name(n)) => Ok(Some(n.as_bytes().to_vec())),
            None | Some(Object::Null) => Ok(None),
            Some(_) => {
                self.warn_catalog(key)?;
                Ok(None)
            }
        }
    }

    fn window_preferences(&self, catalog: &Dict) -> Result<WindowPreferences> {
        let mut prefs = WindowPreferences::default();
        let Some(value) = catalog.get("ViewerPreferences") else {
            return Ok(prefs);
        };
        let resolved = self.read(value)?;
        let Some(dict) = resolved.as_deref().and_then(Object::as_dict) else {
            if !matches!(resolved.as_deref(), None | Some(Object::Null)) {
                self.warn_catalog("ViewerPreferences")?;
            }
            return Ok(prefs);
        };
        for key in WindowPreferences::KEYS {
            if let Some(v) = dict.get(key)
                && let Some(Object::Bool(b)) = self.read(v)?.as_deref()
            {
                prefs.set(key, *b);
            }
        }
        Ok(prefs)
    }

    fn warn_catalog(&self, key: &'static str) -> Result<()> {
        self.warn(Warning::InvalidEntry {
            object: self.trailer().get("Root").and_then(Object::as_ref).map(|r| r.num),
            key,
        })
    }
}
