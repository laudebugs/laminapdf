//! Read-only inspection across the boundary (the extract and text modules, beside the properties
//! view): `conformance()`, `fonts(pages)`, `colorProfiles()`, `trailer()`, `objects()`, `object(num)`,
//! `streamBytes(num, view, max)`, `referencesTo(num)`, `objectSyntax(num)` and `revisions()`.

use js_sys::{Object, Reflect, Uint8Array};
use laminapdf::{
    ColorProfile, ColorProfiles, ConformanceClaims, CosValue, InspectedObject, ObjectEntry, ObjectLocation, OutputIntent,
    Referrer, RevisionSummary, StreamView,
};
use serde::Serialize;
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use crate::convert::{from_js, to_js};
use crate::document::WasmDocument;
use crate::error::{JsResult, invalid_argument, lamina_error};
use crate::properties::FontJs;

/// The version a file declares and the standards it claims, each claimed, not verified.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConformanceJs {
    /// The `%PDF-x.y` header's version.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub header_version: Option<String>,
    /// The catalog's `/Version`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub catalog_version: Option<String>,
    /// The PDF/A claim: its part and conformance level.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub pdfa: Option<PdfaJs>,
    /// The PDF/UA part claimed.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub pdfua: Option<u8>,
    /// The PDF/X version claimed (`PDF/X-4`).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub pdfx: Option<String>,
    /// The Info dictionary's PDF/X version when the XMP metadata claims another.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub pdfx_info_conflict: Option<String>,
}

/// A PDF/A claim.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PdfaJs {
    /// The part of ISO 19005.
    pub part: u8,
    /// The conformance level (`A`, `B`, `U`, `E`, `F`).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub conformance: Option<String>,
}

impl From<ConformanceClaims> for ConformanceJs {
    fn from(c: ConformanceClaims) -> Self {
        Self {
            header_version: c.header_version,
            catalog_version: c.catalog_version,
            pdfa: c.pdfa.map(|p| PdfaJs {
                part: p.part,
                conformance: p.conformance,
            }),
            pdfua: c.pdfua,
            pdfx: c.pdfx,
            pdfx_info_conflict: c.pdfx_info_conflict,
        }
    }
}

/// The output intents and ICC profiles a document carries.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ColorProfilesJs {
    /// The output intents: the catalog's, then each page's.
    pub output_intents: Vec<OutputIntentJs>,
    /// Every ICC profile, once each, by object number.
    pub profiles: Vec<ColorProfileJs>,
}

/// One output intent.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OutputIntentJs {
    /// `/S`: the standard it is for (`GTS_PDFA1`, `GTS_PDFX`).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub subtype: Option<String>,
    /// `/OutputConditionIdentifier`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub condition_identifier: Option<String>,
    /// `/OutputCondition`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub condition: Option<String>,
    /// `/RegistryName`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub registry: Option<String>,
    /// `/Info`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub info: Option<String>,
    /// The profile's object number.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub profile: Option<u32>,
    /// The 0-based page whose output intents hold it; absent for the catalog's.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub page: Option<u32>,
}

/// One ICC profile and what its header says.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ColorProfileJs {
    /// The profile stream's object number.
    pub object: u32,
    /// `/N`: the number of colour components.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub components: Option<i64>,
    /// `/Alternate`, when it is a name.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub alternate: Option<String>,
    /// An output intent's profile.
    pub output_intent: bool,
    /// The ICC version (`4.3.0`).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub version: Option<String>,
    /// The device class (`mntr`, `prtr`).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub device_class: Option<String>,
    /// The colour space (`RGB`, `CMYK`, `GRAY`, `Lab`).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub color_space: Option<String>,
    /// The profile connection space (`XYZ`, `Lab`).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub connection_space: Option<String>,
    /// The profile's description (`sRGB IEC61966-2.1`).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub description: Option<String>,
}

