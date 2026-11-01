//! Size probe: one exported function per generator; inputs come through `black_box` so the
//! optimiser keeps every code path. Never run, only measured.
#![allow(dead_code, reason = "assets.rs helpers that only the native binary uses")]

#[path = "../../src/assets.rs"]
mod assets;
#[cfg(feature = "krilla-gen")]
#[path = "../../src/exp_a.rs"]
mod exp_a;
#[cfg(feature = "lamina-gen")]
#[path = "../../src/exp_c.rs"]
mod exp_c;

use std::hint::black_box;

fn inputs() -> assets::Assets {
    assets::Assets {
        font: black_box(Vec::new()),
        png_alpha: black_box(Vec::new()),
        jpeg: black_box(Vec::new()),
        jpeg_size: black_box((1, 1)),
    }
}

/// Generates the sample and opens it with laminapdf-cos (as `@lamina/pdf` would after `finish`).
#[unsafe(no_mangle)]
pub extern "C" fn generate() -> usize {
    let a = inputs();
    #[cfg(feature = "krilla-gen")]
    let bytes = exp_a::generate(&a, black_box(true)).unwrap_or_default();
    #[cfg(feature = "lamina-gen")]
    let bytes = exp_c::generate(&a, black_box(true))
        .map(|b| b.bytes)
        .unwrap_or_default();
    #[cfg(not(any(feature = "krilla-gen", feature = "lamina-gen")))]
    let bytes = black_box(a.font);
    laminapdf_cos::open(&bytes).and_then(|d| d.page_count()).unwrap_or(0)
}
