//! The CTM, text matrices and text state after each operator, against values computed by
//! hand from ISO 32000-2 §8.3.4 (`cm`), §9.4.2 (positioning) and §9.4.4 (Trm and advances),
//! as research 03 §2.6 writes them.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure"
)]

mod common;

use common::{Event, Recorder, run_content, text_doc};
use lamina_content::{
    ColorSpace, GraphicsState, Matrix, TextAdvance, TextShow, TextState, Visitor, interpret_content, interpret_page,
};

/// The text matrix of every show.
fn tms(content: &[u8], advance_per_byte: f64) -> Vec<Matrix> {
    let doc = text_doc(b"");
    let mut rec = Recorder {
        advance_per_byte,
        ..Recorder::default()
    };
    interpret_content(&doc, content, None, &mut rec).unwrap();
    rec.events
        .iter()
        .filter_map(|e| match e {
            Event::Show { tm, .. } => Some(*tm),
            _ => None,
        })
        .collect()
}

fn translate(x: f64, y: f64) -> Matrix {
    Matrix::translate(x, y)
}

#[test]
fn cm_premultiplies_the_ctm() {
    // [1 0 0 1 10 20] × [2 0 0 2 0 0] = [2 0 0 2 20 40]
    let (_, rec) = run_content(b"2 0 0 2 0 0 cm 1 0 0 1 10 20 cm BT (a) Tj ET");
    let Some(Event::Show { ctm, .. }) = rec.shows().first().copied() else {
        panic!("one show")
    };
    assert_eq!(*ctm, Matrix::new(2.0, 0.0, 0.0, 2.0, 20.0, 40.0));
}

#[test]
fn q_and_q_restore_the_ctm() {
    let (_, rec) = run_content(b"q 3 0 0 3 0 0 cm Q BT (a) Tj ET");
    let Some(Event::Show { ctm, .. }) = rec.shows().first().copied() else {
        panic!("one show")
    };
    assert_eq!(*ctm, Matrix::IDENTITY);
}

#[test]
fn td_moves_from_the_start_of_the_line() {
    assert_eq!(
        tms(b"BT 10 20 Td (a) Tj 5 -2 Td (b) Tj ET", 0.0),
        [translate(10.0, 20.0), translate(15.0, 18.0)]
    );
}

#[test]
fn td_capital_sets_the_leading_for_t_star() {
    // 5 -14 TD: TL = 14, Tlm = [1 0 0 1 5 -14]; T*: 0 -14 Td.
    assert_eq!(
        tms(b"BT 5 -14 TD (a) Tj T* (b) Tj ET", 0.0),
        [translate(5.0, -14.0), translate(5.0, -28.0)]
    );
}

#[test]
fn tm_replaces_both_matrices_and_quote_moves_in_its_space() {
    // Tm [2 0 0 2 100 100], TL 14; ' = T* then Tj: [1 0 0 1 0 -14] × Tlm = [2 0 0 2 100 72].
    assert_eq!(
        tms(b"BT 14 TL 2 0 0 2 100 100 Tm (a) Tj (b) ' ET", 0.0),
        [
            Matrix::new(2.0, 0.0, 0.0, 2.0, 100.0, 100.0),
            Matrix::new(2.0, 0.0, 0.0, 2.0, 100.0, 72.0)
        ]
    );
}

#[test]
fn a_show_advances_the_text_matrix_by_the_visitor_s_displacement() {
    // Two bytes at 0.5 each: Tm = [1 0 0 1 1 0] × [2 0 0 2 10 0] = [2 0 0 2 12 0].
    assert_eq!(
        tms(b"BT 2 0 0 2 10 0 Tm (ab) Tj (c) Tj ET", 0.5),
        [
            Matrix::new(2.0, 0.0, 0.0, 2.0, 10.0, 0.0),
            Matrix::new(2.0, 0.0, 0.0, 2.0, 12.0, 0.0)
        ]
    );
}

#[test]
fn a_new_line_starts_from_the_line_matrix_not_the_advanced_text_matrix() {
    assert_eq!(
        tms(b"BT 10 TL 0 50 Td (abcd) Tj T* (e) Tj ET", 1.0),
        [translate(0.0, 50.0), translate(0.0, 40.0)]
    );
}

