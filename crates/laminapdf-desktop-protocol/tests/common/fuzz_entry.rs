//! The `engine_protocol_decode` fuzz target's entry point (ADR 0018 tier 9), shared with this
//! crate's `tests/regressions.rs`: the main process reads frames an engine wrote, and an engine
//! that parsed a hostile PDF may write anything.

use laminapdf_desktop_protocol::{FromEngine, MAX_FRAME, ProtocolError, ToEngine, read_frame, write_frame};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// Reads `data` as a stream of frames, both as the main process reads them ([`FromEngine`]) and
/// as the engine does ([`ToEngine`]). Every frame either decodes or fails with a
/// [`ProtocolError`]; a decoded one must encode again. Never a panic.
pub fn engine_protocol_decode(data: &[u8]) {
    frames::<FromEngine>(data);
    frames::<ToEngine>(data);
}

fn frames<M: Serialize + DeserializeOwned>(data: &[u8]) {
    let mut input = data;
    // A frame is at least 10 bytes, so this bounds the loop by the input.
    for _ in 0..=data.len() / 10 {
        match read_frame::<_, M>(&mut input, MAX_FRAME) {
            Ok((msg, payload)) => {
                assert!(payload.len() <= MAX_FRAME as usize, "a payload over the frame limit");
                let mut again = Vec::new();
                match write_frame(&mut again, &msg, &payload) {
                    Ok(()) | Err(ProtocolError::TooLarge { .. }) => {}
                    Err(error) => panic!("a decoded message did not encode: {error}"),
                }
            }
            Err(_) => return,
        }
    }
}
