//! Parses arbitrary bytes as a direct object and as an indirect object (with a `/Length`
//! resolver that fails, forcing the `endstream` scan), under default `Limits`.
#![no_main]

use lamina_syntax::{Limits, Parser, parse_object_with};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let limits = Limits::default();
    if let Ok(obj) = parse_object_with(data, &limits) {
        // Whatever parses must serialise.
        let _ = obj.to_bytes();
    }
    let _ = Parser::new(data, 0).with_limits(&limits).indirect_object(&mut |_| None);
});
