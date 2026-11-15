//! Edits to existing objects (ADR 0022 §1, §3; todo E7d): move, resize and rotate (a `cm`
//! around the object), crop (a clip around it), delete, swap an image for another XObject, and
//! move it in the stacking order. Each rewrites only the object it edits, through
//! [`ContentOps::replace`]'s splice, so the encoder copies every other byte as it was.
//!
//! An object can leave state behind it: text-state, colour and line operators inside a text
//! object stay set after `ET`, and later text may rely on them. An edit that wraps the object
//! in `q … Q` or takes it away writes those operators again where the object was, so what
//! follows draws as before. An object that clips what follows ([`NotEditable::ClippingPath`])
//! is refused instead: its clip can't stay behind without the object.

use std::collections::BTreeSet;

use laminapdf_model::{Matrix, PageFrame, RectPt};
use laminapdf_syntax::{Name, Object};

use super::{ContentObjectId, ContentObjectKind, ContentOp, ContentOps, NotEditable};
use crate::operator::Operator;
use crate::state::{ColorSpace, CurrentColor, GraphicsState};

/// Where [`ContentOps::reorder`] puts an object in the stacking order (the order objects
/// paint: later ones paint over earlier ones).
///
/// Added in 0.5.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Placement {
    /// Over every other object: after the last one.
    Front,
    /// Under every other object: before the first one.
    Back,
    /// Just under this object.
    Before(ContentObjectId),
    /// Just over this object.
    After(ContentObjectId),
}

/// A resource an operator names, by the category of the page's `/Resources` it is in.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct ResourceName {
    /// The `/Resources` category: `XObject`, `Font`, `ExtGState`, `Shading`, `Pattern`,
    /// `ColorSpace` or `Properties`.
    pub category: &'static str,
    /// The name.
    pub name: Name,
}

/// The operators that set state an object leaves behind (§8.4.4 Table 56, §9.3.1 Table 103,
/// §8.6.8 Table 73): all of them allowed outside a text object too.
const LEAVES_STATE: [&[u8]; 27] = [
    b"Tc", b"Tw", b"Tz", b"TL", b"Tf", b"Tr", b"Ts", b"w", b"J", b"j", b"M", b"d", b"ri", b"i", b"gs", b"CS", b"cs",
    b"SC", b"SCN", b"sc", b"scn", b"G", b"g", b"RG", b"rg", b"K", b"k",
];

/// How far two clip boxes may differ, in points, and still count as the same clip.
const CLIP_SLACK: f64 = 1e-3;

impl ContentOps {
    /// Transforms the object by `matrix` in page space ([ADR 0027]): where it drew a point `p`
    /// of the page, it now draws `matrix(p)`. Written as `q`, a `cm` in the object's own space,
    /// the object and `Q`, followed by the state it leaves behind.
    ///
    /// # Errors
    /// [`NotEditable::ClippingPath`] for an object that clips what follows;
    /// [`NotEditable::ContentUnreadable`] when its graphics state wasn't recorded (see
    /// [`Recorder`](super::Recorder)) or its transformation can't be inverted.
    ///
    /// [ADR 0027]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0027-coordinates-units-and-page-indexing.md
    ///
    /// Added in 0.5.0.
    pub fn transform(&mut self, id: ContentObjectId, matrix: &Matrix, frame: &PageFrame) -> Result<(), NotEditable> {
        let (ops, ctm) = self.editable(id)?;
        let to_page = frame
            .to_user_space_matrix()
            .invert()
            .ok_or(NotEditable::ContentUnreadable)?;
        let inverse = ctm.invert().ok_or(NotEditable::ContentUnreadable)?;
        // The object's space → user space → page → moved → user space → the object's space.
        let local = ctm
            .then(&to_page)
            .then(matrix)
            .then(&frame.to_user_space_matrix())
            .then(&inverse);
        let leaked = leaked_state(&ops);
        let mut new = vec![op(b"q", Vec::new()), cm(&local)];
        new.extend(ops);
        new.push(op(b"Q", Vec::new()));
        new.extend(leaked);
        self.replace(id, new);
        Ok(())
    }

