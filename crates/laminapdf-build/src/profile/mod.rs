//! Conformance profiles: PDF/UA-1 (ISO 14289-1) and PDF/A-2b, PDF/A-2u, PDF/A-3b (ISO 19005-2
//! and -3). A profile chosen in [`BuildOptions::with_profile`](crate::BuildOptions::with_profile)
//! makes [`Builder::finish`](crate::Builder::finish) check the machine-checkable rules Lamina PDF's
//! output could break, and write what the standards ask for: the identification in the XMP
//! packet, `/DisplayDocTitle`, and for PDF/A the output intent with an embedded sRGB profile.
//!
//! * `checks`: the rules, run before anything is written (research 03 §9.4, the Matterhorn
//!   Protocol 1.1's machine-checkable conditions; ISO 19005-2 §6).
//! * `pdfa`: the output intent and its ICC profile.
//!
//! What no program can check (alternate text that describes the figure, a sensible reading
//! order, colour that isn't the only carrier of meaning) stays the caller's responsibility.

pub(crate) mod checks;
pub(crate) mod pdfa;

/// A conformance profile ([`BuildOptions::with_profile`](crate::BuildOptions::with_profile)).
///
/// PDF/UA-1 combines with one PDF/A profile (a document can be both accessible and archival);
/// choosing a second PDF/A profile replaces the first.
///
/// ```
/// use laminapdf_build::{BuildOptions, Profile};
///
/// let options = BuildOptions::default().with_profile(Profile::PdfA2u).with_profile(Profile::PdfUa1);
/// assert_eq!(options.profiles().collect::<Vec<_>>(), [Profile::PdfUa1, Profile::PdfA2u]);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Profile {
    /// No profile: clears every profile chosen so far.
    None,
    /// PDF/UA-1 (ISO 14289-1): tagged, with a title, a language, alternate text and embedded
    /// fonts that map every glyph to Unicode.
    PdfUa1,
    /// PDF/A-2b (ISO 19005-2, level B): the document looks the same in the future.
    PdfA2b,
    /// PDF/A-2u (level U): level B, and every glyph maps to Unicode.
    PdfA2u,
    /// PDF/A-3b (ISO 19005-3, level B): PDF/A-2b that may carry attached files of any kind.
    PdfA3b,
}

impl Profile {
    /// The profile's usual name: `"PDF/UA-1"`, `"PDF/A-2b"` and so on (`"none"`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::PdfUa1 => "PDF/UA-1",
            Self::PdfA2b => "PDF/A-2b",
            Self::PdfA2u => "PDF/A-2u",
            Self::PdfA3b => "PDF/A-3b",
        }
    }
}

/// The PDF/A part and level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PdfA {
    A2b,
    A2u,
    A3b,
}

impl PdfA {
    /// `pdfaid:part`.
    pub(crate) const fn part(self) -> u8 {
        match self {
            Self::A2b | Self::A2u => 2,
            Self::A3b => 3,
        }
    }

    /// `pdfaid:conformance`.
    pub(crate) const fn conformance(self) -> char {
        match self {
            Self::A2b | Self::A3b => 'B',
            Self::A2u => 'U',
        }
    }

    pub(crate) const fn profile(self) -> Profile {
        match self {
            Self::A2b => Profile::PdfA2b,
            Self::A2u => Profile::PdfA2u,
            Self::A3b => Profile::PdfA3b,
        }
    }
}

/// The profiles of one build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Conformance {
    pub(crate) ua: bool,
    pub(crate) pdfa: Option<PdfA>,
}

impl Conformance {
    pub(crate) const fn with(mut self, profile: Profile) -> Self {
        match profile {
            Profile::None => return Self { ua: false, pdfa: None },
            Profile::PdfUa1 => self.ua = true,
            Profile::PdfA2b => self.pdfa = Some(PdfA::A2b),
            Profile::PdfA2u => self.pdfa = Some(PdfA::A2u),
            Profile::PdfA3b => self.pdfa = Some(PdfA::A3b),
        }
        self
    }

    /// The profiles, PDF/UA first.
    pub(crate) fn profiles(self) -> impl Iterator<Item = Profile> {
        self.ua
            .then_some(Profile::PdfUa1)
            .into_iter()
            .chain(self.pdfa.map(PdfA::profile))
    }

    pub(crate) const fn is_empty(self) -> bool {
        !self.ua && self.pdfa.is_none()
    }

    /// The profile an error names when a rule belongs to every profile chosen (the PDF
    /// version, embedded fonts): the PDF/A one, the stricter about both, else PDF/UA-1.
    pub(crate) fn primary(self) -> Option<Profile> {
        self.pdfa.map(PdfA::profile).or(self.ua.then_some(Profile::PdfUa1))
    }

    /// The profile that needs every glyph mapped to Unicode (PDF/A-2u §6.2.11.7.2, PDF/UA-1
    /// §7.21.7), PDF/A-2u first; `None` when no profile does.
    #[cfg(feature = "text")]
    pub(crate) const fn unicode_profile(self) -> Option<Profile> {
        match (self.pdfa, self.ua) {
            (Some(PdfA::A2u), _) => Some(Profile::PdfA2u),
            (_, true) => Some(Profile::PdfUa1),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_pdfa_profile_replaces_the_first() {
        let c = Conformance::default().with(Profile::PdfA2b).with(Profile::PdfA3b);
        assert_eq!(c.profiles().collect::<Vec<_>>(), [Profile::PdfA3b]);
    }

    #[test]
    fn the_pdfa_profile_is_named_before_ua() {
        let c = Conformance::default().with(Profile::PdfUa1).with(Profile::PdfA2u);
        assert_eq!(c.primary(), Some(Profile::PdfA2u));
    }

    #[test]
    fn none_clears_every_profile() {
        let c = Conformance::default()
            .with(Profile::PdfUa1)
            .with(Profile::PdfA2u)
            .with(Profile::None);
        assert!(c.is_empty());
    }

    #[cfg(feature = "text")]
    #[test]
    fn pdfa_2u_and_ua_need_unicode_but_2b_and_3b_do_not() {
        let need = |p| Conformance::default().with(p).unicode_profile();
        assert_eq!(
            [
                need(Profile::PdfA2b),
                need(Profile::PdfA2u),
                need(Profile::PdfUa1),
                need(Profile::PdfA3b)
            ],
            [None, Some(Profile::PdfA2u), Some(Profile::PdfUa1), None]
        );
    }
}
