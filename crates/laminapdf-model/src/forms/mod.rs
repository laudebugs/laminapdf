//! The interactive form (ISO 32000-2 §12.7): a typed view of every terminal field of
//! `/AcroForm /Fields`, with the values each inherits up `/Parent` (§12.7.4.1), its widgets and
//! the pages they are on, and what the field tree and the pages' `/Annots` disagree on
//! ([`Document::form`](crate::Document::form)); and the order Tab visits a page's widgets in ([`Document::tab_order`](crate::Document::tab_order),
//! §12.5.1, Table 31's `/Tabs`).

mod behaviour;
mod read;
mod tabs;

use crate::annot::AnnotFlags;
use crate::geometry::UserSpaceRect;
use crate::id::{AnnotId, FieldId, PageId};

pub use behaviour::{
    Behaviour, CalculateFunction, DatePattern, FieldBehaviour, FieldTrigger, FormScript, FormValues, InvalidValue,
    NegativeStyle, NumberFormat, PercentFormat, RangeValidate, ScriptLocation, ScriptState, SeparatorStyle, ShownValue,
    SimpleCalculate, SpecialFormat,
};
pub use tabs::{TabOrder, Tabs};

/// The interactive form: its terminal fields and the reconciliation of the field tree with the
/// pages' widgets (04 §3.1). An empty form when the document has no `/AcroForm`.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct Form {
    /// The terminal fields, in the order a depth-first walk of `/Fields` meets them. Objects
    /// with one fully qualified name are one field (§12.7.4.2): the first met holds it, the
    /// others are in [`TerminalField::shared`]. A widget that `/Fields` lists in place of its
    /// field (§12.7.4 asks for the root fields) stands for the field its `/Parent` names.
    pub fields: Vec<TerminalField>,
    /// Widgets that a page lists in `/Annots` and no field of the tree reaches, each with the
    /// field its `/Parent` chain names, when it names one.
    pub orphan_widgets: Vec<OrphanWidget>,
    /// Terminal fields none of whose widgets is on a page.
    pub widgetless: Vec<FieldId>,
    /// Fields or widgets the tree reaches through a node's `/Kids` whose `/Parent` names
    /// another object, or none: the view follows the tree and reports them.
    pub parent_mismatches: Vec<FieldId>,
    /// Terminal fields without a field type (`/FT`), left out of [`Form::fields`].
    pub untyped: Vec<FieldId>,
    /// `/NeedAppearances`: whether the form asks viewers to draw every widget again (deprecated
    /// in PDF 2.0, §12.7.3).
    pub need_appearances: bool,
    /// Whether the form has an XFA part (`/XFA`, §12.7.9).
    pub has_xfa: bool,
}

impl Form {
    /// The field `id` names: the object that holds it, or one that shares its name.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn field(&self, id: &FieldId) -> Option<&TerminalField> {
        self.fields.iter().find(|f| &f.id == id || f.shared.contains(id))
    }

    /// The field whose fully qualified name is `name`.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn field_named(&self, name: &str) -> Option<&TerminalField> {
        self.fields.iter().find(|f| f.name == name)
    }
}

/// A terminal field: one that has no field below it, only widgets (§12.7.4).
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct TerminalField {
    /// The field object's ID.
    pub id: FieldId,
    /// Other field objects with the same fully qualified name, which share its value.
    pub shared: Vec<FieldId>,
    /// Its fully qualified name (§12.7.4.2).
    pub name: String,
    /// `/TU`, the name the interface shows.
    pub tooltip: Option<String>,
    /// The kind of field, from `/FT` and `/Ff`.
    pub kind: FieldKind,
    /// `/Ff`, inherited.
    pub flags: FieldFlags,
    /// `/V`, inherited; `None` when no field up the chain has one. A signature's value is a
    /// dictionary, read as [`TerminalField::signed`] instead.
    pub value: Option<FieldValue>,
    /// `/DV`, inherited: what resetting the form gives it.
    pub default_value: Option<FieldValue>,
    /// `/DA`, inherited, else the form's.
    pub default_appearance: Option<String>,
    /// `/Q`, inherited, else the form's: 0 left, 1 centred, 2 right.
    pub quadding: u8,
    /// `/MaxLen`, inherited (text fields).
    pub max_len: Option<u32>,
    /// `/Opt`: a choice field's options, or a check box's or radio group's export values, one
    /// per widget.
    pub options: Vec<ChoiceOption>,
    /// Whether a signature field holds a signature (its `/V` is a dictionary).
    pub signed: bool,
    /// Its widgets: the field itself when field and widget are merged, else its kids.
    pub widgets: Vec<Widget>,
}