#[test]
fn bt_resets_the_text_matrices() {
    assert_eq!(tms(b"BT 10 10 Td ET BT (a) Tj ET", 0.0), [Matrix::IDENTITY]);
}

/// Captures the text state and rendering matrix of every show.
#[derive(Default)]
struct States(Vec<(TextState, Matrix)>);

impl Visitor for States {
    fn show_text(&mut self, show: &TextShow<'_>) -> TextAdvance {
        self.0.push((show.state.text.clone(), show.rendering_matrix()));
        TextAdvance::default()
    }
}

fn states(content: &[u8]) -> Vec<(TextState, Matrix)> {
    let doc = text_doc(content);
    let pages = doc.pages().unwrap();
    let mut states = States::default();
    interpret_page(&doc, &pages.get(0).unwrap(), &mut states).unwrap();
    states.0
}

#[test]
fn the_rendering_matrix_follows_9_4_4() {
    // [Tfs·Th 0 0 Tfs 0 Trise] = [6 0 0 12 0 3]; × Tm [1 0 0 1 10 20] = [6 0 0 12 10 23];
    // × CTM [2 0 0 2 0 0] = [12 0 0 24 20 46].
    let shows = states(b"2 0 0 2 0 0 cm BT /F1 12 Tf 50 Tz 3 Ts 10 20 Td (a) Tj ET");
    assert_eq!(
        shows.first().map(|(_, trm)| *trm),
        Some(Matrix::new(12.0, 0.0, 0.0, 24.0, 20.0, 46.0))
    );
}

#[test]
fn text_state_operators_set_their_parameters() {
    let shows = states(b"BT /F1 9 Tf 1 Tc 2 Tw 80 Tz 11 TL 3 Tr 4 Ts (a) Tj ET");
    let (text, _) = shows.first().unwrap();
    assert_eq!(
        (
            text.char_spacing,
            text.word_spacing,
            text.horizontal_scaling,
            text.leading,
            text.font_size,
            text.render_mode,
            text.rise
        ),
        (1.0, 2.0, 0.8, 11.0, 9.0, 3, 4.0)
    );
}

#[test]
fn double_quote_sets_word_and_char_spacing_before_showing() {
    let shows = states(b"BT /F1 9 Tf 5 0.5 (a) \" ET");
    let (text, _) = shows.first().unwrap();
    assert_eq!((text.word_spacing, text.char_spacing), (5.0, 0.5));
}

#[test]
fn text_state_survives_et_and_is_restored_by_q() {
    let shows = states(b"BT /F1 9 Tf ET q BT 7 Tc (a) Tj ET Q BT (b) Tj ET");
    let spacing: Vec<_> = shows.iter().map(|(t, _)| (t.font_size, t.char_spacing)).collect();
    assert_eq!(spacing, [(9.0, 7.0), (9.0, 0.0)]);
}

#[test]
fn tf_names_the_font_resource() {
    let doc = text_doc(b"BT /F1 9 Tf (a) Tj ET");
    let (_, rec) = common::run(&doc);
    let Some(Event::Show { font, .. }) = rec.shows().first().copied() else {
        panic!("one show")
    };
    assert_eq!(font.as_deref(), Some(&b"F1"[..]));
}

#[test]
fn a_missing_font_warns_and_keeps_its_name() {
    let doc = text_doc(b"BT /F9 9 Tf (a) Tj ET");
    let (_, rec) = common::run(&doc);
    assert_eq!(rec.codes(), ["content-resource-missing"]);
}

#[test]
fn text_positioning_outside_bt_warns_and_still_runs() {
    let (_, rec) = run_content(b"5 5 Td (a) Tj");
    assert_eq!(rec.codes(), ["content-unbalanced-bt", "content-unbalanced-bt"]);
}

/// Captures the graphics state at every paint.
#[derive(Default)]
struct Paints(Vec<GraphicsState>);

impl Visitor for Paints {
    fn paint_path(&mut self, paint: &lamina_content::PathPaint<'_>) {
        self.0.push(paint.state.clone());
    }
}

fn paint_states(build: impl FnOnce(&mut lamina_save::Writer) -> (Vec<u8>, lamina_cos::Dict)) -> Vec<GraphicsState> {
    let doc = common::page_doc(build);
    let pages = doc.pages().unwrap();
    let mut paints = Paints::default();
    interpret_page(&doc, &pages.get(0).unwrap(), &mut paints).unwrap();
    paints.0
}

