//! The colour profiles a document carries: its output intents (ISO 32000-2 §14.11.5), the
//! catalog's and any page's, and every `ICCBased` colour space (§8.6.5.5), each profile with
//! the facts its ICC header gives (ICC.1:2022 §7.2) and its description tag (§9.2.41).
//! Profiles are decoded within the document's `Limits`; one that can't be decoded is listed
//! without a header.

use std::collections::BTreeMap;

use laminapdf_cos::{ObjRef, Object, Result};

use crate::document::{Document, tolerant};

/// The colour profiles a document carries ([`Document::color_profiles`]).
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct ColorProfiles {
    /// The output intents: the catalog's, then each page's (PDF 2.0), in order.
    pub output_intents: Vec<OutputIntent>,
    /// Every ICC profile, once each by object: the `ICCBased` colour spaces' and the output
    /// intents' profiles, by object number.
    pub profiles: Vec<ColorProfile>,
}

/// One output intent (§14.11.5, Table 401).
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct OutputIntent {
    /// `/S`: which standard it is for (`GTS_PDFA1`, `GTS_PDFX`, `ISO_PDFE1`).
    pub subtype: Option<String>,
    /// `/OutputConditionIdentifier`: the condition's name in a registry (`FOGRA39`, `sRGB`).
    pub condition_identifier: Option<String>,
    /// `/OutputCondition`: a readable description of the condition.
    pub condition: Option<String>,
    /// `/RegistryName`: the registry's URL.
    pub registry: Option<String>,
    /// `/Info`.
    pub info: Option<String>,
    /// `/DestOutputProfile`: the ICC profile, when it is an indirect stream.
    pub profile: Option<ObjRef>,
    /// The 0-based page whose `/OutputIntents` holds it; `None` for the catalog's.
    pub page: Option<u32>,
}

/// One ICC profile the document carries: an `ICCBased` colour space's stream, or an output
/// intent's profile.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ColorProfile {
    /// The profile stream.
    pub object: ObjRef,
    /// `/N`: the number of colour components.
    pub components: Option<i64>,
    /// `/Alternate`: the colour space to use instead, as a name (`DeviceRGB`), when it is one.
    pub alternate: Option<String>,
    /// An output intent's `/DestOutputProfile` is this profile.
    pub output_intent: bool,
    /// What the profile's header says; `None` when the stream can't be decoded within `Limits`
    /// or holds no ICC header.
    pub header: Option<IccHeader>,
}

/// What an ICC profile's header and description tag say (ICC.1:2022 §7.2, §9.2.41).
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct IccHeader {
    /// The profile's size in bytes, as the header states it.
    pub size: u32,
    /// The ICC version, `major.minor.bugfix` (`4.3.0`, `2.1.0`).
    pub version: String,
    /// The device class signature, trimmed (`mntr`, `prtr`, `spac`).
    pub device_class: String,
    /// The data colour space signature, trimmed (`RGB`, `CMYK`, `GRAY`, `Lab`).
    pub color_space: String,
    /// The profile connection space, trimmed (`XYZ`, `Lab`).
    pub connection_space: String,
    /// The `desc` tag's text (`sRGB IEC61966-2.1`); `None` without one.
    pub description: Option<String>,
}

/// How much of a profile is decoded for its header and description: 1 MiB, more than a
/// profile's header and tag table take.
const PROFILE_READ: usize = 1 << 20;

/// The header's length (§7.2.1) and where the tag count sits after it (§7.3.2).
const HEADER_LEN: usize = 128;

impl IccHeader {
    /// Reads the header of an ICC profile; `None` unless it has the `acsp` signature.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn parse(profile: &[u8]) -> Option<Self> {
        let header = profile.get(..HEADER_LEN)?;
        if header.get(36..40)? != b"acsp" {
            return None;
        }
        let sig = |at: usize| -> Option<String> {
            let bytes = header.get(at..at + 4)?;
            Some(String::from_utf8_lossy(bytes).trim_end_matches([' ', '\0']).to_owned())
        };
        let (major, minor) = (*header.get(8)?, *header.get(9)?);
        Some(Self {
            size: u32::from_be_bytes(header.get(..4)?.try_into().ok()?),
            version: format!("{major}.{}.{}", minor >> 4, minor & 0x0f),
            device_class: sig(12)?,
            color_space: sig(16)?,
            connection_space: sig(20)?,
            description: description(profile),
        })
    }
}