impl From<OutputIntent> for OutputIntentJs {
    fn from(i: OutputIntent) -> Self {
        Self {
            subtype: i.subtype,
            condition_identifier: i.condition_identifier,
            condition: i.condition,
            registry: i.registry,
            info: i.info,
            profile: i.profile.map(|r| r.num),
            page: i.page,
        }
    }
}

impl From<ColorProfile> for ColorProfileJs {
    fn from(p: ColorProfile) -> Self {
        let header = p.header;
        Self {
            object: p.object.num,
            components: p.components,
            alternate: p.alternate,
            output_intent: p.output_intent,
            version: header.as_ref().map(|h| h.version.clone()),
            device_class: header.as_ref().map(|h| h.device_class.clone()),
            color_space: header.as_ref().map(|h| h.color_space.clone()),
            connection_space: header.as_ref().map(|h| h.connection_space.clone()),
            description: header.and_then(|h| h.description),
        }
    }
}

impl From<ColorProfiles> for ColorProfilesJs {
    fn from(c: ColorProfiles) -> Self {
        Self {
            output_intents: c.output_intents.into_iter().map(OutputIntentJs::from).collect(),
            profiles: c.profiles.into_iter().map(ColorProfileJs::from).collect(),
        }
    }
}

/// One object: its number and generation, kind, `/Type` and `/Subtype`, and where it is stored.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ObjectEntryJs {
    /// Its number.
    pub num: u32,
    /// Its generation.
    pub generation: u16,
    /// What it is.
    #[tsify(
        type = "\"null\" | \"boolean\" | \"integer\" | \"real\" | \"string\" | \"name\" | \"array\" | \"dictionary\" | \"stream\" | \"reference\""
    )]
    pub kind: &'static str,
    /// A dictionary's or stream's `/Type`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub type_name: Option<String>,
    /// A dictionary's or stream's `/Subtype` (`/S` for a structure element).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub subtype: Option<String>,
    /// Where it is stored: at an `offset` in the file, in object stream `stream` at `index`,
    /// or `created` in this session.
    pub location: LocationJs,
    /// A stream's stored length in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub stream_length: Option<u64>,
}

/// Where an object is stored.
#[derive(Debug, Serialize, Tsify)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(crate) enum LocationJs {
    /// At this byte offset in the file.
    Offset {
        /// The byte offset.
        offset: u64,
    },
    /// Inside an object stream.
    ObjectStream {
        /// The object stream's number.
        stream: u32,
        /// The object's index within it.
        index: u32,
    },
    /// Created in this session.
    Created,
}

impl From<&ObjectEntry> for ObjectEntryJs {
    fn from(e: &ObjectEntry) -> Self {
        Self {
            num: e.id.num,
            generation: e.id.gen_nr,
            kind: e.kind.name(),
            type_name: e.type_name.clone(),
            subtype: e.subtype.clone(),
            location: match e.location {
                ObjectLocation::Offset { offset, .. } => LocationJs::Offset { offset },
                ObjectLocation::ObjectStream { stream, index } => LocationJs::ObjectStream { stream, index },
                _ => LocationJs::Created,
            },
            stream_length: e.stream_length,
        }
    }
}

/// A PDF value: `{ type, … }` by its type.
#[derive(Debug, Serialize, Tsify)]
#[serde(tag = "type", rename_all = "camelCase")]
pub(crate) enum CosValueJs {
    /// `null`.
    Null,
    /// `true` or `false`.
    Boolean {
        /// The value.
        value: bool,
    },
    /// An integer.
    Integer {
        /// The value.
        value: i64,
    },
    /// A real number.
    Real {
        /// The value.
        value: f64,
    },
    /// A string: its bytes as a text string, and whether the file wrote it in hexadecimal.
    String {
        /// The bytes as a text string (§7.9.2.2).
        text: String,
        /// Written as `<…>`.
        #[serde(rename = "writtenAsHex")]
        written_as_hex: bool,
    },
    /// A name, without its `/`.
    Name {
        /// The name.
        value: String,
    },
    /// An array.
    Array {
        /// Its items.
        items: Vec<CosValueJs>,
    },
    /// A dictionary's entries in the file's order, keys without their `/`.
    Dictionary {
        /// Key and value pairs.
        #[tsify(type = "[string, CosValueJs][]")]
        entries: Vec<(String, CosValueJs)>,
    },
    /// A reference to an indirect object.
    Reference {
        /// Its number.
        num: u32,
        /// Its generation.
        generation: u16,
    },
}

