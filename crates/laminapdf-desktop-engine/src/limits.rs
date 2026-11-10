//! The protocol's [`Limits`] as the parser's [`laminapdf::Limits`] and the renderer's
//! [`RenderLimits`]: each field the main process left `None` keeps its default.

use laminapdf_desktop_protocol::Limits;
use laminapdf_render::RenderLimits;

/// Applies each set field with the parser's `with_*` setter, converting it to the setter's type
/// (a value too large for it saturates).
macro_rules! parse_limits {
    ($from:expr, $( $field:ident: $ty:ty => $with:ident; )*) => {{
        let mut limits = laminapdf::Limits::new();
        $(
            if let Some(value) = $from.$field {
                limits = limits.$with(<$ty>::try_from(value).unwrap_or(<$ty>::MAX));
            }
        )*
        limits
    }};
}

/// The parser's limits: ADR 0003's defaults with the main process's overrides.
pub(crate) fn parse(limits: &Limits) -> laminapdf::Limits {
    parse_limits! { limits,
        max_nesting_depth: usize => with_max_nesting_depth;
        max_stream_decoded: usize => with_max_stream_decoded;
        max_decoded_total: usize => with_max_decoded_total;
        max_filter_chain: usize => with_max_filter_chain;
        max_resolve_depth: usize => with_max_resolve_depth;
        max_page_tree_depth: usize => with_max_page_tree_depth;
        max_tree_depth: usize => with_max_tree_depth;
        max_string_length: usize => with_max_string_length;
        max_xref_sections: usize => with_max_xref_sections;
        max_object_number: u32 => with_max_object_number;
        max_warnings: usize => with_max_warnings;
        max_content_operators: u64 => with_max_content_operators;
        max_xobject_depth: usize => with_max_xobject_depth;
        max_image_pixels: u64 => with_max_image_pixels;
        max_read_window: usize => with_max_read_window;
    }
}

/// The renderer's limits: ADR 0004's defaults with the main process's overrides.
pub(crate) fn render(limits: &Limits) -> RenderLimits {
    let mut out = RenderLimits::default();
    if let Some(value) = limits.max_render_pixels {
        out.max_render_pixels = value;
    }
    if let Some(value) = limits.max_render_side {
        out.max_render_side = u32::try_from(value).unwrap_or(u32::MAX);
    }
    if let Some(value) = limits.max_input_bytes {
        out.max_input_bytes = value;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unset_limits_should_keep_the_defaults() {
        assert_eq!(parse(&Limits::default()), laminapdf::Limits::new());
    }

    #[test]
    fn a_set_parse_limit_should_reach_the_parser() {
        let limits = Limits {
            max_nesting_depth: Some(7),
            ..Limits::default()
        };
        assert_eq!(parse(&limits).max_nesting_depth(), 7);
    }

    #[test]
    fn a_set_render_limit_should_reach_the_renderer() {
        let limits = Limits {
            max_render_side: Some(1024),
            ..Limits::default()
        };
        assert_eq!(render(&limits).max_render_side, 1024);
    }
}
