//! Drawing the widgets a value changes (E8 plan §2.2): each widget's request read from its
//! dictionary and the field's `/DA`, checked before anything is written, then drawn through the
//! document's [`Appearances`] and named by the widget's `/AP /N`. The fonts given by their bytes
//! that an appearance embeds join the form's `/DR`.

use laminapdf_cos::{Dict, Name, ObjRef, Object, tolerant};
use laminapdf_model::{FieldFlags, FieldKind, FieldValue, TerminalField, Widget};

use crate::annot::add::intent_space;
use crate::annot::drawing::read_border;
use crate::annot::freetext_keys::{family_of_name, parse_da};
use crate::annot::object_dict;
use crate::annot::{
    AnnotColor, Appearances, FieldAppearanceRequest, FieldContent, IntentSpace, StandardFamily, TextAlign,
};
use crate::blob::{BlobId, BlobStore};
use crate::error::{Error, Result};

/// What every widget of a field draws, read once: the text or rows, and the style.
struct Shown {
    text: String,
    rows: Vec<String>,
    selected: Vec<usize>,
    top: usize,
    style: Style,
    align: TextAlign,
}

/// The text style a `/DA` sets: its font's standard family (Helvetica when it names another),
/// its size (`None` for 0, auto-size) and its fill colour (black when it sets none).
#[derive(Clone, Copy)]
struct Style {
    family: StandardFamily,
    size: Option<f64>,
    color: AnnotColor,
}

/// The style `da` sets, its font looked up in the form's `/DR` when `/DA` names it by a resource
/// name of its own (`/F1`).
fn style(doc: &laminapdf_model::Document, da: Option<&str>) -> Result<Style> {
    let da = parse_da(da.unwrap_or_default());
    let family = match (da.family, da.name.as_deref()) {
        (Some(family), _) => family,
        (None, Some(name)) => dr_family(doc, name)?.unwrap_or(StandardFamily::Helvetica),
        (None, None) => StandardFamily::Helvetica,
    };
    Ok(Style {
        family,
        size: da.size,
        color: da.color.unwrap_or(AnnotColor::Gray(0.0)),
    })
}

/// One widget's dictionary entries the request needs.
struct WidgetLook {
    widget: ObjRef,
    /// `/MK /CA`'s first character: what a check box or radio button shows when on.
    caption: Option<char>,
    /// Whether the widget has a normal appearance with an on-state.
    has_on_state: bool,
    /// The style of the widget's own `/DA`, which a kid widget may set apart from its field's.
    style: Option<Style>,
    rect: laminapdf_model::UserSpaceRect,
    rotation: u16,
    background: Option<AnnotColor>,
    border: Option<AnnotColor>,
    border_width: f64,
}

/// The widgets of a field to draw for a value, and how.
pub(super) struct Redraw {
    shown: Shown,
    widgets: Vec<WidgetLook>,
    kind: FieldKind,
    flags: FieldFlags,
    max_len: Option<u32>,
    intent: Option<IntentSpace>,
    pdfa: bool,
    /// For a check box or radio button, the on-state its widgets gain.
    on_state: Option<String>,
}

impl Redraw {
    /// Reads what `field`'s widgets will show once it holds `value`, and checks that
    /// `appearances` can draw each, before anything is written; `None` when nothing is drawn: a
    /// check box or radio button whose widgets all have appearances keeps them and switches
    /// states.
    ///
    /// # Errors
    /// `appearance-unavailable` without a generator, and the generator's refusal.
    pub(super) fn plan(
        doc: &laminapdf_model::Document,
        field: &TerminalField,
        value: &FieldValue,
        fonts: &[BlobId],
        appearances: Option<&dyn Appearances>,
        blobs: &BlobStore,
    ) -> Result<Option<Self>> {
        // A check box gains appearances where it has none. A radio button doesn't: each needs an
        // on-state of its own, which a value alone can't name.
        let buttons = field.kind == FieldKind::CheckBox;
        if !buttons && !matches!(field.kind, FieldKind::Text | FieldKind::ComboBox | FieldKind::ListBox) {
            return Ok(None);
        }
        let shown = shown(doc, field, value)?;
        let mut widgets = Vec::with_capacity(field.widgets.len());
        for w in &field.widgets {
            if let Some(look) = look(doc, w)? {
                // A button keeps the appearances it has and gains them only when it has none
                // (E8 plan §2.2).
                if !buttons || !look.has_on_state {
                    widgets.push(look);
                }
            }
        }
        if buttons && widgets.is_empty() {
            return Ok(None);
        }
        let redraw = Self {
            shown,
            widgets,
            kind: field.kind,
            flags: field.flags,
            max_len: field.max_len,
            intent: intent_space(doc)?,
            pdfa: matches!(doc.xmp()?, laminapdf_model::xmp::PacketState::Parsed(p) if p.pdfa_part().is_some()),
            on_state: buttons.then(|| match value {
                FieldValue::Button(Some(state)) => state.clone(),
                _ => "Yes".to_owned(),
            }),
        };
        let generator = appearances.ok_or(Error::AppearanceUnavailable)?;
        for look in &redraw.widgets {
            match &redraw.on_state {
                // A button gains both states: each is checked before either is drawn.
                Some(_) => {
                    for on in [true, false] {
                        generator.check_field(&redraw.button_request(look, on))?;
                    }
                }
                None => generator.check_field(&redraw.request(look, fonts, Some(blobs)))?,
            }
        }
        Ok(Some(redraw))
    }

