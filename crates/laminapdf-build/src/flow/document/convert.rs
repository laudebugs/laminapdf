//! A [`FlowDocument`] as a [`Flow`]: styles, fonts and images resolved by name, each refusal an
//! `invalid-document` with the JSON Pointer of the name at fault.

use std::collections::BTreeMap;

use laminapdf_text::{Alignment, Standard14};

use super::{
    AlignSpec, Block, CellSpec, ColumnSpec, ContentsSpec, FlowDocument, ListItemSpec, ListKindSpec, ListSpec,
    MarginsSpec, PageSizeSpec, PageSpec, ParagraphSpec, RegionSpec, Resources, RuleSpec, Span, StyleSpec, Text,
};
use crate::builder::Builder;
use crate::error::{Error, Result};
use crate::flow::{
    ColumnWidth, Contents, Figure, Flow, FlowBlock, List, ListItem, ListKind, Margins, PageRegion, PageTemplate,
    Paragraph, ParagraphStyle, Table, TableCell, TableRow, TableRule, TextStyle,
};
use crate::nav::dest::LinkTarget;
use crate::page::PageSize;
use crate::surface::color::Color;

/// `invalid-document` at `pointer` for `reason`.
fn invalid(pointer: &str, reason: &'static str) -> Error {
    Error::InvalidDocument {
        pointer: pointer.to_owned(),
        reason,
        did_you_mean: Vec::new(),
    }
}

/// The resolved styles and the images, while one document is converted.
struct Ctx<'a> {
    styles: BTreeMap<&'a str, TextStyle>,
    resources: &'a Resources,
}

impl Ctx<'_> {
    /// The style `name` names (`body` without one); `pointer` is where the name stands.
    fn style(&self, name: Option<&String>, pointer: &str) -> Result<&TextStyle> {
        let name = name.map_or("body", String::as_str);
        self.styles.get(name).ok_or_else(|| {
            let known: Vec<String> = self.styles.keys().map(|k| (*k).to_owned()).collect();
            Error::InvalidDocument {
                pointer: pointer.to_owned(),
                reason: "unknown-style",
                did_you_mean: laminapdf_syntax::did_you_mean(name, &known),
            }
        })
    }
}

impl FlowDocument {
    /// The document as a flow, with its fonts and images from `resources` (a style may also
    /// name a standard 14 font, which `builder` registers): `invalid-document` for a style,
    /// font or image the document names but doesn't have, a page size name it doesn't know, or
    /// a colour that isn't `#rrggbb`.
    pub(crate) fn to_flow(&self, builder: &mut Builder, resources: &Resources) -> Result<Flow> {
        let mut styles = BTreeMap::new();
        for (name, spec) in &self.styles {
            styles.insert(
                name.as_str(),
                style(builder, resources, spec, &format!("/styles/{}", escape(name)))?,
            );
        }
        let ctx = Ctx { styles, resources };
        let mut flow = Flow::new(template(&ctx, self.page.as_ref())?);
        if let Some(title) = &self.title {
            flow = flow.with_title(title.clone());
        }
        if let Some(outline) = self.outline {
            flow = flow.with_outline(outline);
        }
        for (i, block) in self.blocks.iter().enumerate() {
            flow = flow.block(convert_block(&ctx, block, &format!("/blocks/{i}"))?);
        }
        Ok(flow)
    }
}

/// `name` as a JSON Pointer segment (RFC 6901 §3).
fn escape(name: &str) -> String {
    name.replace('~', "~0").replace('/', "~1")
}

/// A font given beside the document, else a standard 14 font, by `name`.
fn font(builder: &mut Builder, resources: &Resources, name: &str, pointer: &str) -> Result<crate::FontHandle> {
    if let Some(font) = resources.fonts.get(name) {
        return Ok(font.clone());
    }
    match Standard14::from_base_font(name) {
        Some(standard) => builder.standard_font(standard),
        None => {
            let known: Vec<String> = resources.fonts.keys().cloned().collect();
            Err(Error::InvalidDocument {
                pointer: pointer.to_owned(),
                reason: "unknown-font",
                did_you_mean: laminapdf_syntax::did_you_mean(name, &known),
            })
        }
    }
}

fn color(text: &str, pointer: &str) -> Result<Color> {
    Color::hex(text).map_err(|_| invalid(pointer, "wrong-type"))
}

