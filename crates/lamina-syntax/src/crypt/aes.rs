//! AES-CBC as the standard security handler uses it (ISO 32000-2 §7.6.3): strings and streams
//! are `16-byte IV ‖ CBC(PKCS#5-padded plaintext)`. The chaining is done here over the RustCrypto
//! `aes` block cipher; decryption is tolerant (a bad length or padding keeps the data and is
//! reported), and encryption exists only for Algorithm 2.B's inner AES-128 step.

use aes::cipher::{BlockCipherDecrypt, BlockCipherEncrypt, KeyInit};
use aes::{Aes128, Aes256, Block};

const BLOCK: usize = 16;

/// An AES key of either size the handler uses. Short-lived and on the stack, so the size
/// difference between the variants doesn't matter.
#[expect(clippy::large_enum_variant, reason = "built per call and dropped immediately")]
enum Key {
    Aes128(Aes128),
    Aes256(Aes256),
}

impl Key {
    /// `None` unless `key` is 16 or 32 bytes long.
    fn new(key: &[u8]) -> Option<Self> {
        match key.len() {
            16 => Aes128::new_from_slice(key).ok().map(Self::Aes128),
            32 => Aes256::new_from_slice(key).ok().map(Self::Aes256),
            _ => None,
        }
    }

    fn decrypt(&self, block: &mut Block) {
        match self {
            Self::Aes128(k) => k.decrypt_block(block),
            Self::Aes256(k) => k.decrypt_block(block),
        }
    }

    fn encrypt(&self, block: &mut Block) {
        match self {
            Self::Aes128(k) => k.encrypt_block(block),
            Self::Aes256(k) => k.encrypt_block(block),
        }
    }
}

fn block_of(chunk: &[u8]) -> Block {
    let mut bytes = [0u8; BLOCK];
    for (dst, src) in bytes.iter_mut().zip(chunk) {
        *dst = *src;
    }
    Block::from(bytes)
}

fn xor_into(out: &mut [u8; BLOCK], other: &[u8]) {
    for (o, x) in out.iter_mut().zip(other) {
        *o ^= x;
    }
}

/// CBC-decrypts whole blocks of `data` with `iv`, without touching padding. `None` for a bad
/// key length or data that is not a whole number of blocks.
pub(crate) fn cbc_decrypt_raw(key: &[u8], iv: &[u8; BLOCK], data: &[u8]) -> Option<Vec<u8>> {
    let key = Key::new(key)?;
    if !data.len().is_multiple_of(BLOCK) {
        return None;
    }
    let mut out = Vec::with_capacity(data.len());
    let mut prev = *iv;
    let (blocks, _) = data.as_chunks::<BLOCK>();
    for chunk in blocks {
        let mut block = Block::from(*chunk);
        key.decrypt(&mut block);
        let mut plain: [u8; BLOCK] = block.into();
        xor_into(&mut plain, &prev);
        out.extend_from_slice(&plain);
        prev = *chunk;
    }
    Some(out)
}

/// CBC-encrypts whole blocks (no padding); Algorithm 2.B only. `None` for a bad key length or
/// data that is not a whole number of blocks.
pub(crate) fn cbc_encrypt_raw(key: &[u8], iv: &[u8; BLOCK], data: &[u8]) -> Option<Vec<u8>> {
    let key = Key::new(key)?;
    if !data.len().is_multiple_of(BLOCK) {
        return None;
    }
    let mut out = Vec::with_capacity(data.len());
    let mut prev = *iv;
    let (blocks, _) = data.as_chunks::<BLOCK>();
    for chunk in blocks {
        let mut plain = *chunk;
        xor_into(&mut plain, &prev);
        let mut block = Block::from(plain);
        key.encrypt(&mut block);
        prev = block.into();
        out.extend_from_slice(&prev);
    }
    Some(out)
}

/// Decrypts one ECB block (the R6 `/Perms` check). `None` for a bad key or block length.
pub(crate) fn ecb_decrypt_block(key: &[u8], data: &[u8]) -> Option<[u8; BLOCK]> {
    let key = Key::new(key)?;
    if data.len() != BLOCK {
        return None;
    }
    let mut block = block_of(data);
    key.decrypt(&mut block);
    Some(block.into())
}

/// The result of [`decrypt`]: the plaintext and whether the input was well-formed.
pub(crate) struct Plain {
    pub(crate) data: Vec<u8>,
    /// False when the data was not `IV ‖ whole blocks` (kept as it was) or its PKCS#5 padding
    /// was invalid (decrypted, kept unstripped).
    pub(crate) valid: bool,
}