    /// Draws each widget and names its appearance in `/AP /N`; adds the fonts the appearances
    /// embed to the form's `/DR`.
    ///
    /// # Errors
    /// `appearance-unavailable` without a generator, the generator's errors and the overlay's.
    pub(super) fn write(
        &self,
        doc: &mut laminapdf_model::Document,
        fonts: &[BlobId],
        appearances: Option<&dyn Appearances>,
        blobs: &BlobStore,
    ) -> Result<()> {
        let generator = appearances.ok_or(Error::AppearanceUnavailable)?;
        let mut embedded: Vec<ObjRef> = Vec::new();
        for look in &self.widgets {
            let Some(mut dict) = object_dict(doc, look.widget)? else {
                continue;
            };
            let ap = if let Some(on) = &self.on_state {
                // A button gains both states: its on-state and `Off`.
                let mut states = Dict::new();
                for (name, is_on) in [(on.as_str(), true), ("Off", false)] {
                    let request = self.button_request(look, is_on);
                    let drawn = generator.field(&mut doc.cos_mut().edit(), &request)?;
                    states.insert(Name::from(name), Object::Ref(drawn.normal));
                }
                [("N", Object::Dict(states))].into_iter().collect::<Dict>()
            } else {
                let request = self.request(look, fonts, Some(blobs));
                let drawn = generator.field(&mut doc.cos_mut().edit(), &request)?;
                for (_, font) in drawn.fonts {
                    if !embedded.contains(&font) {
                        embedded.push(font);
                    }
                }
                [("N", Object::Ref(drawn.normal))].into_iter().collect::<Dict>()
            };
            dict.insert("AP", Object::Dict(ap));
            doc.cos_mut().edit().set(look.widget, Object::Dict(dict))?;
        }
        add_to_dr(doc, &embedded)
    }

    /// The request for one state of a button widget: its caption on, or nothing but its
    /// background and border off.
    fn button_request(&self, look: &WidgetLook, on: bool) -> FieldAppearanceRequest<'_> {
        let caption = look
            .caption
            .unwrap_or(if self.kind == FieldKind::RadioGroup { 'l' } else { '4' });
        FieldAppearanceRequest {
            content: FieldContent::Button { caption, on },
            ..self.request(look, &[], None)
        }
    }

    /// The request for one widget.
    fn request<'a>(
        &'a self,
        look: &WidgetLook,
        fonts: &'a [BlobId],
        blobs: Option<&'a BlobStore>,
    ) -> FieldAppearanceRequest<'a> {
        let s = &self.shown;
        let content = if self.kind == FieldKind::ListBox {
            FieldContent::List {
                rows: &s.rows,
                selected: &s.selected,
                top: s.top,
            }
        } else {
            let multiline = self.kind == FieldKind::Text && self.flags.has(FieldFlags::MULTILINE);
            let password = self.kind == FieldKind::Text && self.flags.has(FieldFlags::PASSWORD);
            // §12.7.5.3: a comb needs `/MaxLen` and a field that is neither multi-line nor a
            // password or file select.
            let comb = (self.kind == FieldKind::Text
                && self.flags.has(FieldFlags::COMB)
                && !multiline
                && !password
                && !self.flags.has(FieldFlags::FILE_SELECT))
            .then_some(self.max_len)
            .flatten()
            .filter(|n| *n > 0);
            FieldContent::Text {
                text: &s.text,
                multiline,
                comb,
                password,
            }
        };
        let style = look.style.unwrap_or(s.style);
        FieldAppearanceRequest {
            rect: look.rect,
            rotation: look.rotation,
            background: look.background,
            border: look.border,
            border_width: look.border_width,
            family: style.family,
            size: style.size,
            color: style.color,
            align: s.align,
            content,
            fallbacks: fonts,
            blobs,
            intent: self.intent,
            embed_fonts: self.pdfa,
        }
    }
}