impl From<&CosValue> for CosValueJs {
    fn from(v: &CosValue) -> Self {
        match v {
            CosValue::Boolean(value) => Self::Boolean { value: *value },
            CosValue::Integer(value) => Self::Integer { value: *value },
            CosValue::Real(value) => Self::Real { value: *value },
            CosValue::String { text, hex, .. } => Self::String {
                text: text.clone(),
                written_as_hex: *hex,
            },
            CosValue::Name(value) => Self::Name { value: value.clone() },
            CosValue::Array(items) => Self::Array {
                items: items.iter().map(Self::from).collect(),
            },
            CosValue::Dictionary(entries) => Self::Dictionary {
                entries: entries.iter().map(|(k, v)| (k.clone(), Self::from(v))).collect(),
            },
            CosValue::Reference { num, generation } => Self::Reference {
                num: *num,
                generation: *generation,
            },
            _ => Self::Null,
        }
    }
}

/// One object with its value.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InspectedObjectJs {
    /// Its number, kind, type and place.
    pub entry: ObjectEntryJs,
    /// Its value (a stream's dictionary for a stream).
    pub value: CosValueJs,
    /// A stream's filters and stored length.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub stream: Option<StreamFactsJs>,
    /// The 0-based revision that last wrote it; absent for one this session created.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub revision: Option<u32>,
}

/// A stream's filters and stored length.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StreamFactsJs {
    /// The `/Filter` names, in decoding order.
    pub filters: Vec<String>,
    /// The stored length in bytes.
    pub length: u64,
}

impl From<InspectedObject> for InspectedObjectJs {
    fn from(o: InspectedObject) -> Self {
        Self {
            entry: ObjectEntryJs::from(&o.entry),
            value: CosValueJs::from(&o.value),
            stream: o.stream.map(|s| StreamFactsJs {
                filters: s.filters,
                length: s.length,
            }),
            revision: o.revision,
        }
    }
}

/// One place that refers to an object.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReferrerJs {
    /// The object holding the reference; absent for the trailer.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub from: Option<u32>,
    /// Where in it (`/Kids[2]`).
    pub path: String,
}

impl From<&Referrer> for ReferrerJs {
    fn from(r: &Referrer) -> Self {
        Self {
            from: r.from.map(|r| r.num),
            path: r.path.clone(),
        }
    }
}

/// One revision of the file, oldest first.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RevisionJs {
    /// Its 0-based index.
    pub index: u32,
    /// The byte offset of the cross-reference section that starts it.
    pub section: u64,
    /// Its length in bytes.
    pub bytes: u64,
    /// Objects it adds.
    pub added: Vec<u32>,
    /// Objects an earlier revision had that it rewrites.
    pub changed: Vec<u32>,
    /// Objects it frees.
    pub freed: Vec<u32>,
    /// A signature covers exactly this revision (counted, not verified).
    pub signed: bool,
}

impl From<RevisionSummary> for RevisionJs {
    fn from(r: RevisionSummary) -> Self {
        Self {
            index: r.index,
            section: r.section,
            bytes: r.bytes,
            added: r.added,
            changed: r.changed,
            freed: r.freed,
            signed: r.signed,
        }
    }
}

