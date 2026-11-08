//! The probe's exports. JavaScript writes the inputs with `alloc` and `input`, then calls
//! `invoice` or `report`, which return the PDF's length (0 on error). Throwaway code.
#![allow(unsafe_code, clippy::missing_safety_doc)]

use std::cell::RefCell;

thread_local! {
    static INPUTS: RefCell<Vec<Vec<u8>>> = const { RefCell::new(Vec::new()) };
    static LAST: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// Where the last PDF is (its length is what `invoice` or `report` returned).
#[unsafe(no_mangle)]
pub extern "C" fn last() -> *const u8 {
    LAST.with(|l| l.borrow().as_ptr())
}

fn keep(pdf: Vec<u8>) -> usize {
    let len = pdf.len();
    LAST.with(|l| *l.borrow_mut() = pdf);
    len
}

/// Memory for an input of `len` bytes.
#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut buf = vec![0_u8; len];
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

/// Takes the `len` bytes at `ptr` (from `alloc`) as input `slot`: 0–3 the fonts, 4 the invoice
/// JSON, 5 the invoice's Typst template, 6 Markdown or a template's text.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn input(slot: usize, ptr: *mut u8, len: usize) {
    // SAFETY: `ptr` came from `alloc(len)` and is given back once.
    let bytes = unsafe { Vec::from_raw_parts(ptr, len, len) };
    INPUTS.with(|i| {
        let mut i = i.borrow_mut();
        if i.len() <= slot {
            i.resize(slot + 1, Vec::new());
        }
        i[slot] = bytes;
    });
}

fn inputs() -> Vec<Vec<u8>> {
    INPUTS.with(|i| i.borrow().clone())
}

#[unsafe(no_mangle)]
pub extern "C" fn invoice() -> usize {
    run(true)
}

#[unsafe(no_mangle)]
pub extern "C" fn report() -> usize {
    run(false)
}

#[allow(unused_variables)]
fn run(invoice: bool) -> usize {
    let i = inputs();
    let fonts = i.get(0..4).map(<[Vec<u8>]>::to_vec).unwrap_or_default();
    #[cfg(feature = "own")]
    {
        use lamina_flow_layout::{inputs as docs, own};
        let doc = if invoice {
            docs::invoice(std::str::from_utf8(&i[4]).unwrap_or_default()).ok()
        } else {
            Some(docs::report())
        };
        return doc
            .and_then(|d| own::build(&d, &fonts, false).ok())
            .and_then(|(mut d, _, _)| d.save(&laminapdf::SaveOptions::default()).ok())
            .map_or(0, |s| keep(s.bytes));
    }
    #[cfg(all(feature = "typst", not(feature = "own")))]
    {
        use lamina_flow_layout::{inputs as docs, typst_world::MemoryWorld};
        let world = if invoice {
            let main = String::from_utf8(i[5].clone()).unwrap_or_default();
            MemoryWorld::new(main, &[("invoice.json", i[4].clone())], &fonts)
        } else {
            MemoryWorld::new(docs::to_typst(&docs::report()), &[], &fonts)
        };
        #[cfg(feature = "frames")]
        {
            return lamina_flow_layout::typst_pdf::layout(&world)
                .ok()
                .and_then(|doc| lamina_flow_layout::typst_frames::draw(&doc, usize::MAX).ok())
                .map_or(0, |(pdf, _)| keep(pdf));
        }
        #[cfg(not(feature = "frames"))]
        return lamina_flow_layout::typst_pdf::build(&world).map_or(0, |o| keep(o.pdf));
    }
    #[cfg(feature = "markdown")]
    {
        let text = std::str::from_utf8(i.get(6).map_or(&[][..], Vec::as_slice)).unwrap_or_default();
        return pulldown_cmark::Parser::new_ext(text, pulldown_cmark::Options::all()).count();
    }
    #[cfg(feature = "template")]
    {
        let text = std::str::from_utf8(i.get(6).map_or(&[][..], Vec::as_slice)).unwrap_or_default();
        let env = minijinja::Environment::new();
        return env.render_str(text, minijinja::context! { n => 1 }).map_or(0, |s| s.len());
    }
    #[allow(unreachable_code)]
    {
        // The baseline: one text box, saved.
        let mut b = laminapdf::Builder::new(laminapdf::BuildOptions::new());
        let Ok(font) = laminapdf::Font::from_bytes(fonts.first().cloned().unwrap_or_default()) else { return 0 };
        let Ok(font) = b.font(font) else { return 0 };
        let mut page = b.page(laminapdf::PageSize::A4);
        let area = laminapdf::RectPt::from_points(56.0, 72.0, 480.0, 600.0);
        let options = laminapdf::TextBoxOptions::new();
        if page.surface().text_box("Hello", &font, laminapdf::Pt::new(11.0), area, &options).is_err() {
            return 0;
        }
        if page.finish().is_err() {
            return 0;
        }
        b.finish().ok().and_then(|mut d| d.save(&laminapdf::SaveOptions::default()).ok()).map_or(0, |s| s.bytes.len())
    }
}