fn style(builder: &mut Builder, resources: &Resources, spec: &StyleSpec, pointer: &str) -> Result<TextStyle> {
    let mut style = TextStyle::new(
        &font(builder, resources, &spec.font, &format!("{pointer}/font"))?,
        spec.size,
    );
    for (i, name) in spec.fallbacks.iter().enumerate() {
        style = style.with_fallback(&font(builder, resources, name, &format!("{pointer}/fallbacks/{i}"))?);
    }
    if let Some(text) = &spec.color {
        style = style.with_color(color(text, &format!("{pointer}/color"))?);
    }
    if let Some(language) = &spec.language {
        style = style.with_language(language.clone());
    }
    Ok(style.with_strikethrough(spec.strikethrough))
}

fn page_size(spec: &PageSizeSpec, pointer: &str) -> Result<PageSize> {
    const NAMES: [(&str, PageSize); 10] = [
        ("A0", PageSize::A0),
        ("A1", PageSize::A1),
        ("A2", PageSize::A2),
        ("A3", PageSize::A3),
        ("A4", PageSize::A4),
        ("A5", PageSize::A5),
        ("A6", PageSize::A6),
        ("Letter", PageSize::LETTER),
        ("Legal", PageSize::LEGAL),
        ("Tabloid", PageSize::TABLOID),
    ];
    match spec {
        PageSizeSpec::Points([width, height]) => Ok(PageSize::new(
            laminapdf_model::Pt::new(*width),
            laminapdf_model::Pt::new(*height),
        )),
        PageSizeSpec::Name(name) => NAMES.iter().find(|(n, _)| n == name).map(|(_, s)| *s).ok_or_else(|| {
            let known: Vec<String> = NAMES.iter().map(|(n, _)| (*n).to_owned()).collect();
            Error::InvalidDocument {
                pointer: pointer.to_owned(),
                reason: "wrong-type",
                did_you_mean: laminapdf_syntax::did_you_mean(name, &known),
            }
        }),
    }
}

fn region(spec: &RegionSpec) -> PageRegion {
    let mut region = PageRegion::new();
    if let Some(text) = &spec.start {
        region = region.with_start(text.clone());
    }
    if let Some(text) = &spec.center {
        region = region.with_center(text.clone());
    }
    if let Some(text) = &spec.end {
        region = region.with_end(text.clone());
    }
    region
}

fn template(ctx: &Ctx<'_>, spec: Option<&PageSpec>) -> Result<PageTemplate> {
    let Some(spec) = spec else {
        return Ok(PageTemplate::new(PageSize::A4));
    };
    let mut template = PageTemplate::new(page_size(&spec.size, "/page/size")?);
    if let Some(margins) = spec.margins {
        template = template.with_margins(match margins {
            MarginsSpec::All(m) => Margins::all(m),
            MarginsSpec::Sides([top, right, bottom, left]) => Margins::new(top, right, bottom, left),
        });
    }
    if spec.header.is_some() || spec.footer.is_some() {
        let style = ctx.style(spec.style.as_ref(), "/page/style")?;
        template = template.with_furniture(
            spec.header.as_ref().map(region),
            spec.footer.as_ref().map(region),
            style,
        );
    }
    Ok(template)
}

const fn align(spec: AlignSpec) -> Alignment {
    match spec {
        AlignSpec::Start => Alignment::Start,
        AlignSpec::End => Alignment::End,
        AlignSpec::Left => Alignment::Left,
        AlignSpec::Right => Alignment::Right,
        AlignSpec::Center => Alignment::Center,
    }
}

/// A paragraph from its text or spans in `style` (`pointer` is the paragraph's own).
fn paragraph(
    ctx: &Ctx<'_>,
    text: Option<&Text>,
    spans: Option<&Vec<Span>>,
    style: &TextStyle,
    pointer: &str,
) -> Result<Paragraph> {
    let mut paragraph = Paragraph::new(ParagraphStyle::new());
    if let Some(text) = text {
        paragraph = paragraph.text(text.as_string(), style);
    }
    for (i, span) in spans.into_iter().flatten().enumerate() {
        let at = format!("{pointer}/spans/{i}");
        let span_style = match &span.style {
            Some(_) => ctx.style(span.style.as_ref(), &format!("{at}/style"))?,
            None => style,
        };
        if span.line_break {
            paragraph = paragraph.line_break();
        }
        if let Some(note) = &span.note {
            paragraph = paragraph.note(paragraph_spec(ctx, note, &format!("{at}/note"))?);
        }
        let Some(text) = span.text.as_ref().map(Text::as_string) else {
            continue;
        };
        paragraph = match (&span.href, span.code) {
            (Some(href), _) => {
                let target = match href.strip_prefix('#') {
                    Some(name) => LinkTarget::Named(name.to_owned()),
                    None => LinkTarget::Uri(href.clone()),
                };
                paragraph.link(text, span_style, target)
            }
            (None, true) => paragraph.code(text, span_style),
            (None, false) => paragraph.text(text, span_style),
        };
    }
    Ok(paragraph)
}

