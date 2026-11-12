//! Tiles for the webview over the `lamina://` scheme (ADR 0008 §4): the URL the viewer fetches,
//! checked before anything reaches an engine; the engine's pixels encoded as lossless WebP (or
//! passed on raw, the fallback); and the HTTP answer.
//!
//! The URL is `lamina://localhost/tile/{doc}/{page}?s=…&r=…&a=…&x=…&y=…&w=…&h=…&p=…&t=…[&fmt=webp]`
//! (`http://lamina.localhost/tile/…` on Windows). The window comes from the webview that asked,
//! never from the URL, so a window can only fetch tiles of its own documents. `t` is the
//! viewer's ticket for the tile: `cancel_tiles` names it to drop a tile the viewer no longer
//! wants.

use laminapdf_desktop_protocol::{DocId, Priority, TileAppearance, ViewRotation};
use tauri::http::{Response, StatusCode, header};

use crate::DesktopError;

/// The largest tile side the URL may ask for, in pixels: the engine's own limit
/// (`laminapdf_render::RenderLimits::max_render_side`'s default, copied because the main process
/// links no render crate). Not the viewer's 512 px tile: a page's low-resolution backdrop is one
/// request of up to 8 MiB pixels, as tall as the page needs.
const MAX_SIDE: u32 = 16_384;

/// The most pixels one tile may hold: `RenderLimits::max_render_pixels`' default (4096 × 4096).
const MAX_PIXELS: u64 = 16_777_216;

/// The largest scale the URL may ask for: the viewer's 800 % zoom at a device pixel ratio of 4,
/// with room to spare.
const MAX_SCALE: f32 = 64.0;

/// How the main process answers with the engine's pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Encoding {
    /// Lossless WebP, decoded by the webview's `createImageBitmap(blob)` (the default transport).
    Webp,
    /// Straight-alpha RGBA as the engine drew it (the fallback, for diagnosis).
    Raw,
}

/// One tile as the webview asked for it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TileQuery {
    pub doc: DocId,
    pub page: u32,
    pub scale: f32,
    pub rotation: ViewRotation,
    pub appearance: TileAppearance,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub priority: Priority,
    /// The viewer's ticket for this tile, which `cancel_tiles` names.
    pub ticket: u64,
    pub encoding: Encoding,
}

/// A tile the engine drew: straight RGBA, `w × h`.
#[derive(Debug)]
pub(crate) struct Tile {
    pub w: u32,
    pub h: u32,
    pub pixels: Vec<u8>,
}

/// Why a tile URL was refused. Never echoes the URL: a webview's mistake shouldn't fill the log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum UrlError {
    #[error("not a tile address")]
    NotATile,
    #[error("the tile address lacks `{0}`")]
    Missing(&'static str),
    #[error("the tile address's `{0}` is out of range or malformed")]
    Bad(&'static str),
    #[error("the tile address has a parameter this app doesn't know")]
    Unknown,
}

/// Parses a tile URL's path (`/tile/{doc}/{page}`) and query.
pub(crate) fn parse(path: &str, query: Option<&str>) -> Result<TileQuery, UrlError> {
    let mut parts = path.strip_prefix("/tile/").ok_or(UrlError::NotATile)?.split('/');
    let doc = parts.next().ok_or(UrlError::NotATile)?;
    let page = parts.next().ok_or(UrlError::NotATile)?;
    if parts.next().is_some() {
        return Err(UrlError::NotATile);
    }
    let mut params = Params::default();
    for pair in query.unwrap_or("").split('&').filter(|pair| !pair.is_empty()) {
        let (key, value) = pair.split_once('=').ok_or(UrlError::Unknown)?;
        params.set(key, value)?;
    }
    let scale = number::<f32>(params.s, "s")?;
    if !(scale.is_finite() && scale > 0.0 && scale <= MAX_SCALE) {
        return Err(UrlError::Bad("s"));
    }
    let side = |value, name| match number::<u32>(value, name)? {
        side @ 1..=MAX_SIDE => Ok(side),
        _ => Err(UrlError::Bad(name)),
    };
    let (w, h) = (side(params.w, "w")?, side(params.h, "h")?);
    if u64::from(w) * u64::from(h) > MAX_PIXELS {
        return Err(UrlError::Bad("h"));
    }
    Ok(TileQuery {
        doc: DocId(doc.parse().map_err(|_| UrlError::Bad("doc"))?),
        page: page.parse().map_err(|_| UrlError::Bad("page"))?,
        scale,
        rotation: rotation(params.r)?,
        appearance: appearance(params.a)?,
        x: number(params.x, "x")?,
        y: number(params.y, "y")?,
        w,
        h,
        priority: match params.p {
            Some("v") | None => Priority::Visible,
            Some("p") => Priority::Prefetch,
            Some(_) => return Err(UrlError::Bad("p")),
        },
        ticket: number(params.t, "t")?,
        encoding: match params.fmt {
            None => Encoding::Raw,
            Some("webp") => Encoding::Webp,
            Some(_) => return Err(UrlError::Bad("fmt")),
        },
    })
}

/// The query's values, each named once.
#[derive(Default)]
struct Params<'a> {
    s: Option<&'a str>,
    r: Option<&'a str>,
    a: Option<&'a str>,
    x: Option<&'a str>,
    y: Option<&'a str>,
    w: Option<&'a str>,
    h: Option<&'a str>,
    p: Option<&'a str>,
    t: Option<&'a str>,
    fmt: Option<&'a str>,
}