#[wasm_bindgen]
impl WasmDocument {
    /// The version the file declares and the standards it claims.
    ///
    /// # Errors
    /// `data-pending` and the reading errors.
    #[wasm_bindgen(unchecked_return_type = "ConformanceJs")]
    pub fn conformance(&self) -> JsResult<JsValue> {
        let claims = self.shared().borrow().conformance().map_err(|e| lamina_error(&e))?;
        Ok(to_js(&ConformanceJs::from(claims)))
    }

    /// The fonts the pages `pages` (0-based; every page when absent) use, once each.
    ///
    /// # Errors
    /// `invalid-argument` for an index that isn't a whole number from 0;
    /// `page-index-out-of-range`; the reading errors.
    #[wasm_bindgen(unchecked_return_type = "FontJs[]")]
    pub fn fonts(
        &self,
        #[wasm_bindgen(unchecked_param_type = "number[] | undefined")] pages: &JsValue,
    ) -> JsResult<JsValue> {
        // Read through serde, which refuses a negative or fractional index (`invalid-argument`).
        let pages: Option<Vec<u32>> = if pages.is_undefined() {
            None
        } else {
            Some(from_js(pages, "pages")?)
        };
        let fonts = self
            .shared()
            .borrow()
            .fonts(pages.as_deref())
            .map_err(|e| lamina_error(&e))?;
        Ok(to_js(&fonts.iter().map(FontJs::from).collect::<Vec<_>>()))
    }

    /// The output intents and every ICC profile the document carries.
    ///
    /// # Errors
    /// `data-pending` and the reading errors.
    #[wasm_bindgen(js_name = colorProfiles, unchecked_return_type = "ColorProfilesJs")]
    pub fn color_profiles(&self) -> JsResult<JsValue> {
        let profiles = self.shared().borrow().color_profiles().map_err(|e| lamina_error(&e))?;
        Ok(to_js(&ColorProfilesJs::from(profiles)))
    }

    /// The trailer as it is now: a `dictionary` value, the root of the object tree.
    #[wasm_bindgen(unchecked_return_type = "CosValueJs")]
    pub fn trailer(&self) -> JsValue {
        to_js(&CosValueJs::from(&self.shared().borrow().trailer()))
    }

    /// Every object that exists now, by ascending number.
    ///
    /// # Errors
    /// `data-pending` and the reading errors.
    #[wasm_bindgen(unchecked_return_type = "ObjectEntryJs[]")]
    pub fn objects(&self) -> JsResult<JsValue> {
        let objects = self.shared().borrow().objects().map_err(|e| lamina_error(&e))?;
        Ok(to_js(&objects.iter().map(ObjectEntryJs::from).collect::<Vec<_>>()))
    }

    /// How many objects exist now, without reading any.
    #[wasm_bindgen(js_name = objectCount)]
    pub fn object_count(&self) -> u32 {
        self.shared().borrow().object_count()
    }

    /// Object `num`'s number, kind, type and place, or `undefined` when the document has no
    /// such object.
    ///
    /// # Errors
    /// `data-pending` and the reading errors.
    #[wasm_bindgen(js_name = objectEntry, unchecked_return_type = "ObjectEntryJs | undefined")]
    pub fn object_entry(&self, num: u32) -> JsResult<JsValue> {
        let entry = self.shared().borrow().object_entry(num).map_err(|e| lamina_error(&e))?;
        Ok(entry.map_or(JsValue::UNDEFINED, |e| to_js(&ObjectEntryJs::from(&e))))
    }

    /// Object `num` with its value, or `undefined` when the document has no such object.
    ///
    /// # Errors
    /// `data-pending` and the reading errors.
    #[wasm_bindgen(unchecked_return_type = "InspectedObjectJs | undefined")]
    pub fn object(&self, num: u32) -> JsResult<JsValue> {
        let object = self.shared().borrow().object(num).map_err(|e| lamina_error(&e))?;
        Ok(object.map_or(JsValue::UNDEFINED, |o| to_js(&InspectedObjectJs::from(o))))
    }

