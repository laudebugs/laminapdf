//! Adam7 interlacing: the passes and their sizes.

/// The Adam7 passes as (x start, y start, x step, y step), or the whole image.
pub(super) const ADAM7: [(u32, u32, u32, u32); 7] = [
    (0, 0, 8, 8),
    (4, 0, 8, 8),
    (0, 4, 4, 8),
    (2, 0, 4, 4),
    (0, 2, 2, 4),
    (1, 0, 2, 2),
    (0, 1, 1, 2),
];

/// The number of pixels a pass takes from a line of `size`.
pub(super) const fn pass_size(size: u32, start: u32, step: u32) -> u32 {
    if size > start { (size - start).div_ceil(step) } else { 0 }
}
