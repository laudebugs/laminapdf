//! Framing: how one message and its bytes travel over a pipe.
//!
//! ```text
//! u32 LE  length of everything after this field
//! u16 LE  protocol version
//! u32 LE  header length
//! header  one message, postcard-encoded
//! payload raw bytes (a file range, a tile's pixels), possibly empty
//! ```

use std::io::{self, Read, Write};

use serde::Serialize;
use serde::de::DeserializeOwned;
use zeroize::Zeroize;

use crate::PROTOCOL_VERSION;

/// The largest frame either side sends or accepts: 64 MiB, room for a 4096 × 4096 RGBA tile.
/// [`read_frame`] rejects a longer length before allocating anything.
pub const MAX_FRAME: u32 = 64 * 1024 * 1024;

/// The bytes after the length prefix that every frame has: the version and the header length.
const FIXED: u32 = 2 + 4;

/// Why a frame couldn't be read or written.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ProtocolError {
    /// The pipe closed cleanly between frames: the other side exited or shut down.
    #[error("the pipe closed")]
    Closed,
    /// The pipe closed in the middle of a frame.
    #[error("the frame was cut short")]
    Truncated,
    /// The frame is longer than the limit.
    #[error("a frame of {len} bytes is over the {max}-byte limit")]
    TooLarge {
        /// The frame's declared (or, when writing, actual) length.
        len: u64,
        /// The limit.
        max: u32,
    },
    /// The frame came from another version of the protocol.
    #[error("protocol version {found}, expected {PROTOCOL_VERSION}")]
    Version {
        /// The version the frame carries.
        found: u16,
    },
    /// The frame's lengths don't add up.
    #[error("malformed frame: {0}")]
    Malformed(&'static str),
    /// The header isn't a message of the expected type.
    #[error("the message could not be decoded: {0}")]
    Decode(postcard::Error),
    /// The message could not be encoded.
    #[error("the message could not be encoded: {0}")]
    Encode(postcard::Error),
    /// The pipe failed.
    #[error("the pipe failed: {0}")]
    Io(#[from] io::Error),
}

/// Writes `msg` and `payload` as one frame to `w`, then flushes it.
///
/// # Errors
/// [`ProtocolError::TooLarge`] when the frame would pass [`MAX_FRAME`], `Encode` and `Io`.
pub fn write_frame<W: Write, M: Serialize>(w: &mut W, msg: &M, payload: &[u8]) -> Result<(), ProtocolError> {
    // The header may hold a password: wipe it however this function returns.
    let header = zeroize::Zeroizing::new(postcard::to_allocvec(msg).map_err(ProtocolError::Encode)?);
    let len = u64::from(FIXED) + header.len() as u64 + payload.len() as u64;
    let (Ok(len32), Ok(header_len)) = (u32::try_from(len), u32::try_from(header.len())) else {
        return Err(ProtocolError::TooLarge { len, max: MAX_FRAME });
    };
    if len32 > MAX_FRAME {
        return Err(ProtocolError::TooLarge { len, max: MAX_FRAME });
    }
    w.write_all(&len32.to_le_bytes())?;
    w.write_all(&PROTOCOL_VERSION.to_le_bytes())?;
    w.write_all(&header_len.to_le_bytes())?;
    w.write_all(&header)?;
    w.write_all(payload)?;
    w.flush()?;
    Ok(())
}

/// Reads one frame from `r`: its message and its payload. Frames longer than `max` (at most
/// [`MAX_FRAME`]) are rejected before anything is allocated.
///
/// # Errors
/// [`ProtocolError::Closed`] at a clean end of input, `Truncated` when it ends mid-frame,
/// `TooLarge`, `Version`, `Malformed`, `Decode` and `Io`.
pub fn read_frame<R: Read, M: DeserializeOwned>(r: &mut R, max: u32) -> Result<(M, Vec<u8>), ProtocolError> {
    let mut prefix = [0_u8; 4];
    match read_full(r, &mut prefix)? {
        0 => return Err(ProtocolError::Closed),
        4 => {}
        _ => return Err(ProtocolError::Truncated),
    }
    let len = u32::from_le_bytes(prefix);
    let max = max.min(MAX_FRAME);
    if len > max {
        return Err(ProtocolError::TooLarge {
            len: u64::from(len),
            max,
        });
    }
    if len < FIXED {
        return Err(ProtocolError::Malformed("shorter than its fixed fields"));
    }
    let mut fixed = [0_u8; 6];
    read_exact(r, &mut fixed)?;
    let [v0, v1, h0, h1, h2, h3] = fixed;
    let found = u16::from_le_bytes([v0, v1]);
    if found != PROTOCOL_VERSION {
        return Err(ProtocolError::Version { found });
    }
    let header_len = u32::from_le_bytes([h0, h1, h2, h3]);
    let rest = len - FIXED;
    if header_len > rest {
        return Err(ProtocolError::Malformed("the header is longer than the frame"));
    }
    let mut header = zeroize::Zeroizing::new(vec![0_u8; header_len as usize]);
    read_exact(r, &mut header)?;
    let mut payload = vec![0_u8; (rest - header_len) as usize];
    read_exact(r, &mut payload)?;
    let msg = postcard::from_bytes(&header).map_err(ProtocolError::Decode)?;
    header.zeroize();
    Ok((msg, payload))
}

/// Fills `buf` unless the input ends first; returns the bytes read.
fn read_full<R: Read>(r: &mut R, buf: &mut [u8]) -> Result<usize, ProtocolError> {
    let mut filled = 0;
    while let Some(rest) = buf.get_mut(filled..).filter(|rest| !rest.is_empty()) {
        match r.read(rest) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(ProtocolError::Io(e)),
        }
    }
    Ok(filled)
}