    /// Clips the object to `rect` in page space: written as `q`, the rectangle as a clipping
    /// path in the object's own space, the object and `Q`. Images, forms and shadings only.
    ///
    /// # Errors
    /// [`NotEditable::ClippingPath`] for a path or text object (their clip is edited with the
    /// path); [`NotEditable::ContentUnreadable`] as [`ContentOps::transform`].
    ///
    /// Added in 0.5.0.
    pub fn crop(&mut self, id: ContentObjectId, rect: &RectPt, frame: &PageFrame) -> Result<(), NotEditable> {
        let kind = self.object(id).map(super::ContentObject::kind);
        if matches!(kind, Some(ContentObjectKind::Path | ContentObjectKind::Text)) {
            return Err(NotEditable::ClippingPath);
        }
        let (ops, ctm) = self.editable(id)?;
        let inverse = ctm.invert().ok_or(NotEditable::ContentUnreadable)?;
        let user = frame.rect_to_user_space(rect);
        let corners = [
            (user.x0, user.y0),
            (user.x1, user.y0),
            (user.x1, user.y1),
            (user.x0, user.y1),
        ];
        let mut new = vec![op(b"q", Vec::new())];
        for (i, (x, y)) in corners.into_iter().enumerate() {
            let (lx, ly) = inverse.apply(x, y);
            new.push(op(
                if i == 0 { b"m" } else { b"l" },
                vec![Object::number(lx), Object::number(ly)],
            ));
        }
        new.extend([op(b"h", Vec::new()), op(b"W", Vec::new()), op(b"n", Vec::new())]);
        new.extend(ops);
        new.push(op(b"Q", Vec::new()));
        self.replace(id, new);
        Ok(())
    }

    /// Deletes the object: its operators go, and the state it leaves behind is written where it
    /// was, so what follows draws as before.
    ///
    /// # Errors
    /// [`NotEditable::ClippingPath`] for an object that clips what follows.
    ///
    /// Added in 0.5.0.
    pub fn delete(&mut self, id: ContentObjectId) -> Result<(), NotEditable> {
        let ops = self.own_ops(id)?;
        self.take_out(id, leaked_state(&ops));
        Ok(())
    }

    /// Replaces the image with the XObject `name` from the owner's resources, drawn through
    /// `unit`, a matrix in the image's unit square (an orientation, or the identity): the new
    /// image fills the old one's box.
    ///
    /// # Errors
    /// [`NotEditable::ContentUnreadable`] when the object isn't an image.
    ///
    /// Added in 0.5.0.
    pub fn replace_image(&mut self, id: ContentObjectId, name: Name, unit: &Matrix) -> Result<(), NotEditable> {
        if self.object(id).map(super::ContentObject::kind) != Some(ContentObjectKind::Image) {
            return Err(NotEditable::ContentUnreadable);
        }
        let draw = op(b"Do", vec![Object::Name(name)]);
        let new = if *unit == Matrix::IDENTITY {
            vec![draw]
        } else {
            vec![op(b"q", Vec::new()), cm(unit), draw, op(b"Q", Vec::new())]
        };
        self.replace(id, new);
        Ok(())
    }

