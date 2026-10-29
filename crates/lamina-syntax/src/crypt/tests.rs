//! Known-answer tests for authentication and decryption. Vectors were computed independently
//! with pypdf 6.21.3 (`pypdf._encryption.AlgV4` / `AlgV5`): `/ID[0]` = bytes 100..116, user
//! password `user`, owner password `owner`; for R5/R6 the file key is bytes 200..232.

use super::*;

fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn id0() -> Vec<u8> {
    (100..116).collect()
}

struct Case {
    revision: u8,
    key_length: usize,
    p: i64,
    encrypt_metadata: bool,
    o: &'static str,
    u: &'static str,
    key: &'static str,
}

const R2: Case = Case {
    revision: 2,
    key_length: 5,
    p: -4,
    encrypt_metadata: true,
    o: "94e8094419662a774442fb072e3d9f19e9d130ec09a4d0061e78fe920f7ab62f",
    u: "e5309f7fcef0389c03610b06bcc495f32ad856febd1a2f2cc9e6fb9f8f65d24a",
    key: "2ff2a880ec",
};

const R3: Case = Case {
    revision: 3,
    key_length: 16,
    p: -3904,
    encrypt_metadata: true,
    o: "0ba3835f88f90388e74e54584125ce142be0de24c6b0d37746e075b891756671",
    u: "dacd3045078807fa8c1868833d6556d228bf4e5e4e758a4164004e56fffa0108",
    key: "190942254da77e94b10cc0f23a1216e9",
};

const R4_NO_METADATA: Case = Case {
    revision: 4,
    key_length: 16,
    p: -1028,
    encrypt_metadata: false,
    o: "0ba3835f88f90388e74e54584125ce142be0de24c6b0d37746e075b891756671",
    u: "db346ff1d3d702f77547a81ea8f5e70728bf4e5e4e758a4164004e56fffa0108",
    key: "5a02e75d55aab016f4a29eb00ef52ee2",
};

fn authenticate_case(case: &Case, password: &[u8]) -> Option<Authenticated> {
    let (o, u, id) = (hex(case.o), hex(case.u), id0());
    let sec = StandardSecurity {
        revision: case.revision,
        key_length: case.key_length,
        owner: &o,
        user: &u,
        owner_encryption: &[],
        user_encryption: &[],
        perms: None,
        permissions: Permissions::new(case.p, case.revision).raw(),
        id0: &id,
        encrypt_metadata: case.encrypt_metadata,
    };
    authenticate(&sec, password)
}

mod algorithm_2 {
    use super::*;

    #[test]
    fn r2_user_password_yields_the_40_bit_key() {
        let auth = authenticate_case(&R2, b"user").unwrap();
        assert_eq!((auth.key, auth.matched), (FileKey(hex(R2.key)), PasswordKind::User));
    }

    #[test]
    fn r2_owner_password_yields_the_same_key() {
        let auth = authenticate_case(&R2, b"owner").unwrap();
        assert_eq!((auth.key, auth.matched), (FileKey(hex(R2.key)), PasswordKind::Owner));
    }

    #[test]
    fn r3_user_password_yields_the_128_bit_key() {
        let auth = authenticate_case(&R3, b"user").unwrap();
        assert_eq!((auth.key, auth.matched), (FileKey(hex(R3.key)), PasswordKind::User));
    }

    #[test]
    fn r3_owner_password_yields_the_same_key() {
        let auth = authenticate_case(&R3, b"owner").unwrap();
        assert_eq!((auth.key, auth.matched), (FileKey(hex(R3.key)), PasswordKind::Owner));
    }

    #[test]
    fn r4_without_encrypted_metadata_appends_ff_ff_ff_ff() {
        let auth = authenticate_case(&R4_NO_METADATA, b"user").unwrap();
        assert_eq!(auth.key, FileKey(hex(R4_NO_METADATA.key)));
    }

    #[test]
    fn r4_owner_password_is_accepted() {
        assert_eq!(
            authenticate_case(&R4_NO_METADATA, b"owner").map(|a| a.matched),
            Some(PasswordKind::Owner)
        );
    }

    #[test]
    fn wrong_password_is_rejected() {
        assert!(authenticate_case(&R3, b"nope").is_none());
    }

    #[test]
    fn empty_password_is_rejected_when_the_user_password_is_set() {
        assert!(authenticate_case(&R2, b"").is_none());
    }