/// What `field` shows once it holds `value`: its text (a combo box's chosen option as shown),
/// or a list box's rows, and the style its `/DA` and `/Q` set.
fn shown(doc: &laminapdf_model::Document, field: &TerminalField, value: &FieldValue) -> Result<Shown> {
    let chosen: Vec<String> = match value {
        FieldValue::Text(t) => vec![t.clone()],
        FieldValue::Choice(c) => c.clone(),
        _ => Vec::new(),
    };
    let display = |export: &str| {
        field
            .options
            .iter()
            .find(|o| o.export == export)
            .map_or_else(|| export.to_owned(), |o| o.display.clone())
    };
    let text = match field.kind {
        FieldKind::ComboBox => chosen.first().map(|c| display(c)).unwrap_or_default(),
        _ => chosen.first().cloned().unwrap_or_default(),
    };
    let selected = (0..field.options.len())
        .filter(|&i| field.options.get(i).is_some_and(|o| chosen.contains(&o.export)))
        .collect();
    let top = match object_dict(doc, field_object(field)?)?.and_then(|d| d.get("TI").cloned()) {
        Some(v) => tolerant(doc.deref(&v))?
            .and_then(|o| o.as_int())
            .and_then(|i| usize::try_from(i).ok())
            .unwrap_or(0),
        None => 0,
    };
    Ok(Shown {
        text,
        rows: field.options.iter().map(|o| o.display.clone()).collect(),
        selected,
        top,
        style: style(doc, field.default_appearance.as_deref())?,
        align: match field.quadding {
            1 => TextAlign::Center,
            2 => TextAlign::Right,
            _ => TextAlign::Left,
        },
    })
}

/// The object holding `field`.
fn field_object(field: &TerminalField) -> Result<ObjRef> {
    field
        .id
        .as_lamina_id()
        .obj_ref()
        .ok_or_else(|| Error::UnknownField(field.id.clone()))
}

/// The standard family of the font `/DR /Font` names `name`, by its `/BaseFont`.
fn dr_family(doc: &laminapdf_model::Document, name: &str) -> Result<Option<StandardFamily>> {
    let Some(fonts) = dr_fonts(doc)? else { return Ok(None) };
    let Some(font) = fonts.get(name) else { return Ok(None) };
    let Some(font) = tolerant(doc.deref(font))?.and_then(|o| o.as_dict().cloned()) else {
        return Ok(None);
    };
    Ok(font
        .get_name("BaseFont")
        .and_then(|b| family_of_name(&String::from_utf8_lossy(b))))
}

/// The form's `/DR /Font` dictionary.
fn dr_fonts(doc: &laminapdf_model::Document) -> Result<Option<Dict>> {
    let Some(acro) = acro_form(doc)? else { return Ok(None) };
    let Some(dr) = acro.get("DR") else { return Ok(None) };
    let Some(dr) = tolerant(doc.deref(dr))?.and_then(|o| o.as_dict().cloned()) else {
        return Ok(None);
    };
    let Some(fonts) = dr.get("Font") else { return Ok(None) };
    Ok(tolerant(doc.deref(fonts))?.and_then(|o| o.as_dict().cloned()))
}

/// The interactive form dictionary.
fn acro_form(doc: &laminapdf_model::Document) -> Result<Option<Dict>> {
    let Some(acro) = doc.catalog()?.get("AcroForm").cloned() else {
        return Ok(None);
    };
    Ok(tolerant(doc.deref(&acro))?.and_then(|o| o.as_dict().cloned()))
}