    /// Moves the object in the stacking order. Where it lands it draws as before: wrapped in
    /// `q … Q` with a `cm` and the colours, line and text state it had, and the state it left
    /// behind stays where it was. Moving it next to itself changes nothing.
    ///
    /// # Errors
    /// [`NotEditable::ClippingPath`] for an object that clips what follows;
    /// [`NotEditable::StateNotPortable`] when the clip, the transparency or the font set
    /// through an `ExtGState` differ where it would go; [`NotEditable::ContentUnreadable`] when a
    /// state wasn't recorded or the object or the one it is placed by doesn't exist.
    ///
    /// Added in 0.5.0.
    pub fn reorder(&mut self, id: ContentObjectId, to: Placement) -> Result<(), NotEditable> {
        let ops = self.own_ops(id)?;
        let state = self.state_of(id)?;
        let Some((anchor, after)) = self.anchor_for(id, to) else {
            return Ok(());
        };
        let target = self.position(anchor).and_then(|i| self.objects.get(i));
        let there = if after {
            target.and_then(|o| o.end_state.as_deref())
        } else {
            target.and_then(|o| o.state.as_deref())
        }
        .ok_or(NotEditable::ContentUnreadable)?
        .clone();
        // Marked content that holds nothing but the object goes with it, so its MCID still
        // finds it (ADR 0022 §4).
        let index = self.position(id).ok_or(NotEditable::ContentUnreadable)?;
        let inner = self
            .objects
            .get(index)
            .map(super::ContentObject::operators)
            .unwrap_or_default();
        let outer = self.wrapping(inner.clone());
        let opening = self.ops.get(outer.start..inner.start).unwrap_or_default().to_vec();
        let closing = self.ops.get(inner.end..outer.end).unwrap_or_default().to_vec();
        let mut moved = opening;
        moved.extend(portable(&ops, &state, &there)?);
        moved.extend(closing);
        let anchor_at = self.ops.get(outer.start).map_or(self.source.len(), |o| o.token_start);
        self.splice(index, outer, anchor_at, leaked_state(&ops));
        let mut object = self.objects.remove(index);
        let target = self.position(anchor).ok_or(NotEditable::ContentUnreadable)?;
        let range = self
            .objects
            .get(target)
            .map(super::ContentObject::operators)
            .ok_or(NotEditable::ContentUnreadable)?;
        let (at, slot) = if after {
            (range.end, target + 1)
        } else {
            (range.start, target)
        };
        let count = moved.len();
        self.insert_ops(at, moved);
        object.ops = at..at + count;
        object.marked.clear();
        object.bounds = None;
        self.objects.insert(slot, object);
        Ok(())
    }

    /// `range` grown by every marked-content sequence that holds exactly it: a `BMC`/`BDC`
    /// just before it whose `EMC` is just after it.
    fn wrapping(&self, mut range: std::ops::Range<usize>) -> std::ops::Range<usize> {
        while let Some(start) = range.start.checked_sub(1)
            && self
                .marked
                .iter()
                .any(|seq| seq.begin == start && seq.end == Some(range.end))
        {
            range = start..range.end + 1;
        }
        range
    }

