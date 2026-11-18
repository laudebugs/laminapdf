//! Drawing the widgets a value changes (E8 plan §2.2): each widget's request read from its
//! dictionary and the field's `/DA`, checked before anything is written, then drawn through the
//! document's [`Appearances`] and named by the widget's `/AP /N`. The fonts given by their bytes
//! that an appearance embeds join the form's `/DR`.

use laminapdf_cos::{Dict, Name, ObjRef, Object, tolerant};
use laminapdf_model::{FieldFlags, FieldKind, FieldValue, TerminalField, Widget};

use crate::annot::add::intent_space;
use crate::annot::freetext_keys::{family_of_base_font, parse_da};
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
    family: StandardFamily,
    size: Option<f64>,
    color: AnnotColor,
    align: TextAlign,
}

/// One widget's dictionary entries the request needs.
struct WidgetLook {
    widget: ObjRef,
    rect: laminapdf_model::UserSpaceRect,
    rotation: u16,
    background: Option<AnnotColor>,
    border: Option<AnnotColor>,
    border_width: f64,
}

/// The widgets of `field` to draw for `value`, and how; `None` for a field whose widgets keep
/// the appearances they have (check boxes and radio buttons, which switch states).
pub(super) struct Redraw {
    shown: Shown,
    widgets: Vec<WidgetLook>,
    kind: FieldKind,
    flags: FieldFlags,
    max_len: Option<u32>,
    intent: Option<IntentSpace>,
    pdfa: bool,
}

impl Redraw {
    /// Reads what `field`'s widgets will show once it holds `value`, and checks that
    /// `appearances` can draw each, before anything is written.
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
        if !matches!(field.kind, FieldKind::Text | FieldKind::ComboBox | FieldKind::ListBox) {
            return Ok(None);
        }
        let shown = shown(doc, field, value)?;
        let mut widgets = Vec::with_capacity(field.widgets.len());
        for w in &field.widgets {
            if let Some(look) = look(doc, w)? {
                widgets.push(look);
            }
        }
        let redraw = Self {
            shown,
            widgets,
            kind: field.kind,
            flags: field.flags,
            max_len: field.max_len,
            intent: intent_space(doc)?,
            pdfa: matches!(doc.xmp()?, laminapdf_model::xmp::PacketState::Parsed(p) if p.pdfa_part().is_some()),
        };
        let generator = appearances.ok_or(Error::AppearanceUnavailable)?;
        for look in &redraw.widgets {
            generator.check_field(&redraw.request(look, fonts, Some(blobs)))?;
        }
        Ok(Some(redraw))
    }

    /// Draws each widget and names its appearance in `/AP /N`; adds the fonts the appearances
    /// embed to the form's `/DR`.
    ///
    /// # Errors
    /// The overlay's errors.
    pub(super) fn write(
        &self,
        doc: &mut laminapdf_model::Document,
        fonts: &[BlobId],
        appearances: Option<&dyn Appearances>,
        blobs: &BlobStore,
    ) -> Result<()> {
        let generator = appearances.ok_or(Error::AppearanceUnavailable)?;
        let mut embedded: Vec<(String, ObjRef)> = Vec::new();
        for look in &self.widgets {
            let request = self.request(look, fonts, Some(blobs));
            let drawn = generator.field(&mut doc.cos_mut().edit(), &request)?;
            let Some(mut dict) = dict_at(doc, look.widget)? else {
                continue;
            };
            let ap: Dict = [("N", Object::Ref(drawn.normal))].into_iter().collect();
            dict.insert("AP", Object::Dict(ap));
            doc.cos_mut().edit().set(look.widget, Object::Dict(dict))?;
            for font in drawn.fonts {
                if !embedded.iter().any(|(name, _)| *name == font.0) {
                    embedded.push(font);
                }
            }
        }
        add_to_dr(doc, &embedded)
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
        FieldAppearanceRequest {
            rect: look.rect,
            rotation: look.rotation,
            background: look.background,
            border: look.border,
            border_width: look.border_width,
            family: s.family,
            size: s.size,
            color: s.color,
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
    let top = match dict_at(doc, field_object(field)?)?.and_then(|d| d.get("TI").cloned()) {
        Some(v) => tolerant(doc.deref(&v))?
            .and_then(|o| o.as_int())
            .and_then(|i| usize::try_from(i).ok())
            .unwrap_or(0),
        None => 0,
    };
    let da = parse_da(field.default_appearance.as_deref().unwrap_or_default());
    let family = match (da.family, da.name.as_deref()) {
        (Some(family), _) => family,
        (None, Some(name)) => dr_family(doc, name)?.unwrap_or(StandardFamily::Helvetica),
        (None, None) => StandardFamily::Helvetica,
    };
    Ok(Shown {
        text,
        rows: field.options.iter().map(|o| o.display.clone()).collect(),
        selected,
        top,
        family,
        size: da.size,
        color: da.color.unwrap_or(AnnotColor::Gray(0.0)),
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
        .and_then(|b| family_of_base_font(&String::from_utf8_lossy(b))))
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
    let Some(dict) = dict_at(doc, widget)? else {
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
    let rotation = mk
        .get("R")
        .and_then(Object::as_int)
        .map_or(0, |r| u16::try_from(r.rem_euclid(360) / 90 * 90).unwrap_or(0));
    Ok(Some(WidgetLook {
        widget,
        rect,
        rotation,
        background: color("BG")?,
        border: color("BC")?,
        border_width: border_width(doc, &dict)?,
    }))
}

/// The widget's border width: `/BS /W`, else `/Border`'s third number, else 1 (Table 168).
fn border_width(doc: &laminapdf_model::Document, dict: &Dict) -> Result<f64> {
    let number = |v: &Object| -> Result<Option<f64>> {
        Ok(tolerant(doc.deref(v))?
            .and_then(|o| o.as_f64())
            .filter(|w| w.is_finite() && *w >= 0.0))
    };
    if let Some(bs) = dict.get("BS") {
        let bs = tolerant(doc.deref(bs))?.and_then(|o| o.as_dict().cloned());
        if let Some(w) = bs.as_ref().and_then(|b| b.get("W"))
            && let Some(w) = number(w)?
        {
            return Ok(w);
        }
    }
    if let Some(border) = dict.get("Border")
        && let Some(items) = tolerant(doc.deref(border))?.and_then(|o| o.as_array().map(<[Object]>::to_vec))
        && let Some(w) = items.get(2)
        && let Some(w) = number(w)?
    {
        return Ok(w);
    }
    Ok(1.0)
}

/// Adds `fonts` to the form's `/DR /Font` under their names, but for names it already has;
/// writes the dictionary where it is (the form's own, or an indirect `/DR` or `/Font`).
fn add_to_dr(doc: &mut laminapdf_model::Document, fonts: &[(String, ObjRef)]) -> Result<()> {
    if fonts.is_empty() {
        return Ok(());
    }
    let Some(acro_ref) = doc.catalog()?.get("AcroForm").cloned() else {
        return Ok(());
    };
    let Some(mut acro) = tolerant(doc.deref(&acro_ref))?.and_then(|o| o.as_dict().cloned()) else {
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
    for (name, r) in fonts {
        if font_dict.get(name).is_none() {
            font_dict.insert(Name::from(name.as_str()), Object::Ref(*r));
        }
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

/// The dictionary `r` resolves to, if it is one and can be read.
fn dict_at(doc: &laminapdf_model::Document, r: ObjRef) -> Result<Option<Dict>> {
    Ok(tolerant(doc.deref(&Object::Ref(r)))?.and_then(|o| o.as_dict().cloned()))
}