/// The entries of `w`'s dictionary the appearance needs; `None` for a widget without a `/Rect`
/// or a dictionary.
fn look(doc: &laminapdf_model::Document, w: &Widget) -> Result<Option<WidgetLook>> {
    let (Some(widget), Some(rect)) = (w.id.as_lamina_id().obj_ref(), w.rect) else {
        return Ok(None);
    };
    let Some(dict) = object_dict(doc, widget)? else {
        return Ok(None);
    };
    let mk = match dict.get("MK") {
        Some(v) => tolerant(doc.deref(v))?.and_then(|o| o.as_dict().cloned()),
        None => None,
    }
    .unwrap_or_default();
    let color = |key: &str| -> Result<Option<AnnotColor>> {
        let Some(v) = mk.get(key) else { return Ok(None) };
        let Some(v) = tolerant(doc.deref(v))? else {
            return Ok(None);
        };
        let components: Vec<f64> = v
            .as_array()
            .unwrap_or_default()
            .iter()
            .filter_map(Object::as_f64)
            .collect();
        // An empty array is transparent (Table 192).
        Ok(AnnotColor::from_components(&components).filter(|_| !components.is_empty()))
    };
    let rotation = match mk.get("R") {
        Some(r) => tolerant(doc.deref(r))?.and_then(|o| o.as_int()),
        None => None,
    }
    .map_or(0, |r| u16::try_from(r.rem_euclid(360) / 90 * 90).unwrap_or(0));
    let caption = match mk.get("CA") {
        Some(v) => tolerant(doc.deref(v))?.and_then(|o| match &*o {
            Object::String(s) => laminapdf_syntax::text::decode_text_string(&s.bytes).chars().next(),
            _ => None,
        }),
        None => None,
    };
    let own_da = match dict.get("DA") {
        Some(v) => tolerant(doc.deref(v))?.and_then(|o| match &*o {
            Object::String(s) => Some(laminapdf_syntax::text::decode_text_string(&s.bytes)),
            _ => None,
        }),
        None => None,
    };
    let style = own_da.as_deref().map(|da| style(doc, Some(da))).transpose()?;
    Ok(Some(WidgetLook {
        widget,
        caption,
        style,
        has_on_state: w.on_state.is_some(),
        rect,
        rotation,
        background: color("BG")?,
        border: color("BC")?,
        border_width: read_border(&dict, false).width,
    }))
}

/// The names Lamina PDF gives the fonts it adds to `/DR`: `LmnaFont0`, `LmnaFont1` …
const DR_PREFIX: &str = "LmnaFont";

/// Adds `fonts` to the form's `/DR /Font`, each under a name no font there has
/// (`LmnaFont<n>`) unless `/DR` already lists it, in place of the fonts earlier fills added; writes the dictionary where it is (the form's
/// own, or an indirect `/DR` or `/Font`).
fn add_to_dr(doc: &mut laminapdf_model::Document, fonts: &[ObjRef]) -> Result<()> {
    if fonts.is_empty() {
        return Ok(());
    }
    let Some(acro_ref) = doc.catalog()?.get("AcroForm").cloned() else {
        return Ok(());
    };
    let Some(mut acro) = acro_form(doc)? else {
        return Ok(());
    };
    let dr_value = acro.get("DR").cloned();
    let mut dr = match &dr_value {
        Some(v) => tolerant(doc.deref(v))?.and_then(|o| o.as_dict().cloned()),
        None => None,
    }
    .unwrap_or_default();
    let font_value = dr.get("Font").cloned();
    let mut font_dict = match &font_value {
        Some(v) => tolerant(doc.deref(v))?.and_then(|o| o.as_dict().cloned()),
        None => None,
    }
    .unwrap_or_default();
    // Earlier fills' fonts: no `/DA` names them, and each appearance keeps its own reference,
    // so the newest fill's fonts replace them rather than piling up.
    let earlier: Vec<Name> = font_dict
        .iter()
        .filter(|(name, _)| name.as_bytes().starts_with(DR_PREFIX.as_bytes()))
        .map(|(name, _)| name.clone())
        .collect();
    for name in &earlier {
        font_dict.remove(&String::from_utf8_lossy(name.as_bytes()));
    }
    let mut n = 0_u32;
    for r in fonts {
        if font_dict.iter().any(|(_, v)| v.as_ref() == Some(*r)) {
            continue;
        }
        let name = loop {
            let candidate = format!("{DR_PREFIX}{n}");
            n += 1;
            if font_dict.get(&candidate).is_none() {
                break candidate;
            }
        };
        font_dict.insert(Name::from(name.as_str()), Object::Ref(*r));
    }
    let mut edit = doc.cos_mut().edit();
    if let Some(Object::Ref(r)) = font_value {
        edit.set(r, Object::Dict(font_dict))?;
        return Ok(());
    }
    dr.insert("Font", Object::Dict(font_dict));
    if let Some(Object::Ref(r)) = dr_value {
        edit.set(r, Object::Dict(dr))?;
        return Ok(());
    }
    acro.insert("DR", Object::Dict(dr));
    match acro_ref {
        Object::Ref(r) => edit.set(r, Object::Dict(acro))?,
        // A direct `/AcroForm` lives in the catalog.
        _ => laminapdf_model::set_catalog_entry(&mut edit, "AcroForm", Some(Object::Dict(acro)))?,
    }
    Ok(())
}
