//! Permission flags (`/P`, ISO 32000-2 §7.6.4.2 Table 22) and the R5/R6 `/Perms` check.

use super::aes;

/// The user access permissions of an encrypted document, from `/P` (ISO 32000-2 Table 22).
///
/// `/P` is read as a 32-bit two's-complement value and only the defined bits are looked at
/// (§7.6.4.2; research ch. 04 §7.3). Under revision 2 the R3-only bits (9–12) don't exist and
/// their operations follow bits 3–6, as qpdf and Acrobat interpret them.
///
/// These are a policy marker the file asks readers to honour, not access control: anyone who can
/// open the file can decrypt it (research ch. 04 §7.5). Enforcing them is the caller's choice.
///
/// ```
/// use lamina_syntax::crypt::Permissions;
///
/// let p = Permissions::new(-3116, 4); // print, copy, form filling, accessibility; reserved bits set
/// assert!(p.print_low_res() && p.extract() && p.fill_forms() && p.accessibility());
/// assert!(!p.modify() && !p.print_high_res());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Permissions {
    bits: u32,
    revision: u8,
}

impl Permissions {
    /// Bit 3: print.
    const PRINT: u32 = 1 << 2;
    /// Bit 4: modify contents (other than by bits 6, 9, 11).
    const MODIFY: u32 = 1 << 3;
    /// Bit 5: copy or extract text and graphics.
    const EXTRACT: u32 = 1 << 4;
    /// Bit 6: add or modify annotations, fill fields.
    const ANNOTATE: u32 = 1 << 5;
    /// Bit 9 (R3+): fill existing form fields.
    const FILL_FORMS: u32 = 1 << 8;
    /// Bit 10 (R3+): extract for accessibility.
    const ACCESSIBILITY: u32 = 1 << 9;
    /// Bit 11 (R3+): assemble (insert, rotate, delete pages; bookmarks; thumbnails).
    const ASSEMBLE: u32 = 1 << 10;
    /// Bit 12 (R3+): high-quality print.
    const PRINT_HIGH: u32 = 1 << 11;

    /// Permissions from a `/P` value (any integer; its low 32 bits are used) and the handler's
    /// revision `/R`.
    #[must_use]
    pub const fn new(p: i64, revision: u8) -> Self {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "/P is a 32-bit two's-complement field: keeping the low 32 bits is the point"
        )]
        let bits = p as u32;
        Self { bits, revision }
    }

    /// The `/P` bits as an unsigned 32-bit value, unmasked.
    #[must_use]
    pub const fn raw(&self) -> u32 {
        self.bits
    }

    /// The defined bits only: 3–6, plus 9–12 from revision 3 on.
    #[must_use]
    pub const fn bits(&self) -> u32 {
        let r2 = Self::PRINT | Self::MODIFY | Self::EXTRACT | Self::ANNOTATE;
        let r3 = Self::FILL_FORMS | Self::ACCESSIBILITY | Self::ASSEMBLE | Self::PRINT_HIGH;
        self.bits & if self.revision >= 3 { r2 | r3 } else { r2 }
    }

    const fn has(self, bit: u32) -> bool {
        self.bits & bit != 0
    }

    const fn r2(self) -> bool {
        self.revision < 3
    }

    /// Bit 3: printing is allowed (possibly only at low quality, see [`Self::print_high_res`]).
    #[must_use]
    pub const fn print_low_res(&self) -> bool {
        self.has(Self::PRINT)
    }

    /// Bits 3 and 12: faithful (high-quality) printing is allowed. Under R2, bit 3 alone.
    #[must_use]
    pub const fn print_high_res(&self) -> bool {
        self.has(Self::PRINT) && (self.r2() || self.has(Self::PRINT_HIGH))
    }

    /// Bit 4: modifying the contents is allowed (other than by annotation, forms or assembly).
    #[must_use]
    pub const fn modify(&self) -> bool {
        self.has(Self::MODIFY)
    }

    /// Bit 5: copying or extracting text and graphics is allowed.
    #[must_use]
    pub const fn extract(&self) -> bool {
        self.has(Self::EXTRACT)
    }

    /// Bit 6: adding or modifying annotations (and filling fields) is allowed.
    #[must_use]
    pub const fn annotate(&self) -> bool {
        self.has(Self::ANNOTATE)
    }

    /// Bit 6 or (R3+) bit 9: filling existing form fields is allowed. Table 22: bit 6 allows
    /// filling fields, and bit 9 allows it "even if bit 6 is clear". (qpdf's "modify forms"
    /// reports bit 9 alone for R3+.)
    #[must_use]
    pub const fn fill_forms(&self) -> bool {
        self.has(Self::ANNOTATE) || (!self.r2() && self.has(Self::FILL_FORMS))
    }

    /// Bit 10: extraction for accessibility is allowed. Under R2, bit 5. (PDF 2.0 deprecates
    /// the bit and asks readers to treat it as set; this reports what the file says.)
    #[must_use]
    pub const fn accessibility(&self) -> bool {
        self.has(if self.r2() { Self::EXTRACT } else { Self::ACCESSIBILITY })
    }

    /// Bit 11 (R3+): assembling the document is allowed, "even if bit 4 is clear" (Table 22); bit
    /// 4 itself excludes assembly from R3 on. Under R2 assembly falls under bit 4.
    #[must_use]
    pub const fn assemble(&self) -> bool {
        self.has(if self.r2() { Self::MODIFY } else { Self::ASSEMBLE })
    }
}

/// Whether `/Perms` (R5/R6) decrypts, under the file key, to `P ‖ FF FF FF FF ‖ T|F ‖ "adb"`
/// matching `/P` and `/EncryptMetadata` (ISO 32000-2 Algorithm 13).
pub(crate) fn perms_match(file_key: &[u8], perms: &[u8], p: u32, encrypt_metadata: bool) -> bool {
    let Some(plain) = perms
        .get(..16)
        .and_then(|block| aes::ecb_decrypt_block(file_key, block))
    else {
        return false;
    };
    let flag = if encrypt_metadata { b'T' } else { b'F' };
    plain.get(..4) == Some(&p.to_le_bytes()[..]) && plain.get(8) == Some(&flag) && plain.get(9..12) == Some(b"adb")
}

#[cfg(test)]
mod tests {
    use super::Permissions;

    #[test]
    fn new_reads_p_as_32_bit_twos_complement() {
        assert_eq!(Permissions::new(-4, 4).raw(), 0xFFFF_FFFC);
    }

    #[test]
    fn bits_keeps_only_defined_bits() {
        assert_eq!(Permissions::new(-1, 4).bits(), 0b1111_0011_1100);
    }

    #[test]
    fn bits_under_r2_drops_r3_only_bits() {
        assert_eq!(Permissions::new(-1, 2).bits(), 0b11_1100);
    }

    #[test]
    fn fill_forms_under_r2_follows_bit_6() {
        assert!(Permissions::new(32, 2).fill_forms());
    }

    #[test]
    fn fill_forms_from_r3_follows_bit_6_or_bit_9() {
        assert_eq!(
            (
                Permissions::new(32, 3).fill_forms(),
                Permissions::new(256, 3).fill_forms(),
                Permissions::new(0, 3).fill_forms()
            ),
            (true, true, false)
        );
    }

    #[test]
    fn assemble_from_r3_ignores_bit_4() {
        assert!(!Permissions::new(8, 3).assemble());
    }

    #[test]
    fn print_high_res_needs_bit_12_from_r3() {
        assert!(!Permissions::new(4, 3).print_high_res());
    }
}
