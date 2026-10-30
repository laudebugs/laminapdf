//! Resources (ISO 32000-2 §7.8.3): names per page in order of first use (`GS1`, `Im1`,
//! `CS1`, …), objects per document, each written once.

use std::collections::BTreeMap;

use lamina_cos::{CosMut, Dict, Name, ObjRef, Object, Stream};

use crate::error::Result;
use crate::surface::color::{IccProfile, IccSpace};
use crate::surface::image::{Image, Samples};
use crate::surface::style::BlendMode;

/// An `ExtGState` the builder writes (§8.4.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum GsKey {
    /// `/ca` and `/CA`, as the bits of their `f64` values.
    Alpha { fill: u64, stroke: u64 },
    /// `/BM`.
    Blend(BlendMode),
}

impl GsKey {
    pub(crate) fn alpha(fill: f64, stroke: f64) -> Self {
        let unit = |v: f64| if v.is_nan() { 1.0 } else { v.clamp(0.0, 1.0) };
        Self::Alpha {
            fill: unit(fill).to_bits(),
            stroke: unit(stroke).to_bits(),
        }
    }

    fn object(self) -> Object {
        let mut d = Dict::new();
        d.insert("Type", Name::from("ExtGState"));
        match self {
            Self::Alpha { fill, stroke } => {
                d.insert("ca", f64::from_bits(fill));
                d.insert("CA", f64::from_bits(stroke));
            }
            Self::Blend(mode) => d.insert("BM", Name::from(mode.pdf_name())),
        }
        Object::Dict(d)
    }
}

/// The objects already written in this document, by key: each is written once.
#[derive(Debug, Clone, Default)]
pub(crate) struct DocResources {
    gstates: BTreeMap<GsKey, ObjRef>,
    images: BTreeMap<[u8; 32], ObjRef>,
    profiles: BTreeMap<[u8; 32], ObjRef>,
}

/// One page's resources, in order of first use (the names follow that order).
#[derive(Debug, Default)]
pub(crate) struct PageResources {
    gstates: Vec<GsKey>,
    images: Vec<Image>,
    profiles: Vec<IccProfile>,
    /// Font objects (reserved by the document's `FontRegistry`, written when the build
    /// finishes).
    fonts: Vec<ObjRef>,
    /// Whether the content uses `/ActualText` (PDF 1.5).
    actual_text: bool,
}

fn name_of(prefix: &str, index: usize) -> String {
    format!("{prefix}{}", index + 1)
}

impl PageResources {
    pub(crate) fn gstate(&mut self, key: GsKey) -> String {
        let index = self.gstates.iter().position(|k| *k == key).unwrap_or_else(|| {
            self.gstates.push(key);
            self.gstates.len() - 1
        });
        name_of("GS", index)
    }

    pub(crate) fn image(&mut self, image: &Image) -> String {
        let hash = image.data.hash;
        let index = self.images.iter().position(|i| i.data.hash == hash).unwrap_or_else(|| {
            self.images.push(image.clone());
            self.images.len() - 1
        });
        name_of("Im", index)
    }

    pub(crate) fn color_space(&mut self, profile: &IccProfile) -> String {
        let hash = profile.hash();
        let index = self.profiles.iter().position(|p| p.hash() == hash).unwrap_or_else(|| {
            self.profiles.push(profile.clone());
            self.profiles.len() - 1
        });
        name_of("CS", index)
    }

    #[cfg(feature = "text")]
    pub(crate) fn font(&mut self, object: ObjRef) -> String {
        let index = self.fonts.iter().position(|r| *r == object).unwrap_or_else(|| {
            self.fonts.push(object);
            self.fonts.len() - 1
        });
        name_of("F", index)
    }

    #[cfg(feature = "text")]
    pub(crate) fn uses_actual_text(&mut self) {
        self.actual_text = true;
    }

    /// The lowest PDF version that has every feature these resources use: `ExtGState`
    /// transparency and soft masks 1.4 (§11.6), 16-bit samples and `/ActualText` 1.5
    /// (§8.9.5.1, §14.9.4).
    pub(crate) fn required_version(&self) -> (u8, u8) {
        let mut v = (1, 0);
        if !self.gstates.is_empty() || self.images.iter().any(|i| i.data.alpha.is_some()) {
            v = v.max((1, 4));
        }
        if self.actual_text || self.images.iter().any(|i| i.data.bits == 16) {
            v = v.max((1, 5));
        }
        v
    }