fn be_u32(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(bytes.get(at..at.checked_add(4)?)?.try_into().ok()?))
}

fn usize_at(bytes: &[u8], at: usize) -> Option<usize> {
    usize::try_from(be_u32(bytes, at)?).ok()
}

/// The `desc` tag's text: a `textDescriptionType` (v2, ASCII) or a
/// `multiLocalizedUnicodeType` (v4, its first record, UTF-16BE). Every offset and length is the
/// file's, so each sum is checked.
fn description(profile: &[u8]) -> Option<String> {
    let count = usize_at(profile, HEADER_LEN)?;
    let tag = (0..count.min(256)).find_map(|i| {
        let entry = HEADER_LEN.checked_add(4)?.checked_add(i.checked_mul(12)?)?;
        (profile.get(entry..entry.checked_add(4)?)? == b"desc").then_some(entry)
    })?;
    let (offset, size) = (
        usize_at(profile, tag.checked_add(4)?)?,
        usize_at(profile, tag.checked_add(8)?)?,
    );
    let data = profile.get(offset..offset.checked_add(size)?)?;
    let text = match data.get(..4)? {
        b"desc" => {
            let len = usize_at(data, 8)?;
            String::from_utf8_lossy(data.get(12..12usize.checked_add(len)?)?).into_owned()
        }
        b"mluc" => {
            let (len, at) = (usize_at(data, 20)?, usize_at(data, 24)?);
            let units: Vec<u16> = data
                .get(at..at.checked_add(len)?)?
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u16::from_be_bytes(*pair))
                .collect();
            String::from_utf16_lossy(&units)
        }
        _ => return None,
    };
    let text = text.trim_end_matches('\0').trim().to_owned();
    (!text.is_empty()).then_some(text)
}

impl Document {
    /// The output intents and every ICC profile the document carries. Finding the `ICCBased`
    /// colour spaces reads every object once, so call it when a view opens.
    ///
    /// # Errors
    /// Reading errors that aren't repairs (`data-pending`, a failed source).
    ///
    /// Added in 0.6.0.
    pub fn color_profiles(&self) -> Result<ColorProfiles> {
        let mut out = ColorProfiles::default();
        if let Some(intents) = self.catalog()?.get("OutputIntents") {
            self.output_intents(intents, None, &mut out.output_intents)?;
        }
        for page in self.pages()?.iter() {
            if let Some(intents) = page.dict().get("OutputIntents") {
                let index = u32::try_from(page.index()).unwrap_or(u32::MAX);
                self.output_intents(intents, Some(index), &mut out.output_intents)?;
            }
        }
        let mut spaces: BTreeMap<u32, ObjRef> = BTreeMap::new();
        for item in self.each_object() {
            if let (_, Some(object)) = item? {
                find_icc_based(&object, &mut spaces);
            }
        }
        for intent in &out.output_intents {
            if let Some(profile) = intent.profile {
                spaces.entry(profile.num).or_insert(profile);
            }
        }
        for object in spaces.into_values() {
            let in_intent = out.output_intents.iter().any(|i| i.profile == Some(object));
            out.profiles.push(self.color_profile(object, in_intent)?);
        }
        Ok(out)
    }

    fn output_intents(&self, list: &Object, page: Option<u32>, out: &mut Vec<OutputIntent>) -> Result<()> {
        let Some(list) = self.read(list)? else { return Ok(()) };
        let Some(items) = list.as_array() else { return Ok(()) };
        for item in items {
            let Some(intent) = self.read(item)? else { continue };
            let Some(dict) = intent.as_dict() else { continue };
            out.push(OutputIntent {
                subtype: dict.get_name("S").map(|n| String::from_utf8_lossy(n).into_owned()),
                condition_identifier: self.text_entry(dict, "OutputConditionIdentifier")?,
                condition: self.text_entry(dict, "OutputCondition")?,
                registry: self.text_entry(dict, "RegistryName")?,
                info: self.text_entry(dict, "Info")?,
                profile: dict.get("DestOutputProfile").and_then(Object::as_ref),
                page,
            });
        }
        Ok(())
    }