impl TerminalField {
    /// Whether the interface may change it: `ReadOnly` (§12.7.4.1). The library's ops may.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn read_only(&self) -> bool {
        self.flags.has(FieldFlags::READ_ONLY)
    }

    /// Whether it must have a value when the form is submitted: `Required`.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn required(&self) -> bool {
        self.flags.has(FieldFlags::REQUIRED)
    }

    /// Whether the field holds a value: every kind but push buttons and signature fields, whose
    /// value a fill, an export and a reset leave alone.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn takes_value(&self) -> bool {
        !matches!(self.kind, FieldKind::PushButton | FieldKind::Signature)
    }

    /// The on-state of the widget whose `/Opt` entry exports `export`, for a button with `/Opt`
    /// (§12.7.5.2.4: the entry at a widget's index is its export value).
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn state_exporting(&self, export: &str) -> Option<&str> {
        let index = self.options.iter().position(|o| o.export == export)?;
        self.widgets.get(index)?.on_state.as_deref()
    }

    /// The export value of on-state `state`, for a button with `/Opt`: the `/Opt` entry at the
    /// index of the first widget with that on-state; the inverse of
    /// [`state_exporting`](Self::state_exporting).
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn export_of_state(&self, state: &str) -> Option<&str> {
        let index = self.widgets.iter().position(|w| w.on_state.as_deref() == Some(state))?;
        Some(self.options.get(index)?.export.as_str())
    }

    /// The on-state names its widgets' normal appearances have, each once, in widget order
    /// (check boxes and radio buttons).
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn on_states(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        for state in self.widgets.iter().filter_map(|w| w.on_state.as_deref()) {
            if !out.contains(&state) {
                out.push(state);
            }
        }
        out
    }
}

/// What kind of field (§12.7.5, from `/FT` and the `/Ff` bits that pick a button's or a
/// choice's kind).
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FieldKind {
    /// `/Tx`: text, single or multi-line.
    Text,
    /// `/Btn` with neither `Radio` nor `Pushbutton`.
    CheckBox,
    /// `/Btn` with `Radio`.
    RadioGroup,
    /// `/Btn` with `Pushbutton`: it has no value.
    PushButton,
    /// `/Ch` with `Combo`: a drop-down list, editable with `Edit`.
    ComboBox,
    /// `/Ch` without `Combo`.
    ListBox,
    /// `/Sig`.
    Signature,
}

impl FieldKind {
    /// The kind `/FT` `ft` and flags `flags` give; `None` for a type this reader doesn't know.
    #[must_use]
    pub(crate) fn of(ft: &[u8], flags: FieldFlags) -> Option<Self> {
        Some(match ft {
            b"Tx" => Self::Text,
            b"Btn" if flags.has(FieldFlags::PUSHBUTTON) => Self::PushButton,
            b"Btn" if flags.has(FieldFlags::RADIO) => Self::RadioGroup,
            b"Btn" => Self::CheckBox,
            b"Ch" if flags.has(FieldFlags::COMBO) => Self::ComboBox,
            b"Ch" => Self::ListBox,
            b"Sig" => Self::Signature,
            _ => return None,
        })
    }

    /// The kind's name, as the bindings spell it: `text`, `checkBox`, `radioGroup`,
    /// `pushButton`, `comboBox`, `listBox` or `signature`.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::CheckBox => "checkBox",
            Self::RadioGroup => "radioGroup",
            Self::PushButton => "pushButton",
            Self::ComboBox => "comboBox",
            Self::ListBox => "listBox",
            Self::Signature => "signature",
        }
    }
}

/// A field's `/Ff` bits (§12.7.4.1, Tables 227, 229, 231, 233): bit 1 is `1 << 0`.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FieldFlags(pub u32);

impl FieldFlags {
    /// The interface may not change the value.
    ///
    /// Added in 0.6.0.
    pub const READ_ONLY: u32 = 1;
    /// The field must have a value when the form is submitted.
    ///
    /// Added in 0.6.0.
    pub const REQUIRED: u32 = 1 << 1;
    /// The field is left out when the form is submitted.
    ///
    /// Added in 0.6.0.
    pub const NO_EXPORT: u32 = 1 << 2;
    /// Text: several lines.
    ///
    /// Added in 0.6.0.
    pub const MULTILINE: u32 = 1 << 12;
    /// Text: shown as bullets, never stored.
    ///
    /// Added in 0.6.0.
    pub const PASSWORD: u32 = 1 << 13;
    /// Radio buttons: one is always on.
    ///
    /// Added in 0.6.0.
    pub const NO_TOGGLE_TO_OFF: u32 = 1 << 14;
    /// Buttons: radio buttons.
    ///
    /// Added in 0.6.0.
    pub const RADIO: u32 = 1 << 15;
    /// Buttons: a push button.
    ///
    /// Added in 0.6.0.
    pub const PUSHBUTTON: u32 = 1 << 16;
    /// Choices: a combo box.
    ///
    /// Added in 0.6.0.
    pub const COMBO: u32 = 1 << 17;
    /// Combo boxes: text may be typed besides the options.
    ///
    /// Added in 0.6.0.
    pub const EDIT: u32 = 1 << 18;
    /// Choices: the options are sorted for display.
    ///
    /// Added in 0.6.0.
    pub const SORT: u32 = 1 << 19;
    /// Text: the value names a file to submit.
    ///
    /// Added in 0.6.0.
    pub const FILE_SELECT: u32 = 1 << 20;
    /// List boxes: several options may be chosen.
    ///
    /// Added in 0.6.0.
    pub const MULTI_SELECT: u32 = 1 << 21;
    /// Text and choices: no spelling check.
    ///
    /// Added in 0.6.0.
    pub const DO_NOT_SPELL_CHECK: u32 = 1 << 22;
    /// Text: no scrolling past the box.
    ///
    /// Added in 0.6.0.
    pub const DO_NOT_SCROLL: u32 = 1 << 23;
    /// Text: `/MaxLen` cells of equal width.
    ///
    /// Added in 0.6.0.
    pub const COMB: u32 = 1 << 24;
    /// Radio buttons: widgets with one on-state turn on and off together (and, for text, the
    /// same bit is `RichText`).
    ///
    /// Added in 0.6.0.
    pub const RADIOS_IN_UNISON: u32 = 1 << 25;
    /// Choices: the value is committed as soon as an option is chosen.
    ///
    /// Added in 0.6.0.
    pub const COMMIT_ON_SEL_CHANGE: u32 = 1 << 26;