/// A paragraph with its own style settings.
fn paragraph_spec(ctx: &Ctx<'_>, spec: &ParagraphSpec, pointer: &str) -> Result<Paragraph> {
    let style = ctx.style(spec.style.as_ref(), &format!("{pointer}/style"))?;
    let paragraph = paragraph(ctx, spec.text.as_ref(), spec.spans.as_ref(), style, pointer)?;
    let mut settings = ParagraphStyle::new();
    if let Some(a) = spec.align {
        settings = settings.with_align(align(a));
    }
    if let Some(factor) = spec.line_height {
        settings = settings.with_line_height(factor);
    }
    if spec.space_before.is_some() || spec.space_after.is_some() {
        settings = settings.with_space(spec.space_before.unwrap_or(0.0), spec.space_after.unwrap_or(0.0));
    }
    Ok(paragraph.with_style(
        settings
            .with_keep_with_next(spec.keep_with_next)
            .with_break_before(spec.break_before),
    ))
}

fn list(ctx: &Ctx<'_>, spec: &ListSpec, pointer: &str) -> Result<List> {
    let kind = match spec.kind {
        ListKindSpec::Bullet => ListKind::Bullet,
        ListKindSpec::Numbered => ListKind::Numbered,
    };
    let mut list = List::new(kind, ctx.style(spec.style.as_ref(), &format!("{pointer}/style"))?);
    if let Some(start) = spec.start {
        list = list.with_start(start);
    }
    if let Some(indent) = spec.indent {
        list = list.with_indent(indent);
    }
    for (i, item) in spec.items.iter().enumerate() {
        list = list.item(list_item(ctx, item, &format!("{pointer}/items/{i}"))?);
    }
    Ok(list)
}

fn list_item(ctx: &Ctx<'_>, spec: &ListItemSpec, pointer: &str) -> Result<ListItem> {
    let style = ctx.style(spec.style.as_ref(), &format!("{pointer}/style"))?;
    let mut item = ListItem::new(paragraph(ctx, spec.text.as_ref(), spec.spans.as_ref(), style, pointer)?);
    if let Some(marker) = &spec.marker {
        item = item.with_marker(marker.clone());
    }
    if let Some(nested) = &spec.list {
        item = item.with_nested(list(ctx, nested, &format!("{pointer}/list"))?);
    }
    Ok(item)
}

fn rule(spec: &RuleSpec, pointer: &str) -> Result<TableRule> {
    Ok(TableRule::new(
        spec.width,
        color(&spec.color, &format!("{pointer}/color"))?,
    ))
}

fn cell(ctx: &Ctx<'_>, spec: &CellSpec, pointer: &str) -> Result<TableCell> {
    let style = ctx.style(spec.style.as_ref(), &format!("{pointer}/style"))?;
    let mut cell = if spec.text.is_none() && spec.spans.is_none() {
        TableCell::empty()
    } else {
        TableCell::new(paragraph(ctx, spec.text.as_ref(), spec.spans.as_ref(), style, pointer)?)
    };
    if let Some(span) = spec.span {
        cell = cell.with_span(span);
    }
    if let Some(a) = spec.align {
        cell = cell.with_align(align(a));
    }
    Ok(cell)
}

fn rows(ctx: &Ctx<'_>, rows: &[super::RowSpec], pointer: &str) -> Result<Vec<TableRow>> {
    rows.iter()
        .enumerate()
        .map(|(r, row)| {
            row.cells
                .iter()
                .enumerate()
                .try_fold(TableRow::new(), |acc, (c, spec)| {
                    Ok(acc.cell(cell(ctx, spec, &format!("{pointer}/{r}/cells/{c}"))?))
                })
        })
        .collect()
}

fn table(ctx: &Ctx<'_>, spec: &super::TableSpec, pointer: &str) -> Result<Table> {
    let columns = spec.columns.iter().map(|c| match *c {
        ColumnSpec::Auto => ColumnWidth::Auto,
        ColumnSpec::Fixed(w) => ColumnWidth::Fixed(w),
        ColumnSpec::Fraction(f) => ColumnWidth::Fraction(f),
    });
    let mut table = Table::new(columns);
    if let Some(padding) = spec.padding {
        table = table.with_padding(padding);
    }
    if let Some(r) = &spec.frame {
        table = table.with_frame(rule(r, &format!("{pointer}/frame"))?);
    }
    if let Some(r) = &spec.header_rule {
        table = table.with_header_rule(rule(r, &format!("{pointer}/headerRule"))?);
    }
    if let Some(r) = &spec.row_rules {
        table = table.with_row_rules(rule(r, &format!("{pointer}/rowRules"))?);
    }
    if let Some(text) = &spec.header_background {
        table = table.with_header_background(color(text, &format!("{pointer}/headerBackground"))?);
    }
    for row in rows(ctx, &spec.header, &format!("{pointer}/header"))? {
        table = table.header(row);
    }
    for row in rows(ctx, &spec.rows, &format!("{pointer}/rows"))? {
        table = table.row(row);
    }
    for row in rows(ctx, &spec.footer, &format!("{pointer}/footer"))? {
        table = table.footer(row);
    }
    Ok(table)
}