/// Decrypts `IV ‖ ciphertext`, stripping PKCS#5 padding. Tolerant (ADR 0003): empty input is
/// empty plaintext; input that is not an IV plus at least one whole block is kept as it is, and
/// a decrypted block with invalid padding is kept unstripped, both with `valid: false`.
pub(crate) fn decrypt(key: &[u8], data: &[u8]) -> Plain {
    if data.is_empty() {
        return Plain {
            data: Vec::new(),
            valid: true,
        };
    }
    let invalid = || Plain {
        data: data.to_vec(),
        valid: false,
    };
    let (Some(iv), Some(body)) = (data.get(..BLOCK), data.get(BLOCK..)) else {
        return invalid();
    };
    if body.is_empty() {
        return invalid();
    }
    let iv: [u8; BLOCK] = block_of(iv).into();
    let Some(mut plain) = cbc_decrypt_raw(key, &iv, body) else {
        return invalid();
    };
    let pad = plain.last().copied().map_or(0, usize::from);
    let padding_ok = (1..=BLOCK).contains(&pad)
        && plain
            .len()
            .checked_sub(pad)
            .and_then(|start| plain.get(start..))
            .is_some_and(|tail| tail.iter().all(|b| usize::from(*b) == pad));
    if padding_ok {
        plain.truncate(plain.len().saturating_sub(pad));
    }
    Plain {
        data: plain,
        valid: padding_ok,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `IV ‖ CBC(PKCS#5(plain))`, for building test inputs.
    fn encrypt(key: &[u8], iv: [u8; BLOCK], plain: &[u8]) -> Vec<u8> {
        let pad = BLOCK - plain.len() % BLOCK;
        let mut padded = plain.to_vec();
        padded.extend(std::iter::repeat_n(u8::try_from(pad).unwrap(), pad));
        let mut out = iv.to_vec();
        out.extend(cbc_encrypt_raw(key, &iv, &padded).unwrap());
        out
    }

    // FIPS-197 Appendix C.1 (AES-128) and C.3 (AES-256) known answers.
    #[test]
    fn ecb_matches_fips_197_aes_128() {
        let key: Vec<u8> = (0u8..16).collect();
        let cipher = hex("69c4e0d86a7b0430d8cdb78070b4c55a");
        assert_eq!(
            ecb_decrypt_block(&key, &cipher).unwrap(),
            hex16("00112233445566778899aabbccddeeff")
        );
    }

    #[test]
    fn ecb_matches_fips_197_aes_256() {
        let key: Vec<u8> = (0u8..32).collect();
        let cipher = hex("8ea2b7ca516745bfeafc49904b496089");
        assert_eq!(
            ecb_decrypt_block(&key, &cipher).unwrap(),
            hex16("00112233445566778899aabbccddeeff")
        );
    }

    // NIST SP 800-38A F.2.1 CBC-AES128.Encrypt, first two blocks.
    #[test]
    fn cbc_encrypt_matches_sp_800_38a() {
        let key = hex("2b7e151628aed2a6abf7158809cf4f3c");
        let iv = hex16("000102030405060708090a0b0c0d0e0f");
        let plain = hex("6bc1bee22e409f96e93d7e117393172aae2d8a571e03ac9c9eb76fac45af8e51");
        assert_eq!(
            cbc_encrypt_raw(&key, &iv, &plain).unwrap(),
            hex("7649abac8119b246cee98e9b12e9197d5086cb9b507219ee95db113a917678b2")
        );
    }

    #[test]
    fn decrypt_strips_valid_padding() {
        let key = [7u8; 16];
        let plain = decrypt(&key, &encrypt(&key, [1; BLOCK], b"hello world"));
        assert_eq!((plain.data.as_slice(), plain.valid), (&b"hello world"[..], true));
    }

    #[test]
    fn decrypt_handles_a_full_padding_block() {
        let key = [7u8; 32];
        let plain = decrypt(&key, &encrypt(&key, [2; BLOCK], &[b'x'; 16]));
        assert_eq!((plain.data, plain.valid), (vec![b'x'; 16], true));
    }

    #[test]
    fn decrypt_keeps_data_with_invalid_padding() {
        let key = [7u8; 16];
        let iv = [0u8; BLOCK];
        let mut data = iv.to_vec();
        data.extend(cbc_encrypt_raw(&key, &iv, &[0u8; 16]).unwrap());
        let plain = decrypt(&key, &data);
        assert_eq!((plain.data, plain.valid), (vec![0u8; 16], false));
    }

    #[test]
    fn decrypt_keeps_data_that_is_not_whole_blocks() {
        let plain = decrypt(&[7u8; 16], b"plaintext (PDFex)");
        assert_eq!((plain.data.as_slice(), plain.valid), (&b"plaintext (PDFex)"[..], false));
    }

    #[test]
    fn decrypt_of_empty_input_is_empty_and_valid() {
        let plain = decrypt(&[7u8; 16], b"");
        assert_eq!((plain.data.len(), plain.valid), (0, true));
    }

    #[test]
    fn decrypt_keeps_data_for_a_bad_key_length() {
        let plain = decrypt(&[7u8; 5], &[0u8; 32]);
        assert!(!plain.valid);
    }

    fn hex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    fn hex16(s: &str) -> [u8; 16] {
        hex(s).try_into().unwrap()
    }
}