impl<'a> Params<'a> {
    fn set(&mut self, key: &str, value: &'a str) -> Result<(), UrlError> {
        let slot = match key {
            "s" => &mut self.s,
            "r" => &mut self.r,
            "a" => &mut self.a,
            "x" => &mut self.x,
            "y" => &mut self.y,
            "w" => &mut self.w,
            "h" => &mut self.h,
            "p" => &mut self.p,
            "t" => &mut self.t,
            "fmt" => &mut self.fmt,
            _ => return Err(UrlError::Unknown),
        };
        if slot.replace(value).is_some() {
            return Err(UrlError::Unknown);
        }
        Ok(())
    }
}

fn number<T: std::str::FromStr>(value: Option<&str>, name: &'static str) -> Result<T, UrlError> {
    value
        .ok_or(UrlError::Missing(name))?
        .parse()
        .map_err(|_| UrlError::Bad(name))
}

fn rotation(value: Option<&str>) -> Result<ViewRotation, UrlError> {
    match value.unwrap_or("0") {
        "0" => Ok(ViewRotation::None),
        "90" => Ok(ViewRotation::Quarter),
        "180" => Ok(ViewRotation::Half),
        "270" => Ok(ViewRotation::ThreeQuarters),
        _ => Err(UrlError::Bad("r")),
    }
}

/// `original` (or absent), `sepia`, `dark`, or `hc-RRGGBBAA-RRGGBBAA` (ink, then paper).
fn appearance(value: Option<&str>) -> Result<TileAppearance, UrlError> {
    match value.unwrap_or("original") {
        "original" => Ok(TileAppearance::Original),
        "sepia" => Ok(TileAppearance::Sepia),
        "dark" => Ok(TileAppearance::Dark),
        other => {
            let colours = other.strip_prefix("hc-").ok_or(UrlError::Bad("a"))?;
            let (ink, paper) = colours.split_once('-').ok_or(UrlError::Bad("a"))?;
            Ok(TileAppearance::HighContrast {
                ink: rgba(ink)?,
                paper: rgba(paper)?,
            })
        }
    }
}

/// Eight hex digits as straight RGBA.
fn rgba(hex: &str) -> Result<[u8; 4], UrlError> {
    // Digits only: `from_str_radix` alone would also take a leading `+`.
    if hex.len() != 8 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(UrlError::Bad("a"));
    }
    let value = u32::from_str_radix(hex, 16).map_err(|_| UrlError::Bad("a"))?;
    Ok(value.to_be_bytes())
}

/// Encodes `tile` as the query asked; WebP only fails on a size the engine never sends.
pub(crate) fn encode(tile: &Tile, encoding: Encoding) -> Result<Vec<u8>, DesktopError> {
    match encoding {
        Encoding::Raw => Ok(tile.pixels.clone()),
        Encoding::Webp => {
            let mut out = Vec::with_capacity(tile.pixels.len() / 8);
            image_webp::WebPEncoder::new(&mut out)
                .encode(&tile.pixels, tile.w, tile.h, image_webp::ColorType::Rgba8)
                .map_err(|error| DesktopError::Internal(format!("could not encode a tile: {error}")))?;
            Ok(out)
        }
    }
}