fn contents(ctx: &Ctx<'_>, spec: &ContentsSpec, pointer: &str) -> Result<Contents> {
    let contents = Contents::new(ctx.style(spec.style.as_ref(), &format!("{pointer}/style"))?);
    Ok(match spec.levels {
        Some(levels) => contents.with_levels(levels),
        None => contents,
    })
}

fn blocks(ctx: &Ctx<'_>, blocks: &[Block], pointer: &str) -> Result<Vec<FlowBlock>> {
    blocks
        .iter()
        .enumerate()
        .map(|(i, b)| convert_block(ctx, b, &format!("{pointer}/blocks/{i}")))
        .collect()
}

fn convert_block(ctx: &Ctx<'_>, block: &Block, pointer: &str) -> Result<FlowBlock> {
    Ok(match block {
        Block::Paragraph(spec) => FlowBlock::Paragraph(paragraph_spec(ctx, spec, pointer)?),
        Block::Heading(spec) => {
            let level = spec.level.filter(|l| (1..=6).contains(l));
            let level = level.ok_or_else(|| invalid(&format!("{pointer}/level"), "wrong-type"))?;
            FlowBlock::Heading(level, paragraph_spec(ctx, spec, pointer)?)
        }
        Block::List(spec) => FlowBlock::List(list(ctx, spec, pointer)?),
        Block::Figure(spec) => {
            let image = ctx.resources.images.get(&spec.image).ok_or_else(|| {
                let known: Vec<String> = ctx.resources.images.keys().cloned().collect();
                Error::InvalidDocument {
                    pointer: format!("{pointer}/image"),
                    reason: "unknown-image",
                    did_you_mean: laminapdf_syntax::did_you_mean(&spec.image, &known),
                }
            })?;
            let figure = Figure::new(image.clone(), spec.alt.as_string());
            FlowBlock::Figure(match spec.width {
                Some(width) => figure.with_width(width),
                None => figure,
            })
        }
        Block::Spacer { height } => FlowBlock::Spacer(*height),
        Block::PageBreak => FlowBlock::PageBreak,
        Block::Keep { blocks: inner } => FlowBlock::Keep(blocks(ctx, inner, pointer)?),
        Block::Table(spec) => FlowBlock::Table(table(ctx, spec, pointer)?),
        Block::Contents(spec) => FlowBlock::Contents(contents(ctx, spec, pointer)?),
        Block::Quote { blocks: inner } => FlowBlock::Quote(blocks(ctx, inner, pointer)?),
        Block::CodeBlock { text, style } => FlowBlock::CodeBlock(
            text.as_string(),
            ctx.style(style.as_ref(), &format!("{pointer}/style"))?.clone(),
        ),
        Block::Rule => FlowBlock::Rule,
    })
}

impl Builder {
    /// Lays `document` out and adds its pages after any already added, with its fonts and images
    /// from `resources`; its title, author, subject and language become the document's metadata
    /// (each only when given, over what the builder already has). The caller passes its
    /// [profiles](FlowDocument::profiles) to the builder's options.
    ///
    /// # Errors
    /// `invalid-document` for a style, font or image the document names but doesn't have, a
    /// page size name it doesn't know, or a colour that isn't `#rrggbb`; as [`Builder::flow`].
    pub fn document(&mut self, document: &FlowDocument, resources: &Resources) -> Result<crate::flow::FlowReport> {
        let flow = document.to_flow(self, resources)?;
        let report = self.flow(&flow)?;
        if document.title.is_some() || document.author.is_some() || document.subject.is_some() {
            let mut metadata = self.metadata_so_far().cloned().unwrap_or_default();
            if let Some(title) = &document.title {
                metadata = metadata.with_title(title.clone());
            }
            if let Some(author) = &document.author {
                metadata = metadata.with_author(author.clone());
            }
            if let Some(subject) = &document.subject {
                metadata = metadata.with_subject(subject.clone());
            }
            self.metadata(metadata);
        }
        if let Some(lang) = &document.lang {
            self.lang(lang.clone());
        }
        Ok(report)
    }
}