#[test]
fn colour_operators_set_space_and_components() {
    let states = paint_states(|_| {
        (
            b"0.1 0.2 0.3 RG 0.5 g 0 0 0 1 k n /DeviceRGB CS 0.4 0.5 0.6 SC S".to_vec(),
            common::dict([]),
        )
    });
    let colours: Vec<_> = states
        .iter()
        .map(|s| {
            (
                s.stroke.space.clone(),
                s.stroke.components.clone(),
                s.fill.space.clone(),
            )
        })
        .collect();
    assert_eq!(
        colours,
        [
            (ColorSpace::DeviceRgb, vec![0.1, 0.2, 0.3], ColorSpace::DeviceCmyk),
            (ColorSpace::DeviceRgb, vec![0.4, 0.5, 0.6], ColorSpace::DeviceCmyk)
        ]
    );
}

#[test]
fn line_style_operators_set_their_parameters() {
    let states = paint_states(|_| {
        (
            b"3 w 1 J 2 j 4 M [2 1] 0.5 d /Perceptual ri 5 i S".to_vec(),
            common::dict([]),
        )
    });
    let s = states.first().unwrap();
    assert_eq!(
        (
            s.line_width,
            s.line_cap,
            s.line_join,
            s.miter_limit,
            s.dash.array.clone(),
            s.dash.phase,
            s.flatness
        ),
        (3.0, 1, 2, 4.0, vec![2.0, 1.0], 0.5, 5.0)
    );
}

#[test]
fn ext_gstate_sets_line_width_opacity_and_font() {
    let states = paint_states(|w| {
        let font = w.add(common::dict([("Type", lamina_cos::Object::name("Font"))]));
        let gs = common::dict([
            ("LW", 7.into()),
            ("ca", lamina_cos::Object::Real(0.25)),
            ("Font", vec![font.into(), 14.into()].into()),
        ]);
        let resources = common::dict([("ExtGState", common::dict([("GS1", gs.into())]).into())]);
        (b"/GS1 gs S".to_vec(), resources)
    });
    let s = states.first().unwrap();
    assert_eq!(
        (
            s.line_width,
            s.fill_alpha,
            s.text.font_size,
            s.text.font.as_ref().and_then(|f| f.object).is_some()
        ),
        (7.0, 0.25, 14.0, true)
    );
}

#[test]
fn a_named_colour_space_needs_its_resource() {
    let (_, rec) = run_content(b"/CS9 cs 0.5 sc");
    assert_eq!(rec.codes(), ["content-resource-missing"]);
}

// Review round 1.

fn fill_after(
    build: impl FnOnce(&mut lamina_save::Writer) -> lamina_cos::Dict,
    content: &[u8],
) -> (Vec<f64>, Vec<&'static str>) {
    let doc = common::page_doc(|w| (content.to_vec(), build(w)));
    let pages = doc.pages().unwrap();
    let mut paints = Paints::default();
    let mut rec = Recorder::default();
    interpret_page(&doc, &pages.get(0).unwrap(), &mut paints).unwrap();
    interpret_page(&doc, &pages.get(0).unwrap(), &mut rec).unwrap();
    (paints.0.first().unwrap().fill.components.clone(), rec.codes())
}

fn color_spaces(space: lamina_cos::Object) -> lamina_cos::Dict {
    common::dict([("ColorSpace", common::dict([("CS1", space)]).into())])
}

#[test]
fn an_indexed_space_starts_at_index_0() {
    use lamina_cos::Object;
    let space = vec![
        Object::name("Indexed"),
        Object::name("DeviceRGB"),
        0.into(),
        Object::String(lamina_cos::PdfString::hex(vec![0, 0, 0])),
    ];
    let (components, _) = fill_after(|_| color_spaces(space.into()), b"/CS1 cs n");
    assert_eq!(components, [0.0]);
}

#[test]
fn a_separation_space_starts_at_tint_1() {
    use lamina_cos::Object;
    let space = vec![
        Object::name("Separation"),
        Object::name("Spot"),
        Object::name("DeviceCMYK"),
        Object::Null,
    ];
    let (components, _) = fill_after(|_| color_spaces(space.into()), b"/CS1 cs n");
    assert_eq!(components, [1.0]);
}