    /// Every resource the operators name, by category: what [`ContentOps`] still needs from
    /// the owner's `/Resources` after an edit.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn resource_names(&self) -> BTreeSet<ResourceName> {
        self.ops.iter().filter_map(resource_name).collect()
    }

    /// The resources the object's operators name.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn object_resource_names(&self, id: ContentObjectId) -> BTreeSet<ResourceName> {
        let range = self.object(id).map(super::ContentObject::operators).unwrap_or_default();
        self.ops
            .get(range)
            .unwrap_or_default()
            .iter()
            .filter_map(resource_name)
            .collect()
    }

    /// The object's operators and the transformation in effect before them, when it can be
    /// wrapped: it doesn't clip what follows, and its state was recorded.
    fn editable(&self, id: ContentObjectId) -> Result<(Vec<ContentOp>, Matrix), NotEditable> {
        let ops = self.own_ops(id)?;
        Ok((ops, self.state_of(id)?.ctm))
    }

    /// The object's operators, refused when it clips what follows.
    fn own_ops(&self, id: ContentObjectId) -> Result<Vec<ContentOp>, NotEditable> {
        let object = self.object(id).ok_or(NotEditable::ContentUnreadable)?;
        let ops = self.ops.get(object.operators()).unwrap_or_default().to_vec();
        let text_clips = object.kind == ContentObjectKind::Text
            && (object.state().is_some_and(|s| s.text.render_mode >= 4)
                || ops.iter().any(|o| o.keyword[..] == *b"Tr" && clipping_mode(o)));
        if text_clips || ops.iter().any(|o| matches!(&o.keyword[..], b"W" | b"W*")) {
            return Err(NotEditable::ClippingPath);
        }
        Ok(ops)
    }

    fn state_of(&self, id: ContentObjectId) -> Result<GraphicsState, NotEditable> {
        self.object(id)
            .and_then(super::ContentObject::state)
            .cloned()
            .ok_or(NotEditable::ContentUnreadable)
    }

    /// The object `to` places `id` by, and whether `id` goes after it; `None` when the object
    /// would stay where it is.
    fn anchor_for(&self, id: ContentObjectId, to: Placement) -> Option<(ContentObjectId, bool)> {
        let others = || self.objects.iter().map(|o| o.id).filter(|&o| o != id);
        let (anchor, after) = match to {
            Placement::Front => (others().next_back()?, true),
            Placement::Back => (others().next()?, false),
            Placement::Before(anchor) => (anchor, false),
            Placement::After(anchor) => (anchor, true),
        };
        let here = self.position(id)?;
        let there = self.position(anchor)?;
        let unchanged = anchor == id || (after && there + 1 == here) || (!after && here + 1 == there);
        (!unchanged).then_some((anchor, after))
    }

    /// Takes the object out of the content, writing `keep` where it was.
    fn take_out(&mut self, id: ContentObjectId, keep: Vec<ContentOp>) {
        self.replace(id, keep);
        if let Some(index) = self.position(id) {
            self.objects.remove(index);
        }
    }

    /// Inserts `new` before the operator at `at` (or at the end), written canonically, and
    /// keeps every index into the operators in step.
    fn insert_ops(&mut self, at: usize, mut new: Vec<ContentOp>) {
        self.edited = true;
        let anchor = self.ops.get(at).map_or(self.source.len(), |o| o.token_start);
        super::mark_anchor(&self.parts, &mut self.dirty, anchor);
        for op in &mut new {
            op.span = None;
            op.token_start = anchor;
        }
        let added = new.len();
        self.ops.splice(at..at, new);
        let shift = |i: usize| if i >= at { i + added } else { i };
        for object in &mut self.objects {
            object.ops = shift(object.ops.start)..shift(object.ops.end);
        }
        for seq in &mut self.marked {
            seq.begin = shift(seq.begin);
            seq.end = seq.end.map(shift);
        }
    }
}

/// The object's operators wrapped to draw at a place whose state is `there` as they drew where
/// the state was `here`: `q`, a `cm` from one transformation to the other, the colours, line
/// and text state, the operators and `Q`.
fn portable(ops: &[ContentOp], here: &GraphicsState, there: &GraphicsState) -> Result<Vec<ContentOp>, NotEditable> {
    let same_clip = match (here.clip_box, there.clip_box) {
        (None, None) => true,
        (Some(a), Some(b)) => [(a.x0, b.x0), (a.y0, b.y0), (a.x1, b.x1), (a.y1, b.y1)]
            .iter()
            .all(|(p, q)| (p - q).abs() <= CLIP_SLACK),
        _ => false,
    };
    let same_transparency = (here.stroke_alpha - there.stroke_alpha).abs() <= f64::EPSILON
        && (here.fill_alpha - there.fill_alpha).abs() <= f64::EPSILON
        && here.blend_mode == there.blend_mode
        && here.soft_mask == there.soft_mask;
    let font_portable = here.text.font.as_ref().is_none_or(|f| f.name.is_some());
    if !(same_clip && same_transparency && font_portable) {
        return Err(NotEditable::StateNotPortable);
    }
    let inverse = there.ctm.invert().ok_or(NotEditable::ContentUnreadable)?;
    let mut out = vec![op(b"q", Vec::new()), cm(&here.ctm.then(&inverse))];
    out.extend(state_ops(here));
    out.extend(ops.iter().cloned());
    out.push(op(b"Q", Vec::new()));
    Ok(out)
}