/// Fills `buf`; an early end of input is [`ProtocolError::Truncated`].
fn read_exact<R: Read>(r: &mut R, buf: &mut [u8]) -> Result<(), ProtocolError> {
    if read_full(r, buf)? == buf.len() {
        Ok(())
    } else {
        Err(ProtocolError::Truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DocId, ErrorKind, FromEngine, Limits, PageSize, Password, Priority, ReqId, TileFormat, TileRequest, ToEngine,
    };

    fn encode<M: Serialize>(msg: &M, payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        assert!(write_frame(&mut out, msg, payload).is_ok());
        out
    }

    fn round_trip<M: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug>(msg: &M, payload: &[u8]) {
        let bytes = encode(msg, payload);
        let decoded: Result<(M, Vec<u8>), _> = read_frame(&mut bytes.as_slice(), MAX_FRAME);
        match decoded {
            Ok((back, back_payload)) => {
                assert_eq!((&back, back_payload.as_slice()), (msg, payload));
            }
            Err(e) => panic!("{msg:?} did not round-trip: {e}"),
        }
    }

    fn to_engine_messages() -> Vec<ToEngine> {
        let req = ReqId(7);
        let doc = DocId(3);
        vec![
            ToEngine::Hello {
                version: PROTOCOL_VERSION,
                limits: Box::new(Limits {
                    max_render_side: Some(16_384),
                    ..Limits::default()
                }),
            },
            ToEngine::OpenDocument { req, doc, len: 1 << 40 },
            ToEngine::UnlockDocument {
                req,
                doc,
                password: Password::new("hunter2".into()),
            },
            ToEngine::RangeData { req, offset: 99 },
            ToEngine::RangeFailed {
                req,
                kind: ErrorKind::Io,
                message: "truncated".into(),
            },
            ToEngine::GetPageInfo { req, doc },
            ToEngine::RenderTile(TileRequest {
                req,
                doc,
                page: 2,
                scale_milli: 2000,
                x: 512,
                y: 0,
                w: 512,
                h: 256,
                format: TileFormat::Rgba8,
                priority: Priority::Visible,
            }),
            ToEngine::Cancel { req },
            ToEngine::CloseDocument { doc },
            ToEngine::Shutdown,
        ]
    }

    fn from_engine_messages() -> Vec<FromEngine> {
        let req = ReqId(9);
        let doc = DocId(1);
        vec![
            FromEngine::Ready {
                version: PROTOCOL_VERSION,
                pid: 4242,
            },
            FromEngine::ReadRange {
                req,
                doc,
                offset: 1 << 33,
                len: 262_144,
            },
            FromEngine::Opened {
                req,
                doc,
                page_count: 0,
                encrypted: true,
                needs_password: true,
                repaired: false,
            },
            FromEngine::NeedsPassword { req, doc },
            FromEngine::Unlocked {
                req,
                doc,
                page_count: 12,
                repaired: true,
            },
            FromEngine::PageInfo {
                req,
                doc,
                sizes: vec![PageSize {
                    width: 612.0,
                    height: 792.0,
                    rotate: 90,
                }],
            },
            FromEngine::Tile {
                req,
                w: 2,
                h: 1,
                format: TileFormat::Rgba8,
            },
            FromEngine::Error {
                req: Some(req),
                kind: ErrorKind::WrongPassword,
                message: "wrong password".into(),
            },
            FromEngine::Error {
                req: None,
                kind: ErrorKind::Protocol,
                message: "bad frame".into(),
            },
            FromEngine::Heartbeat,
        ]
    }

    #[test]
    fn every_message_to_the_engine_should_round_trip() {
        for msg in to_engine_messages() {
            round_trip(&msg, b"");
        }
    }

    #[test]
    fn every_message_from_the_engine_should_round_trip() {
        for msg in from_engine_messages() {
            round_trip(&msg, b"");
        }
    }

    #[test]
    fn a_payload_should_round_trip_beside_its_message() {
        round_trip(
            &FromEngine::Tile {
                req: ReqId(1),
                w: 2,
                h: 1,
                format: TileFormat::Rgba8,
            },
            &[1, 2, 3, 4, 5, 6, 7, 8],
        );
    }

    #[test]
    fn frames_should_read_back_to_back_from_one_stream() {
        let mut stream = encode(&ToEngine::Shutdown, b"");
        stream.extend(encode(&ToEngine::Cancel { req: ReqId(5) }, b"xy"));
        let mut r = stream.as_slice();
        let first: Result<(ToEngine, Vec<u8>), _> = read_frame(&mut r, MAX_FRAME);
        let second: Result<(ToEngine, Vec<u8>), _> = read_frame(&mut r, MAX_FRAME);
        let third: Result<(ToEngine, Vec<u8>), _> = read_frame(&mut r, MAX_FRAME);
        assert!(matches!(
            (first, second, third),
            (
                Ok((ToEngine::Shutdown, _)),
                Ok((ToEngine::Cancel { req: ReqId(5) }, _)),
                Err(ProtocolError::Closed)
            )
        ));
    }

    #[test]
    fn an_oversized_length_should_be_rejected_before_reading_on() {
        // Only the prefix exists: a reader that allocated or read on would fail differently.
        let bytes = (MAX_FRAME + 1).to_le_bytes();
        let result: Result<(ToEngine, Vec<u8>), _> = read_frame(&mut bytes.as_slice(), MAX_FRAME);
        assert!(matches!(result, Err(ProtocolError::TooLarge { .. })));
    }

    #[test]
    fn a_length_over_the_callers_limit_should_be_rejected() {
        let bytes = encode(&ToEngine::Shutdown, &[0; 64]);
        let result: Result<(ToEngine, Vec<u8>), _> = read_frame(&mut bytes.as_slice(), 16);
        assert!(matches!(result, Err(ProtocolError::TooLarge { .. })));
    }

    #[test]
    fn writing_an_oversized_frame_should_fail() {
        let payload = vec![0_u8; MAX_FRAME as usize];
        let result = write_frame(&mut Vec::new(), &ToEngine::Shutdown, &payload);
        assert!(matches!(result, Err(ProtocolError::TooLarge { .. })));
    }

    #[test]
    fn another_protocol_version_should_be_rejected() {
        let mut bytes = encode(&ToEngine::Shutdown, b"");
        if let Some(version) = bytes.get_mut(4..6) {
            version.copy_from_slice(&(PROTOCOL_VERSION + 1).to_le_bytes());
        }
        let result: Result<(ToEngine, Vec<u8>), _> = read_frame(&mut bytes.as_slice(), MAX_FRAME);
        assert!(matches!(result, Err(ProtocolError::Version { found }) if found == PROTOCOL_VERSION + 1));
    }

    #[test]
    fn a_truncated_payload_should_be_rejected() {
        let bytes = encode(
            &ToEngine::RangeData {
                req: ReqId(1),
                offset: 0,
            },
            &[7; 100],
        );
        let cut = bytes.get(..bytes.len() - 1).unwrap_or_default();
        let result: Result<(ToEngine, Vec<u8>), _> = read_frame(&mut &cut[..], MAX_FRAME);
        assert!(matches!(result, Err(ProtocolError::Truncated)));
    }

    #[test]
    fn a_truncated_prefix_should_be_rejected() {
        let result: Result<(ToEngine, Vec<u8>), _> = read_frame(&mut &[1_u8, 0][..], MAX_FRAME);
        assert!(matches!(result, Err(ProtocolError::Truncated)));
    }

    #[test]
    fn a_header_longer_than_its_frame_should_be_rejected() {
        let mut bytes = encode(&ToEngine::Shutdown, b"");
        if let Some(header_len) = bytes.get_mut(6..10) {
            header_len.copy_from_slice(&1000_u32.to_le_bytes());
        }
        let result: Result<(ToEngine, Vec<u8>), _> = read_frame(&mut bytes.as_slice(), MAX_FRAME);
        assert!(matches!(result, Err(ProtocolError::Malformed(_))));
    }

    #[test]
    fn a_frame_shorter_than_its_fixed_fields_should_be_rejected() {
        let bytes = 3_u32.to_le_bytes();
        let result: Result<(ToEngine, Vec<u8>), _> = read_frame(&mut bytes.as_slice(), MAX_FRAME);
        assert!(matches!(result, Err(ProtocolError::Malformed(_))));
    }

    #[test]
    fn a_header_of_the_other_direction_should_fail_to_decode() {
        let bytes = encode(&FromEngine::Heartbeat, b"");
        let result: Result<(ToEngine, Vec<u8>), _> = read_frame(&mut bytes.as_slice(), MAX_FRAME);
        assert!(result.is_err());
    }

    #[test]
    fn debug_of_an_unlock_message_should_not_show_the_password() {
        let msg = ToEngine::UnlockDocument {
            req: ReqId(1),
            doc: DocId(1),
            password: Password::new("correct horse".into()),
        };
        assert!(!format!("{msg:?}").contains("correct horse"));
    }

    #[test]
    fn a_password_should_survive_the_frame() {
        let bytes = encode(
            &ToEngine::UnlockDocument {
                req: ReqId(1),
                doc: DocId(1),
                password: Password::new("pässwörd".into()),
            },
            b"",
        );
        let result: Result<(ToEngine, Vec<u8>), _> = read_frame(&mut bytes.as_slice(), MAX_FRAME);
        assert!(
            matches!(result, Ok((ToEngine::UnlockDocument { password, .. }, _)) if password.expose() == "pässwörd")
        );
    }
}