    /// Writes the objects this document doesn't have yet and returns the page's
    /// `/Resources` dictionary.
    pub(crate) fn write(&self, cos: &mut CosMut<'_>, doc: &mut DocResources) -> Result<Dict> {
        let mut resources = Dict::new();
        if !self.gstates.is_empty() {
            let mut dict = Dict::new();
            for (i, key) in self.gstates.iter().enumerate() {
                let r = cached(&mut doc.gstates, *key, || cos.create(key.object()))?;
                dict.insert(Name::new(name_of("GS", i)), r);
            }
            resources.insert("ExtGState", dict);
        }
        if !self.profiles.is_empty() {
            let mut dict = Dict::new();
            for (i, profile) in self.profiles.iter().enumerate() {
                let r = cached(&mut doc.profiles, profile.hash(), || cos.create(icc_stream(profile)))?;
                dict.insert(Name::new(name_of("CS", i)), vec![Object::name("ICCBased"), r.into()]);
            }
            resources.insert("ColorSpace", dict);
        }
        if !self.fonts.is_empty() {
            let dict: Dict = self
                .fonts
                .iter()
                .enumerate()
                .map(|(i, r)| (Name::new(name_of("F", i)), Object::Ref(*r)))
                .collect();
            resources.insert("Font", dict);
        }
        if !self.images.is_empty() {
            let mut dict = Dict::new();
            for (i, image) in self.images.iter().enumerate() {
                let r = cached(&mut doc.images, image.data.hash, || write_image(cos, image))?;
                dict.insert(Name::new(name_of("Im", i)), r);
            }
            resources.insert("XObject", dict);
        }
        Ok(resources)
    }
}

/// The object for `key`: the one already written, else a new one from `create`.
fn cached<K: Ord, E>(
    map: &mut BTreeMap<K, ObjRef>,
    key: K,
    create: impl FnOnce() -> std::result::Result<ObjRef, E>,
) -> Result<ObjRef>
where
    crate::Error: From<E>,
{
    if let Some(r) = map.get(&key) {
        return Ok(*r);
    }
    let r = create()?;
    map.insert(key, r);
    Ok(r)
}

/// The `ICCBased` stream (§8.6.5.5): `/N` and the device `/Alternate`, Flate-compressed.
fn icc_stream(profile: &IccProfile) -> Object {
    let mut d = Dict::new();
    d.insert("N", i64::try_from(profile.components()).unwrap_or(0));
    let alternate = match profile.space() {
        IccSpace::Gray => "DeviceGray",
        IccSpace::Rgb => "DeviceRGB",
        IccSpace::Cmyk => "DeviceCMYK",
    };
    d.insert("Alternate", Name::from(alternate));
    Object::Stream(Stream::flate(d, profile.as_bytes()))
}

fn image_dict(width: u32, height: u32, color_space: &str, bits: u8) -> Dict {
    let mut d = Dict::new();
    d.insert("Type", Name::from("XObject"));
    d.insert("Subtype", Name::from("Image"));
    d.insert("Width", width);
    d.insert("Height", height);
    d.insert("ColorSpace", Name::from(color_space));
    d.insert("BitsPerComponent", i64::from(bits));
    d
}

/// An image XObject (§8.9.5), with its `/SMask` (§11.6.5.3) first when it has alpha.
fn write_image(cos: &mut CosMut<'_>, image: &Image) -> Result<ObjRef> {
    let data = &image.data;
    let mut dict = image_dict(data.width, data.height, data.color.pdf_name(), data.bits);
    if let Some(alpha) = &data.alpha {
        let smask = Stream::flate(image_dict(data.width, data.height, "DeviceGray", data.bits), alpha);
        dict.insert("SMask", cos.create(Object::Stream(smask))?);
    }
    if data.inverted {
        dict.insert(
            "Decode",
            vec![
                1.into(),
                0.into(),
                1.into(),
                0.into(),
                1.into(),
                0.into(),
                1.into(),
                0.into(),
            ],
        );
    }
    let stream = match &data.samples {
        Samples::Raw(samples) => Stream::flate(dict, samples),
        Samples::Jpeg(bytes) => {
            dict.insert("Filter", Name::from("DCTDecode"));
            Stream::new(dict, bytes.clone())
        }
    };
    Ok(cos.create(Object::Stream(stream))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_follow_the_order_of_first_use() {
        let mut page = PageResources::default();
        let a = page.gstate(GsKey::alpha(0.5, 0.5));
        let b = page.gstate(GsKey::Blend(BlendMode::Multiply));
        let again = page.gstate(GsKey::alpha(0.5, 0.5));
        assert_eq!([a, b, again], ["GS1", "GS2", "GS1"]);
    }

    #[test]
    fn an_image_used_twice_on_a_page_gets_one_name() {
        let mut page = PageResources::default();
        let img = Image::rgba(1, 1, &[0, 0, 0, 255]).unwrap();
        let copy = Image::rgba(1, 1, &[0, 0, 0, 255]).unwrap();
        assert_eq!([page.image(&img), page.image(&copy)], ["Im1", "Im1"]);
    }

    #[test]
    fn alpha_is_clamped_and_nan_is_opaque() {
        assert_eq!(GsKey::alpha(2.0, f64::NAN), GsKey::alpha(1.0, 1.0));
    }
}
