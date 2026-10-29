//! Opens arbitrary bytes and drives every read path and model view, first with the empty
//! password, then with each password in `fuzz/seeds/open/passwords.txt` (the encrypted seeds'
//! passwords, maintained by the decryption work, B2b).
#![no_main]

use lamina_model::{Document, OpenOptions};
use libfuzzer_sys::fuzz_target;

const PASSWORDS: &str = include_str!("../seeds/open/passwords.txt");

fuzz_target!(|data: &[u8]| {
    let passwords = PASSWORDS
        .lines()
        .filter(|line| !line.starts_with('#'))
        .map(str::as_bytes);
    for password in std::iter::once(&b""[..]).chain(passwords) {
        let options = OpenOptions::default().with_password(password);
        if let Ok(doc) = Document::open(data.to_vec(), options) {
            lamina_fuzz::exercise(&doc);
            if !doc.is_encrypted() {
                // Passwords only matter for encrypted files.
                break;
            }
        }
    }
});