    /// Whether `flag` (one of the constants) is set.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn has(self, flag: u32) -> bool {
        self.0 & flag != 0
    }
}

/// A field's value (`/V`, `/DV`), or the value an op gives one.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FieldValue {
    /// A text field's text (a text string, or a text stream's decoded bytes as text).
    Text(String),
    /// A check box's or radio group's on-state name, `None` for `Off`.
    Button(Option<String>),
    /// A choice field's export values (several for a multi-select list box).
    Choice(Vec<String>),
    /// No value: an op removes `/V`. Reading never gives it; an absent value is `None`.
    Clear,
}

/// One entry of `/Opt` (§12.7.5.4): what is exported and what is shown, the same when the
/// entry is a single string.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct ChoiceOption {
    /// The export value.
    pub export: String,
    /// The text shown.
    pub display: String,
}

impl ChoiceOption {
    /// An option exported as `export` and shown as `display`.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn new(export: String, display: String) -> Self {
        Self { export, display }
    }
}

/// A field's widget annotation.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Widget {
    /// The widget's ID (an annotation ID; the field's own object when they are merged).
    pub id: AnnotId,
    /// The page whose `/Annots` lists it; `None` when no page does.
    pub page: Option<PageId>,
    /// `/Rect` in user space.
    pub rect: Option<UserSpaceRect>,
    /// `/F`.
    pub flags: AnnotFlags,
    /// For a check box or radio button, the name of its normal appearance's state that isn't
    /// `Off` (§12.7.5.2.3), when it has one.
    pub on_state: Option<String>,
    /// `/AS`, the state it shows.
    pub state: Option<String>,
}

/// A widget that a page lists and no field of the tree reaches.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct OrphanWidget {
    /// The widget.
    pub widget: AnnotId,
    /// The page that lists it.
    pub page: PageId,
    /// The field its `/Parent` chain names; `None` when it has no parent.
    pub field: Option<FieldId>,
}

#[cfg(test)]
mod tests {
    use laminapdf_cos::ObjRef;

    use super::*;
    use crate::AnnotId;

    /// A radio group with `/Opt [(Small) (Large)]` whose widgets' on-states are named `1` and
    /// `0`: the second widget first, so a name is not its widget's index.
    fn sizes() -> TerminalField {
        let widget = |num, state: &str| Widget {
            id: AnnotId::object(ObjRef::new(num, 0)),
            page: None,
            rect: None,
            flags: AnnotFlags::default(),
            on_state: Some(state.to_owned()),
            state: None,
        };
        TerminalField {
            id: FieldId::object(ObjRef::new(10, 0)),
            shared: Vec::new(),
            name: "size".to_owned(),
            tooltip: None,
            kind: FieldKind::RadioGroup,
            flags: FieldFlags(FieldFlags::RADIO),
            value: None,
            default_value: None,
            default_appearance: None,
            quadding: 0,
            max_len: None,
            options: ["Small", "Large"]
                .map(|e| ChoiceOption::new(e.to_owned(), e.to_owned()))
                .to_vec(),
            signed: false,
            widgets: vec![widget(11, "1"), widget(12, "0")],
        }
    }

    #[test]
    fn export_of_state_should_take_the_export_at_the_widgets_index_not_the_states_name() {
        let field = sizes();
        assert_eq!(
            [field.export_of_state("1"), field.export_of_state("0")],
            [Some("Small"), Some("Large")]
        );
    }

    #[test]
    fn state_exporting_should_be_the_inverse_of_export_of_state() {
        let field = sizes();
        assert_eq!(
            ["Small", "Large"].map(|e| field.state_exporting(e)),
            [Some("1"), Some("0")]
        );
    }
}
