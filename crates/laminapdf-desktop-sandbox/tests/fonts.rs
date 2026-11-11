//! The main process's font list against this machine's fonts: every OS the desktop app supports
//! ships some, and each face comes out as a font file of its own.
#![allow(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

use laminapdf_desktop_protocol::Charsets;
use laminapdf_desktop_protocol::supervisor::FontServer as _;
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
