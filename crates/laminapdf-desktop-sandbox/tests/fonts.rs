//! The main process's font list against this machine's fonts: every OS the desktop app supports
//! ships some, and each face comes out as a font file of its own.
#![expect(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

use laminapdf_desktop_protocol::supervisor::FontServer as _;
use laminapdf_desktop_protocol::{Charsets, MAX_FRAME, ToEngine};
use laminapdf_desktop_sandbox::SystemFonts;

#[test]
fn the_system_fonts_should_list_faces_and_serve_each_as_a_standalone_font() {
    let fonts = SystemFonts::scan();
    let faces = fonts.faces();
    eprintln!("{} faces in {:?}", faces.len(), fonts.took);
    assert!(!faces.is_empty(), "no system font found");
    // The last face of the largest collection is the one a wrong offset would get wrong.
    let face = faces.last().unwrap();
    let bytes = fonts.read(face.id).unwrap();
    assert!(
        matches!(bytes.get(..4), Some([0, 1, 0, 0] | b"OTTO" | b"true")),
        "{} is not a standalone font",
        face.family
    );
    let cjk = faces.iter().filter(|f| f.charsets.contains(Charsets::GB2312)).count();
    eprintln!("{cjk} faces cover GB 2312");
}

#[test]
fn an_unknown_face_should_be_refused() {
    assert!(SystemFonts::default().read(7).is_err());
}

/// A collection of 256 faces, all one face whose family name is 2,000 characters long: the most a
/// file may claim, with names far past any real one.
fn crafted_collection() -> Vec<u8> {
    let name: Vec<u8> = "x".repeat(2_000).encode_utf16().flat_map(u16::to_be_bytes).collect();
    let mut table = vec![0, 0, 0, 1, 0, 18];
    for value in [3u16, 1, 0x409, 1, u16::try_from(name.len()).unwrap(), 0] {
        table.extend_from_slice(&value.to_be_bytes());
    }
    table.extend_from_slice(&name);
    let faces = 256u32;
    let face_at = 12 + 4 * faces;
    let table_at = face_at + 12 + 16;
    let mut file = b"ttcf".to_vec();
    file.extend_from_slice(&0x0001_0000u32.to_be_bytes());
    file.extend_from_slice(&faces.to_be_bytes());
    for _ in 0..faces {
        file.extend_from_slice(&face_at.to_be_bytes());
    }
    file.extend_from_slice(&0x0001_0000u32.to_be_bytes());
    file.extend_from_slice(&[0, 1, 0, 16, 0, 0, 0, 0]);
    file.extend_from_slice(b"name");
    file.extend_from_slice(&[0; 4]);
    file.extend_from_slice(&table_at.to_be_bytes());
    file.extend_from_slice(&u32::try_from(table.len()).unwrap().to_be_bytes());
    file.extend_from_slice(&table);
    file
}

#[test]
fn crafted_collections_should_not_push_the_font_list_past_one_frame() {
    let folder = std::env::temp_dir().join(format!("laminapdf-crafted-fonts-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let file = crafted_collection();
    // 33 files of 256 faces: past the cap on faces.
    for i in 0..33 {
        std::fs::write(folder.join(format!("crafted-{i}.ttc")), &file).unwrap();
    }
    let fonts = SystemFonts::scan_folders(std::slice::from_ref(&folder));
    let _ = std::fs::remove_dir_all(&folder);
    let faces = fonts.faces();
    assert!(fonts.dropped > 0, "nothing was dropped from {} faces", faces.len());
    assert!(faces.iter().all(|face| face.family.chars().count() <= 256));
    let mut frame = Vec::new();
    let written = laminapdf_desktop_protocol::write_frame(&mut frame, &ToEngine::Fonts { faces }, &[]);
    assert!(written.is_ok(), "{written:?}");
    assert!(
        frame.len() < MAX_FRAME as usize / 8,
        "the list takes {} bytes",
        frame.len()
    );
}