    #[test]
    fn unknown_revision_is_rejected() {
        let case = Case { revision: 7, ..R3 };
        assert!(authenticate_case(&case, b"user").is_none());
    }

    #[test]
    fn short_o_and_u_are_rejected_without_panicking() {
        let id = id0();
        let sec = StandardSecurity {
            revision: 3,
            key_length: 16,
            owner: b"short",
            user: b"",
            owner_encryption: &[],
            user_encryption: &[],
            perms: None,
            permissions: 0,
            id0: &id,
            encrypt_metadata: true,
        };
        assert!(authenticate(&sec, b"user").is_none());
    }
}

mod algorithm_2a {
    use super::*;

    struct Modern {
        revision: u8,
        u: &'static str,
        ue: &'static str,
        o: &'static str,
        oe: &'static str,
        perms: &'static str,
    }

    // `/P` -3900 (0xFFFFF0C4), `/EncryptMetadata true`.
    const R5: Modern = Modern {
        revision: 5,
        u: "fc80ff009857666041846b781c2a75f5746ab8bbf4612c88cac4a0974740a1c23d517bd622df8a42aa7a83b0f16b3691",
        ue: "fc8d98e2912dc06c983764f96a20ef269eda35c8d64480cde1ecc9403b92e917",
        o: "3f65e8f14e7c8ef780501fe93678bdcb3b27d37b8d2264c8dc53126fc109827cfae016d29b840de3e32e7c361966a5ac",
        oe: "6ad297d8bd7173cdb97c08509f082366ff83c45f2fae9022541a0ffba8c09d1d",
        perms: "58739ee65bd680ac14ed987715242107",
    };

    const R6: Modern = Modern {
        revision: 6,
        u: "e53fc0dd3e589f0e86761aff3438d2677b4b970d3e6c8baee9aa6fdd720be97be1a85e695c9583d0833a932d591031dd",
        ue: "60dd4979a3734c81dd79bc2c8caf13fb91924a39238ac036f52d69d05034c0ff",
        o: "3721c88d1f3be136e126cc3efd1edfa971c1a0dc714547a826096b7ece202c7810582c3f9fbe3651ebc08f33204dc481",
        oe: "cd7409a56e3124286579f35bd85422c6074363ff1832b98309dfc75932df597f",
        perms: "5b3868cd20efafaae56c827f1ac579cd",
    };

    fn file_key() -> FileKey {
        FileKey((200..232).collect())
    }

    fn run(case: &Modern, password: &[u8], p: u32) -> Option<Authenticated> {
        let (u, ue, o, oe, perms) = (hex(case.u), hex(case.ue), hex(case.o), hex(case.oe), hex(case.perms));
        let sec = StandardSecurity {
            revision: case.revision,
            key_length: 32,
            owner: &o,
            user: &u,
            owner_encryption: &oe,
            user_encryption: &ue,
            perms: Some(&perms),
            permissions: p,
            id0: &[],
            encrypt_metadata: true,
        };
        authenticate(&sec, password)
    }

    #[test]
    fn r6_user_password_unwraps_the_file_key() {
        let auth = run(&R6, b"user", 0xFFFF_F0C4).unwrap();
        assert_eq!(
            (auth.key, auth.matched, auth.perms_valid),
            (file_key(), PasswordKind::User, Some(true))
        );
    }

    #[test]
    fn r6_owner_password_unwraps_the_file_key() {
        let auth = run(&R6, b"owner", 0xFFFF_F0C4).unwrap();
        assert_eq!((auth.key, auth.matched), (file_key(), PasswordKind::Owner));
    }

    #[test]
    fn r6_wrong_password_is_rejected() {
        assert!(run(&R6, b"", 0xFFFF_F0C4).is_none());
    }

    #[test]
    fn r6_perms_mismatch_is_reported_not_fatal() {
        let auth = run(&R6, b"user", 0xFFFF_FFFC).unwrap();
        assert_eq!(auth.perms_valid, Some(false));
    }