/// The scheme's answer: the tile, or the failure as `{ "kind", "message" }` with a status the
/// viewer's client maps back to the kind.
pub(crate) fn respond(result: Result<(Tile, Encoding), DesktopError>) -> Response<Vec<u8>> {
    let builder = Response::builder()
        // The scheme's origin isn't the page's (`lamina://localhost` against `tauri://localhost`).
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(header::ACCESS_CONTROL_EXPOSE_HEADERS, "x-tile-width, x-tile-height")
        // The viewer keeps its own cache of bitmaps.
        .header(header::CACHE_CONTROL, "no-store");
    let made = match result.and_then(|(tile, encoding)| Ok((encode(&tile, encoding)?, tile, encoding))) {
        Ok((body, tile, encoding)) => builder
            .status(StatusCode::OK)
            .header(
                header::CONTENT_TYPE,
                match encoding {
                    Encoding::Webp => "image/webp",
                    Encoding::Raw => "application/octet-stream",
                },
            )
            .header("x-tile-width", tile.w)
            .header("x-tile-height", tile.h)
            .body(body),
        Err(error) => builder
            .status(status_of(&error))
            .header(header::CONTENT_TYPE, "application/json")
            .body(serde_json::to_vec(&error).unwrap_or_default()),
    };
    made.unwrap_or_else(|_| {
        let mut fallback = Response::new(Vec::new());
        *fallback.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        fallback
    })
}