    /// The first `max` bytes of stream `num`, `"raw"` or `"decoded"`, and its whole length in
    /// that view (`total`, absent for a decoded stream longer than `max`, which is decoded no
    /// further): `{ bytes, total }`; `undefined` when object `num` isn't a stream.
    ///
    /// # Errors
    /// `invalid-argument` for another view; `limit-exceeded` when it decodes past the
    /// document's limits; decoding errors.
    #[wasm_bindgen(
        js_name = streamBytes,
        unchecked_return_type = "{ bytes: Uint8Array<ArrayBuffer>; total?: number } | undefined"
    )]
    pub fn stream_bytes(&self, num: u32, view: &str, max: u32) -> JsResult<JsValue> {
        let view = match view {
            "raw" => StreamView::Raw,
            "decoded" => StreamView::Decoded,
            other => {
                return Err(invalid_argument(
                    "view",
                    format!("\"{other}\" isn't a stream view"),
                    vec!["raw".to_owned(), "decoded".to_owned()],
                ));
            }
        };
        let max = usize::try_from(max).unwrap_or(usize::MAX);
        let read = self
            .shared()
            .borrow()
            .stream_bytes(num, view, max)
            .map_err(|e| lamina_error(&e))?;
        let Some(read) = read else {
            return Ok(JsValue::UNDEFINED);
        };
        let out = Object::new();
        Reflect::set(&out, &"bytes".into(), &Uint8Array::from(read.bytes.as_slice()))?;
        // A stream's length stays far below 2^53.
        #[expect(clippy::cast_precision_loss, reason = "stream lengths stay below 2^53")]
        let total = read.total.map_or(JsValue::UNDEFINED, |t| JsValue::from_f64(t as f64));
        Reflect::set(&out, &"total".into(), &total)?;
        Ok(out.into())
    }

    /// The places that refer to object `num`: the trailer, then objects by number.
    ///
    /// # Errors
    /// `data-pending` and the reading errors.
    #[wasm_bindgen(js_name = referencesTo, unchecked_return_type = "ReferrerJs[]")]
    pub fn references_to(&self, num: u32) -> JsResult<JsValue> {
        let referrers = self
            .shared()
            .borrow()
            .references_to(num)
            .map_err(|e| lamina_error(&e))?;
        Ok(to_js(&referrers.iter().map(ReferrerJs::from).collect::<Vec<_>>()))
    }

    /// Object `num` written as PDF syntax, a stream's data left out; `undefined` when the
    /// document has no such object.
    ///
    /// # Errors
    /// `data-pending` and the reading errors.
    #[wasm_bindgen(js_name = objectSyntax)]
    pub fn object_syntax(&self, num: u32) -> JsResult<Option<String>> {
        self.shared().borrow().object_syntax(num).map_err(|e| lamina_error(&e))
    }

    /// The file's revisions, oldest first.
    ///
    /// # Errors
    /// `data-pending` and the reading errors.
    #[wasm_bindgen(unchecked_return_type = "RevisionJs[]")]
    pub fn revisions(&self) -> JsResult<JsValue> {
        let revisions = self.shared().borrow().revisions().map_err(|e| lamina_error(&e))?;
        Ok(to_js(&revisions.into_iter().map(RevisionJs::from).collect::<Vec<_>>()))
    }
}

#[cfg(test)]
mod tests {
    use laminapdf::ObjectKind;

    /// `ObjectEntryJs::kind`'s TypeScript type lists every name of [`ObjectKind::ALL`], in order.
    #[test]
    fn the_typescript_type_lists_every_object_kind() {
        let union = ObjectKind::ALL
            .iter()
            .map(|k| format!("\\\"{}\\\"", k.name()))
            .collect::<Vec<_>>()
            .join(" | ");
        assert!(include_str!("inspect.rs").contains(&union), "{union}");
    }
}