    fn color_profile(&self, object: ObjRef, output_intent: bool) -> Result<ColorProfile> {
        let stream = tolerant(self.resolve(object))?;
        let stream = match stream.as_deref() {
            Some(Object::Stream(s)) => Some(s),
            _ => None,
        };
        let dict = stream.map(|s| &s.dict);
        let header = match stream {
            // The header and its tags sit at the start: a profile is decoded only that far.
            Some(s) => {
                tolerant(self.decode_stream_prefix(s, PROFILE_READ))?.and_then(|(bytes, _)| IccHeader::parse(&bytes))
            }
            None => None,
        };
        Ok(ColorProfile {
            object,
            components: dict.and_then(|d| d.get_int("N")),
            alternate: dict
                .and_then(|d| d.get_name("Alternate"))
                .map(|n| String::from_utf8_lossy(n).into_owned()),
            output_intent,
            header,
        })
    }
}

/// Collects every `[/ICCBased n 0 R]` inside `value`, by the profile's object number.
fn find_icc_based(value: &Object, out: &mut BTreeMap<u32, ObjRef>) {
    match value {
        Object::Array(items) => {
            if let [Object::Name(name), Object::Ref(profile)] = items.as_slice()
                && name.as_bytes() == b"ICCBased"
            {
                out.entry(profile.num).or_insert(*profile);
                return;
            }
            for item in items {
                find_icc_based(item, out);
            }
        }
        Object::Dict(dict) => dict.iter().for_each(|(_, v)| find_icc_based(v, out)),
        Object::Stream(stream) => stream.dict.iter().for_each(|(_, v)| find_icc_based(v, out)),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::IccHeader;

    /// A minimal v4 profile: a header and one `desc` tag of type `mluc`.
    fn profile() -> Vec<u8> {
        let mut p = vec![0u8; 128];
        p[8] = 4;
        p[9] = 0x30;
        p[12..16].copy_from_slice(b"mntr");
        p[16..20].copy_from_slice(b"RGB ");
        p[20..24].copy_from_slice(b"XYZ ");
        p[36..40].copy_from_slice(b"acsp");
        p.extend_from_slice(&1u32.to_be_bytes());
        let text: Vec<u8> = "sRGB".encode_utf16().flat_map(u16::to_be_bytes).collect();
        let tag_at = 128 + 4 + 12;
        let size = 28 + text.len();
        p.extend_from_slice(b"desc");
        p.extend_from_slice(&u32::try_from(tag_at).unwrap().to_be_bytes());
        p.extend_from_slice(&u32::try_from(size).unwrap().to_be_bytes());
        p.extend_from_slice(b"mluc\0\0\0\0");
        p.extend_from_slice(&1u32.to_be_bytes());
        p.extend_from_slice(&12u32.to_be_bytes());
        p.extend_from_slice(b"enUS");
        p.extend_from_slice(&u32::try_from(text.len()).unwrap().to_be_bytes());
        p.extend_from_slice(&28u32.to_be_bytes());
        p.extend_from_slice(&text);
        let len = u32::try_from(p.len()).unwrap();
        p[..4].copy_from_slice(&len.to_be_bytes());
        p
    }

    #[test]
    fn a_v4_header_gives_its_signatures_and_description() {
        let header = IccHeader::parse(&profile()).unwrap();
        assert_eq!(header.version, "4.3.0");
        assert_eq!(
            (
                header.device_class.as_str(),
                header.color_space.as_str(),
                header.connection_space.as_str()
            ),
            ("mntr", "RGB", "XYZ")
        );
        assert_eq!(header.description.as_deref(), Some("sRGB"));
    }

    #[test]
    fn a_description_whose_length_runs_past_the_end_is_left_out() {
        let mut p = profile();
        // The mluc record's length (offset 144 + 20) claims nearly 4 GiB.
        let at = 128 + 4 + 12 + 20;
        p[at..at + 4].copy_from_slice(&u32::MAX.to_be_bytes());
        let header = IccHeader::parse(&p).unwrap();
        assert_eq!(header.description, None);
        // So does a tag offset at the very end of the address space.
        p[128 + 4 + 4..128 + 4 + 8].copy_from_slice(&u32::MAX.to_be_bytes());
        assert_eq!(IccHeader::parse(&p).unwrap().description, None);
    }

    #[test]
    fn bytes_without_the_acsp_signature_are_no_profile() {
        let mut p = profile();
        p[36] = b'x';
        assert_eq!(IccHeader::parse(&p), None);
    }
}
