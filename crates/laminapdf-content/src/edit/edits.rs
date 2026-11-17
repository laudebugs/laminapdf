//! Edits to existing objects (ADR 0022 §1, §3): move, resize and rotate (a `cm`
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
    /// was, so what follows draws as before. A marked-content sequence that holds nothing but the
    /// object goes with it, so its MCID leaves the content (ADR 0022 §4).
    ///
    /// # Errors
    /// [`NotEditable::ClippingPath`] for an object that clips what follows.
    ///
    /// Added in 0.5.0.
    pub fn delete(&mut self, id: ContentObjectId) -> Result<(), NotEditable> {
        let ops = self.own_ops(id)?;
        let index = self.position(id).ok_or(NotEditable::ContentUnreadable)?;
        let inner = self
            .objects
            .get(index)
            .map(super::ContentObject::operators)
            .unwrap_or_default();
        let outer = self.wrapping(inner.clone());
        if outer == inner {
            self.take_out(id, leaked_state(&ops));
        } else {
            let anchor = self.ops.get(outer.start).map_or(self.source.len(), |o| o.token_start);
            self.splice(index, outer, anchor, leaked_state(&ops));
            self.objects.remove(index);
        }
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
        let new_draw = if *unit == Matrix::IDENTITY {
            vec![draw]
        } else {
            vec![op(b"q", Vec::new()), cm(unit), draw, op(b"Q", Vec::new())]
        };
        // Only the `Do` (or the inline image) changes: the rest of the object's operators, the
        // state it sets and an earlier edit's `q … cm … Q` around it, stay. An XObject image and
        // an inline image both fill the unit square, so the new one fills the same box.
        let mut new = Vec::new();
        for old in self.own_ops(id)? {
            if old.keyword[..] == *b"Do" || old.inline_image.is_some() {
                new.extend(new_draw.iter().cloned());
            } else {
                new.push(old);
            }
        }
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
        let Some((anchor, after)) = self.anchor_for(id, to)? else {
            return Ok(());
        };
        // The state after a clipping object includes its clip, which its recorded states
        // don't: nothing can be placed after it as it would draw there.
        if after && matches!(self.own_ops(anchor), Err(NotEditable::ClippingPath)) {
            return Err(NotEditable::StateNotPortable);
        }
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
        // Every refusal comes before this point: `anchor_for` found the anchor, and it isn't the
        // moved object, so the lookups below find it after the moved object is taken out.
        self.splice(index, outer, anchor_at, leaked_state(&ops));
        let mut object = self.objects.remove(index);
        let target = self.position(anchor).ok_or(NotEditable::ContentUnreadable)?;
        let range = self
            .objects
            .get(target)
            .map(super::ContentObject::operators)
            .ok_or(NotEditable::ContentUnreadable)?;
        // Outside the marked content that starts or ends with the anchor, so the moved object
        // joins no other structure element.
        let (at, slot) = if after {
            (self.past_closing(range.end), target + 1)
        } else {
            (self.before_opening(range.start), target)
        };
        let count = moved.len();
        self.insert_ops(at, moved);
        object.ops = at..at + count;
        object.marked.clear();
        object.bounds = None;
        // It now draws in its new place's state, inside its own `q … Q`.
        object.state = Some(Box::new(there.clone()));
        object.end_state = Some(Box::new(there));
        self.objects.insert(slot, object);
        self.reordered = true;
        Ok(())
    }

    /// `at` moved past every `EMC` right at it that closes a sequence begun before it.
    fn past_closing(&self, mut at: usize) -> usize {
        while self.marked.iter().any(|seq| seq.end == Some(at) && seq.begin < at) {
            at += 1;
        }
        at
    }

    /// `at` moved before every `BMC`/`BDC` right before it whose sequence holds it.
    fn before_opening(&self, mut at: usize) -> usize {
        while let Some(start) = at.checked_sub(1)
            && self
                .marked
                .iter()
                .any(|seq| seq.begin == start && seq.end.is_none_or(|end| end >= at))
        {
            at = start;
        }
        at
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

    /// Why the object can't be moved, deleted or reordered, or `None` when it can: it clips
    /// what follows ([`NotEditable::ClippingPath`]), or its graphics state wasn't recorded
    /// ([`NotEditable::ContentUnreadable`]). An editor shows such an object read-only.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn edit_refusal(&self, id: ContentObjectId) -> Option<NotEditable> {
        self.editable(id).err()
    }

    /// Every resource the operators name, by category: what [`ContentOps`] still needs from
    /// the owner's `/Resources` after an edit.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub fn resource_names(&self) -> BTreeSet<ResourceName> {
        self.ops.iter().flat_map(resource_names_of).collect()
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
            .flat_map(resource_names_of)
            .collect()
    }

    /// The object's operators and the transformation in effect before them, when it can be
    /// wrapped: it doesn't clip what follows, and its state was recorded.
    fn editable(&self, id: ContentObjectId) -> Result<(Vec<ContentOp>, Matrix), NotEditable> {
        let ops = self.own_ops(id)?;
        Ok((ops, self.state_of(id)?.ctm))
    }

    /// The object's operators, refused when it clips what follows.
    pub(super) fn own_ops(&self, id: ContentObjectId) -> Result<Vec<ContentOp>, NotEditable> {
        let object = self.object(id).ok_or(NotEditable::ContentUnreadable)?;
        let ops = self.ops.get(object.operators()).unwrap_or_default().to_vec();
        let text_clips = object.kind == ContentObjectKind::Text
            && (object.state().is_some_and(|s| s.text.render_mode >= 4)
                || ops.iter().any(|o| o.keyword[..] == *b"Tr" && clipping_mode(o)));
        if text_clips || clips_after(&ops) {
            return Err(NotEditable::ClippingPath);
        }
        Ok(ops)
    }

    pub(super) fn state_of(&self, id: ContentObjectId) -> Result<GraphicsState, NotEditable> {
        self.object(id)
            .and_then(super::ContentObject::state)
            .cloned()
            .ok_or(NotEditable::ContentUnreadable)
    }

    /// The object `to` places `id` by, and whether `id` goes after it; `None` when the object
    /// would stay where it is.
    ///
    /// # Errors
    /// [`NotEditable::ContentUnreadable`] when `id` or the object named by `to` doesn't exist
    /// (an earlier edit deleted it).
    fn anchor_for(&self, id: ContentObjectId, to: Placement) -> Result<Option<(ContentObjectId, bool)>, NotEditable> {
        let others = || self.objects.iter().map(|o| o.id).filter(|&o| o != id);
        let (anchor, after) = match to {
            Placement::Front => match others().next_back() {
                Some(last) => (last, true),
                None => return Ok(None),
            },
            Placement::Back => match others().next() {
                Some(first) => (first, false),
                None => return Ok(None),
            },
            Placement::Before(anchor) => (anchor, false),
            Placement::After(anchor) => (anchor, true),
        };
        let here = self.position(id).ok_or(NotEditable::ContentUnreadable)?;
        let there = self.position(anchor).ok_or(NotEditable::ContentUnreadable)?;
        let unchanged = anchor == id || (after && there + 1 == here) || (!after && here + 1 == there);
        Ok((!unchanged).then_some((anchor, after)))
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
            // A range that ends at `at` stays where it is: the new operators follow it.
            let start = shift(object.ops.start);
            let end = if object.ops.end > at || object.ops.start >= at {
                object.ops.end + added
            } else {
                object.ops.end
            };
            object.ops = start..end;
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

/// Whether `ops` set a clip that outlasts them: a `W` or `W*` outside every `q … Q` they open
/// (an earlier crop's clip sits inside its own `q … Q`).
fn clips_after(ops: &[ContentOp]) -> bool {
    let mut depth = 0_usize;
    ops.iter().any(|o| match &o.keyword[..] {
        b"q" => {
            depth += 1;
            false
        }
        b"Q" => {
            depth = depth.saturating_sub(1);
            false
        }
        b"W" | b"W*" => depth == 0,
        _ => false,
    })
}

/// The operators of `ops` that set state the object leaves behind, in order.
fn leaked_state(ops: &[ContentOp]) -> Vec<ContentOp> {
    // State set inside a `q … Q` the operators open (an earlier edit's wrapper) ends there.
    let mut depth = 0_usize;
    let mut out = Vec::new();
    for o in ops {
        match &o.keyword[..] {
            b"q" => depth += 1,
            b"Q" => depth = depth.saturating_sub(1),
            _ if depth > 0 => {}
            keyword if LEAVES_STATE.contains(&keyword) => out.push(o.clone()),
            // `tx ty TD` also sets the leading to -ty (§9.4.2), and `aw ac string "` sets the
            // word and character spacing (§9.4.3); both are text-object operators, so the state
            // is written as `TL`, `Tw` and `Tc`, which are allowed anywhere.
            b"TD" => {
                if let Some(ty) = o.operands.get(1).and_then(Object::as_f64) {
                    out.push(op(b"TL", vec![Object::number(-ty)]));
                }
            }
            b"\"" => {
                if let [aw, ac, ..] = o.operands.as_slice() {
                    out.push(op(b"Tw", vec![aw.clone()]));
                    out.push(op(b"Tc", vec![ac.clone()]));
                }
            }
            _ => {}
        }
    }
    out
}

/// The resources an operator names: one for most, the colour space an inline image names by
/// `/CS` or `/ColorSpace` (§8.9.7).
fn resource_names_of(op: &ContentOp) -> Vec<ResourceName> {
    match &op.inline_image {
        Some(image) => inline_color_space(image)
            .map(|name| ResourceName {
                category: "ColorSpace",
                name,
            })
            .into_iter()
            .collect(),
        None => resource_name(op).into_iter().collect(),
    }
}

/// The colour space an inline image names in its dictionary (before `ID`), if it names one
/// rather than giving a device space by its abbreviation.
fn inline_color_space(image: &[u8]) -> Option<Name> {
    let dict = image
        .windows(2)
        .position(|w| w == b"ID")
        .map_or(image, |end| image.get(..end).unwrap_or(image));
    let mut tokens = dict
        .split(|&b| b.is_ascii_whitespace() || b == b'/')
        .filter(|t| !t.is_empty());
    while let Some(token) = tokens.next() {
        if token == b"CS" || token == b"ColorSpace" {
            let name = tokens.next()?;
            let device = [
                &b"G"[..],
                b"RGB",
                b"CMYK",
                b"I",
                b"DeviceGray",
                b"DeviceRGB",
                b"DeviceCMYK",
                b"Indexed",
            ];
            return (!device.contains(&name)).then(|| Name::new(name));
        }
    }
    None
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

pub(super) fn op(keyword: &'static [u8], operands: Vec<Object>) -> ContentOp {
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