/// The HTTP status for a failed tile: the viewer's client reads the body's `kind`, the status
/// keeps the network panel and logs readable.
pub(crate) fn status_of(error: &DesktopError) -> StatusCode {
    match error.kind() {
        "bad-tile-url" => StatusCode::BAD_REQUEST,
        "unknown-document" | "page-out-of-range" => StatusCode::NOT_FOUND,
        // A tile the viewer dropped, or one of a document swapped since: stale, not missing.
        "cancelled" => StatusCode::CONFLICT,
        "needs-password" => StatusCode::LOCKED,
        "engine-restarted" | "engine-timeout" | "engine-failed" | "engine-start" => StatusCode::SERVICE_UNAVAILABLE,
        "too-large" | "limit" => StatusCode::PAYLOAD_TOO_LARGE,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

impl From<UrlError> for DesktopError {
    fn from(error: UrlError) -> Self {
        Self::Engine {
            kind: "bad-tile-url",
            message: error.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const QUERY: &str = "s=1.5&r=90&a=dark&x=512&y=0&w=512&h=300&p=p&t=17&fmt=webp";

    fn parsed(path: &str, query: &str) -> Result<TileQuery, UrlError> {
        parse(path, Some(query))
    }

    #[test]
    fn rgba_should_take_eight_hex_digits_and_nothing_else() {
        assert_eq!(rgba("0a0b0cff"), Ok([10, 11, 12, 255]));
        assert_eq!(rgba("+000000f"), Err(UrlError::Bad("a")));
        assert_eq!(rgba("0a0b0c"), Err(UrlError::Bad("a")));
    }

    #[test]
    fn parse_should_read_every_field_of_a_tile_url() {
        let query = parsed("/tile/3/12", QUERY);
        assert_eq!(
            query,
            Ok(TileQuery {
                doc: DocId(3),
                page: 12,
                scale: 1.5,
                rotation: ViewRotation::Quarter,
                appearance: TileAppearance::Dark,
                x: 512,
                y: 0,
                w: 512,
                h: 300,
                priority: Priority::Prefetch,
                ticket: 17,
                encoding: Encoding::Webp,
            })
        );
    }

    #[test]
    fn parse_should_default_to_upright_original_visible_raw() {
        let query = parsed("/tile/1/0", "s=1&x=0&y=0&w=10&h=10&t=1");
        assert_eq!(
            query.map(|q| (q.rotation, q.appearance, q.priority, q.encoding)),
            Ok((
                ViewRotation::None,
                TileAppearance::Original,
                Priority::Visible,
                Encoding::Raw
            ))
        );
    }

    #[test]
    fn parse_should_read_a_high_contrast_appearance_as_ink_then_paper() {
        let query = parsed("/tile/1/0", "s=1&x=0&y=0&w=10&h=10&t=1&a=hc-000000ff-ffffffff");
        assert_eq!(
            query.map(|q| q.appearance),
            Ok(TileAppearance::HighContrast {
                ink: [0, 0, 0, 255],
                paper: [255, 255, 255, 255]
            })
        );
    }

    #[test]
    fn parse_should_refuse_a_path_that_isnt_a_tile() {
        for path in [
            "/",
            "/tile",
            "/tile/1",
            "/tiles/1/2",
            "/tile/1/2/3",
            "/tile/a/2",
            "/tile/1/-2",
        ] {
            assert!(parsed(path, "s=1&x=0&y=0&w=1&h=1&t=1").is_err(), "{path}");
        }
    }

    #[test]
    fn parse_should_refuse_a_missing_or_malformed_value() {
        let cases = [
            ("x=0&y=0&w=1&h=1&t=1", UrlError::Missing("s")),
            ("s=1&x=0&y=0&w=1&h=1", UrlError::Missing("t")),
            ("s=0&x=0&y=0&w=1&h=1&t=1", UrlError::Bad("s")),
            ("s=NaN&x=0&y=0&w=1&h=1&t=1", UrlError::Bad("s")),
            ("s=100&x=0&y=0&w=1&h=1&t=1", UrlError::Bad("s")),
            ("s=1&x=0&y=0&w=0&h=1&t=1", UrlError::Bad("w")),
            ("s=1&x=0&y=0&w=1&h=16385&t=1", UrlError::Bad("h")),
            ("s=1&x=-1&y=0&w=1&h=1&t=1", UrlError::Bad("x")),
            ("s=1&x=0&y=0&w=1&h=1&t=1&r=45", UrlError::Bad("r")),
            ("s=1&x=0&y=0&w=1&h=1&t=1&a=hc-000000-ffffff", UrlError::Bad("a")),
            ("s=1&x=0&y=0&w=1&h=1&t=1&fmt=png", UrlError::Bad("fmt")),
            ("s=1&x=0&y=0&w=1&h=1&t=1&p=now", UrlError::Bad("p")),
        ];
        for (query, expected) in cases {
            assert_eq!(parsed("/tile/1/0", query), Err(expected), "{query}");
        }
    }

    #[test]
    fn parse_should_take_a_backdrop_taller_than_the_viewer_tiles() {
        // The viewer's whole-page backdrop at 250 % on a Retina screen (D2e's zoom scenario).
        let query = parsed("/tile/1/0", "s=5.8&x=0&y=0&w=2431&h=3450&t=1");
        assert_eq!(query.map(|q| (q.w, q.h)), Ok((2431, 3450)));
    }

    #[test]
    fn parse_should_refuse_a_tile_past_the_engine_limits() {
        assert_eq!(
            parsed("/tile/1/0", "s=1&x=0&y=0&w=16385&h=1&t=1"),
            Err(UrlError::Bad("w"))
        );
        assert_eq!(
            parsed("/tile/1/0", "s=1&x=0&y=0&w=4097&h=4096&t=1"),
            Err(UrlError::Bad("h"))
        );
    }

    #[test]
    fn parse_should_refuse_an_unknown_or_repeated_parameter() {
        for query in ["s=1&x=0&y=0&w=1&h=1&t=1&doc=2", "s=1&s=2&x=0&y=0&w=1&h=1&t=1", "s=1&x"] {
            assert_eq!(parsed("/tile/1/0", query), Err(UrlError::Unknown), "{query}");
        }
    }

    #[test]
    fn encode_should_round_trip_a_tile_through_webp_without_loss() -> Result<(), Box<dyn std::error::Error>> {
        let pixels: Vec<u8> = (0..16_u32 * 8 * 4).map(|i| (i * 37 % 251) as u8).collect();
        let tile = Tile { w: 16, h: 8, pixels };
        let webp = encode(&tile, Encoding::Webp)?;
        let mut decoder = image_webp::WebPDecoder::new(std::io::Cursor::new(webp))?;
        let mut back = vec![0; tile.pixels.len()];
        decoder.read_image(&mut back)?;
        assert_eq!(back, tile.pixels);
        Ok(())
    }

    #[test]
    fn respond_should_answer_a_tile_with_its_size_and_cors() {
        let tile = Tile {
            w: 2,
            h: 1,
            pixels: vec![1, 2, 3, 4, 5, 6, 7, 8],
        };
        let response = respond(Ok((tile, Encoding::Raw)));
        let header = |name: &str| response.headers().get(name).and_then(|v| v.to_str().ok());
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            (
                header("x-tile-width"),
                header("x-tile-height"),
                header("access-control-allow-origin")
            ),
            (Some("2"), Some("1"), Some("*"))
        );
        assert_eq!(response.body(), &vec![1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn respond_should_answer_a_failure_with_its_kind_and_status() {
        let response = respond(Err(DesktopError::UnknownDocument));
        let body: serde_json::Value = serde_json::from_slice(response.body()).unwrap_or_default();
        assert_eq!(
            (response.status(), &body["kind"]),
            (StatusCode::NOT_FOUND, &serde_json::json!("unknown-document"))
        );
    }

    #[test]
    fn status_of_should_tell_a_stale_tile_from_a_missing_one() {
        let engine = |kind| DesktopError::Engine {
            kind,
            message: String::new(),
        };
        assert_eq!(status_of(&engine("cancelled")), StatusCode::CONFLICT);
        assert_eq!(status_of(&engine("page-out-of-range")), StatusCode::NOT_FOUND);
        assert_eq!(status_of(&engine("engine-restarted")), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(status_of(&UrlError::Unknown.into()), StatusCode::BAD_REQUEST);
    }
}
