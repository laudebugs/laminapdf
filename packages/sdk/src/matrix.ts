// Transformation matrices `[a b c d e f]` in page space, computed as the Rust `Matrix` does so
// that the same drawing gives the same bytes.

/**
 * A transformation matrix `[a, b, c, d, e, f]` (ISO 32000-2 §8.3.4), in page space.
 *
 * @since 0.2.0
 */
export type Transform = readonly [a: number, b: number, c: number, d: number, e: number, f: number];

/**
 * Building transformation matrices.
 *
 * @since 0.2.0
 */
export const Matrix = {
  /** The identity. */
  identity: [1, 0, 0, 1, 0, 0] as Transform,
  /**
   * A translation by (x, y) points.
   *
   * @since 0.2.0
   */
  translate(x: number, y: number): Transform {
    return [1, 0, 0, 1, x, y];
  },
  /**
   * A scaling by `sx` horizontally and `sy` vertically.
   *
   * @since 0.2.0
   */
  scale(sx: number, sy: number): Transform {
    return [sx, 0, 0, sy, 0, 0];
  },
  /**
   * A rotation by `degrees` about the origin, clockwise as displayed.
   *
   * @since 0.2.0
   */
  rotate(degrees: number): Transform {
    const radians = degrees * (Math.PI / 180);
    const s = Math.sin(radians);
    const c = Math.cos(radians);
    return [c, s, -s, c, 0, 0];
  },
  /**
   * `outer` applied after `inner`: maps p to outer(inner(p)).
   *
   * @since 0.2.0
   */
  after(outer: Transform, inner: Transform): Transform {
    const [a, b, c, d, e, f] = outer;
    const [ia, ib, ic, id, ie, iff] = inner;
    return [
      a * ia + c * ib,
      b * ia + d * ib,
      a * ic + c * id,
      b * ic + d * id,
      a * ie + c * iff + e,
      b * ie + d * iff + f,
    ];
  },
} as const;
