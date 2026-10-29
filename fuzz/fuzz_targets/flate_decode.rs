//! Inflates arbitrary bytes, and runs them through the filter pipeline with PNG predictor
//! parameters taken from the first bytes, under default `Limits`.
#![no_main]

use lamina_syntax::filters::{decode, flate_decode};
use lamina_syntax::{DecodeBudget, Dict, Limits, Object};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let limits = Limits::default();
    let _ = flate_decode(data, &limits, &mut DecodeBudget::new(&limits));
    let [predictor, colors, bpc, columns, rest @ ..] = data else {
        return;
    };
    let params: Dict = [
        ("Predictor", Object::Integer(i64::from(*predictor))),
        ("Colors", Object::Integer(i64::from(*colors))),
        ("BitsPerComponent", Object::Integer(i64::from(*bpc))),
        ("Columns", Object::Integer(i64::from(*columns))),
    ]
    .into_iter()
    .collect();
    let dict: Dict = [("Filter", Object::name("FlateDecode")), ("DecodeParms", params.into())]
        .into_iter()
        .collect();
    let _ = decode(&dict, rest, &limits, &mut DecodeBudget::new(&limits));
});