/// Operators that set `state`'s line, colour and text parameters (not the clip, transparency
/// or transformation).
fn state_ops(state: &GraphicsState) -> Vec<ContentOp> {
    let n = Object::number;
    let mut out = vec![
        op(b"w", vec![n(state.line_width)]),
        op(b"J", vec![Object::Integer(state.line_cap)]),
        op(b"j", vec![Object::Integer(state.line_join)]),
        op(b"M", vec![n(state.miter_limit)]),
        op(
            b"d",
            vec![
                Object::Array(state.dash.array.iter().copied().map(n).collect()),
                n(state.dash.phase),
            ],
        ),
        op(b"i", vec![n(state.flatness)]),
    ];
    if let Some(intent) = &state.rendering_intent {
        out.push(op(b"ri", vec![Object::Name(intent.clone())]));
    }
    out.extend(color_ops(&state.stroke, true));
    out.extend(color_ops(&state.fill, false));
    let text = &state.text;
    out.extend([
        op(b"Tc", vec![n(text.char_spacing)]),
        op(b"Tw", vec![n(text.word_spacing)]),
        op(b"Tz", vec![n(text.horizontal_scaling * 100.0)]),
        op(b"TL", vec![n(text.leading)]),
        op(b"Tr", vec![Object::Integer(text.render_mode)]),
        op(b"Ts", vec![n(text.rise)]),
    ]);
    if let Some(name) = text.font.as_ref().and_then(|f| f.name.clone()) {
        out.push(op(b"Tf", vec![Object::Name(name), n(text.font_size)]));
    }
    out
}

/// `CS`/`cs` and `SCN`/`scn` setting `color`.
fn color_ops(color: &CurrentColor, stroke: bool) -> Vec<ContentOp> {
    let space = match &color.space {
        ColorSpace::DeviceGray => Name::from("DeviceGray"),
        ColorSpace::DeviceRgb => Name::from("DeviceRGB"),
        ColorSpace::DeviceCmyk => Name::from("DeviceCMYK"),
        ColorSpace::Pattern => Name::from("Pattern"),
        ColorSpace::Named(name) => name.clone(),
    };
    let mut operands: Vec<Object> = color.components.iter().copied().map(Object::number).collect();
    operands.extend(color.pattern.clone().map(Object::Name));
    let (set_space, set_color): (&[u8], &[u8]) = if stroke { (b"CS", b"SCN") } else { (b"cs", b"scn") };
    let mut out = vec![op(set_space, vec![Object::Name(space)])];
    if !operands.is_empty() {
        out.push(op(set_color, operands));
    }
    out
}

/// The operators of `ops` that set state the object leaves behind, in order.
fn leaked_state(ops: &[ContentOp]) -> Vec<ContentOp> {
    ops.iter()
        .filter(|o| LEAVES_STATE.contains(&&o.keyword[..]))
        .cloned()
        .collect()
}

/// The resource an operator names, if any.
fn resource_name(op: &ContentOp) -> Option<ResourceName> {
    let operands = &op.operands;
    let (category, name) = match &op.keyword[..] {
        b"Do" => ("XObject", operands.first()),
        b"Tf" => ("Font", operands.first()),
        b"gs" => ("ExtGState", operands.first()),
        b"sh" => ("Shading", operands.first()),
        b"CS" | b"cs" => ("ColorSpace", operands.first()),
        b"SCN" | b"scn" => ("Pattern", operands.last()),
        b"BDC" | b"DP" => ("Properties", operands.get(1)),
        _ => return None,
    };
    match name {
        Some(Object::Name(name)) => Some(ResourceName {
            category,
            name: name.clone(),
        }),
        _ => None,
    }
}

/// Whether the operator's first operand is a text rendering mode that clips (4–7, §9.3.6).
fn clipping_mode(op: &ContentOp) -> bool {
    match op.operands.first() {
        Some(Object::Integer(mode)) => *mode >= 4,
        Some(Object::Real(mode)) => *mode >= 4.0,
        _ => false,
    }
}

fn op(keyword: &'static [u8], operands: Vec<Object>) -> ContentOp {
    ContentOp::new(Operator::from_keyword(keyword), operands)
}

fn cm(m: &Matrix) -> ContentOp {
    op(
        b"cm",
        [m.a, m.b, m.c, m.d, m.e, m.f].into_iter().map(Object::number).collect(),
    )
}

#[cfg(test)]
#[path = "edits_tests.rs"]
mod tests;