    // User and owner password both `same`.
    const R6_SAME: Modern = Modern {
        revision: 6,
        u: "6fed7b068af229f4fdc4b5dbb24f615826c0fd9761cab984a9febf2e793da8951ed2256d27aabc40e1c42feb252f51a8",
        ue: "c5f92c9f1b28aa062c90daabcc40031ac2c48dc6a8536c994c94fa9ffd53e383",
        o: "c7c1cf48ab51830530d949285f6795793a20f1405e9b6f5fbcce81fda9d78a780122ed7f1f67edf1bf7e7c7bca0d25e9",
        oe: "fa3990336728b86da0cb1c856daedfefd223e9843b473e7074a0219af2344260",
        perms: "69fa0e95b8687e449da003362de7aa92",
    };

    #[test]
    fn r6_password_that_is_both_is_reported_as_owner() {
        let auth = run(&R6_SAME, b"same", 0xFFFF_F0C4).unwrap();
        assert_eq!((auth.key, auth.matched), (file_key(), PasswordKind::Owner));
    }

    #[test]
    fn r5_user_password_unwraps_the_file_key() {
        let auth = run(&R5, b"user", 0xFFFF_F0C4).unwrap();
        assert_eq!((auth.key, auth.perms_valid), (file_key(), Some(true)));
    }

    #[test]
    fn r5_owner_password_unwraps_the_file_key() {
        assert_eq!(
            run(&R5, b"owner", 0xFFFF_F0C4).map(|a| a.matched),
            Some(PasswordKind::Owner)
        );
    }
}

mod candidates {
    use super::super::candidates;

    #[test]
    fn r4_tries_latin1_for_non_ascii_text() {
        assert_eq!(candidates("é".as_bytes(), 4), [vec![0xC3, 0xA9], vec![0xE9]]);
    }

    #[test]
    fn r6_applies_saslprep() {
        // U+00A0 NO-BREAK SPACE maps to U+0020 under SASLprep (RFC 4013 §2.1).
        assert_eq!(candidates("a\u{A0}b".as_bytes(), 6)[0], b"a b");
    }

    #[test]
    fn r6_truncates_to_127_bytes() {
        assert_eq!(candidates(&[b'x'; 200], 6), [vec![b'x'; 127]]);
    }
}

mod decrypt {
    use super::*;

    #[test]
    fn rc4_round_trips_with_the_per_object_key() {
        let key = FileKey(hex(R3.key));
        let object_key = key::object_key(&key.0, 12, 0, false);
        let cipher = rc4::apply(&object_key, b"secret text");
        assert_eq!(decrypt(Cipher::Rc4, &key, 12, 0, &cipher).data, b"secret text");
    }

    #[test]
    fn rc4_with_another_object_number_is_garbage() {
        let key = FileKey(hex(R3.key));
        let cipher = rc4::apply(&key::object_key(&key.0, 12, 0, false), b"secret text");
        assert_ne!(decrypt(Cipher::Rc4, &key, 13, 0, &cipher).data, b"secret text");
    }

    #[test]
    fn aes_128_uses_the_salted_object_key() {
        let key = FileKey(hex(R3.key));
        let object_key = key::object_key(&key.0, 3, 0, true);
        let iv = [9u8; 16];
        let mut data = iv.to_vec();
        let mut padded = b"0123456789".to_vec();
        padded.extend([6u8; 6]);
        data.extend(aes::cbc_encrypt_raw(&object_key, &iv, &padded).unwrap());
        assert_eq!(decrypt(Cipher::Aes128, &key, 3, 0, &data).data, b"0123456789");
    }

    #[test]
    fn aes_128_with_a_40_bit_file_key_still_uses_a_16_byte_object_key() {
        let key = FileKey(vec![1, 2, 3, 4, 5]);
        let object_key = key::object_key(&key.0, 3, 0, true);
        let iv = [9u8; 16];
        let mut data = iv.to_vec();
        data.extend(aes::cbc_encrypt_raw(&object_key, &iv, &[16u8; 16]).unwrap());
        let out = decrypt(Cipher::Aes128, &key, 3, 0, &data);
        assert_eq!((out.data.len(), out.well_formed), (0, true));
    }

    #[test]
    fn aes_reports_malformed_input() {
        let key = FileKey(vec![0; 32]);
        assert!(!decrypt(Cipher::Aes256, &key, 1, 0, b"not aes").well_formed);
    }

    #[test]
    fn file_key_debug_is_redacted() {
        assert_eq!(format!("{:?}", FileKey(vec![1; 16])), "FileKey(<128 bits redacted>)");
    }
}