#[test]
fn a_devicen_space_starts_at_tint_1_per_colourant() {
    use lamina_cos::Object;
    let names = vec![Object::name("A"), Object::name("B"), Object::name("C")];
    let space = vec![
        Object::name("DeviceN"),
        names.into(),
        Object::name("DeviceCMYK"),
        Object::Null,
    ];
    let (components, _) = fill_after(|_| color_spaces(space.into()), b"/CS1 cs n");
    assert_eq!(components, [1.0, 1.0, 1.0]);
}

#[test]
fn an_icc_space_starts_at_zero_per_component() {
    use lamina_cos::{Object, Stream};
    let (components, _) = fill_after(
        |w| {
            let profile = w.add(Stream::new(common::dict([("N", 4.into())]), vec![0]));
            color_spaces(vec![Object::name("ICCBased"), profile.into()].into())
        },
        b"/CS1 cs n",
    );
    assert_eq!(components, [0.0; 4]);
}

#[test]
fn a_named_space_that_is_not_a_space_warns() {
    let (_, codes) = fill_after(|_| color_spaces(7.into()), b"/CS1 cs n");
    assert_eq!(codes, ["content-resource-missing"]);
}

#[test]
fn a_pattern_resource_that_is_not_a_pattern_warns() {
    let (_, codes) = fill_after(
        |_| common::dict([("Pattern", common::dict([("P1", 7.into())]).into())]),
        b"/Pattern cs /P1 scn n",
    );
    assert_eq!(codes, ["content-resource-missing"]);
}

#[test]
fn ext_gstate_entries_may_be_indirect() {
    let states = paint_states(|w| {
        let width = w.add(lamina_cos::Object::Integer(5));
        let gs = common::dict([("LW", width.into())]);
        (
            b"/GS1 gs S".to_vec(),
            common::dict([("ExtGState", common::dict([("GS1", gs.into())]).into())]),
        )
    });
    assert_eq!(states.first().map(|s| s.line_width), Some(5.0));
}

#[test]
fn an_ext_gstate_font_that_is_missing_warns() {
    let doc = common::page_doc(|w| {
        let missing = w.reserve();
        let gs = common::dict([("Font", vec![missing.into(), 12.into()].into())]);
        w.set(missing, lamina_cos::Object::Null);
        (
            b"/GS1 gs".to_vec(),
            common::dict([("ExtGState", common::dict([("GS1", gs.into())]).into())]),
        )
    });
    let (_, rec) = common::run(&doc);
    assert_eq!(rec.codes(), ["content-resource-missing"]);
}

#[test]
fn a_missing_font_keeps_its_name() {
    let doc = text_doc(b"BT /F9 9 Tf (a) Tj ET");
    let (_, rec) = common::run(&doc);
    let Some(Event::Show { font, .. }) = rec.shows().first().copied() else {
        panic!("one show")
    };
    assert_eq!(font.as_deref(), Some(&b"F9"[..]));
}

#[test]
fn text_shown_outside_bt_is_still_reported() {
    let (_, rec) = run_content(b"(a) Tj");
    assert_eq!(rec.shows().len(), 1);
}

#[test]
fn a_stray_et_warns() {
    let (_, rec) = run_content(b"ET");
    assert_eq!(rec.codes(), ["content-unbalanced-bt"]);
}

#[test]
fn q_and_cm_inside_bt_run_without_a_warning() {
    let (_, rec) = run_content(b"BT q 2 0 0 2 0 0 cm (a) Tj Q (b) Tj ET");
    let ctms: Vec<_> = rec
        .shows()
        .iter()
        .filter_map(|e| match e {
            Event::Show { ctm, .. } => Some(ctm.a),
            _ => None,
        })
        .collect();
    assert_eq!((ctms, rec.codes()), (vec![2.0, 1.0], vec![]));
}

#[test]
fn a_missing_ext_gstate_is_skipped_with_a_warning() {
    let (_, rec) = run_content(b"/GS9 gs");
    assert_eq!(rec.codes(), ["content-resource-missing"]);
}

#[test]
fn a_missing_shading_is_skipped_with_a_warning() {
    let (_, rec) = run_content(b"/Sh9 sh");
    assert_eq!((rec.events.len(), rec.codes()), (0, vec!["content-resource-missing"]));
}

#[test]
fn a_stray_id_is_content_syntax() {
    let (_, rec) = run_content(b"ID");
    assert_eq!(rec.codes(), ["content-syntax"]);
}
