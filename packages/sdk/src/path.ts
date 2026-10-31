// Paths in the flat encoding lamina-wasm reads (ADR 0016 §7): a verb, then its numbers.
import type { Point, Rect } from "./types.js";

/** Builds a path in page space; every method returns the builder, so calls chain. */
export interface PathBuilder {
  /** Starts a subpath at (x, y). */
  moveTo(x: number, y: number): PathBuilder;
  /** A straight line to (x, y). */
  lineTo(x: number, y: number): PathBuilder;
  /** A cubic Bézier curve through two control points to (x, y). */
  cubicTo(x1: number, y1: number, x2: number, y2: number, x: number, y: number): PathBuilder;
  /** A quadratic Bézier curve through one control point to (x, y). */
  quadTo(x1: number, y1: number, x: number, y: number): PathBuilder;
  /** Closes the subpath. */
  close(): PathBuilder;
  /** A rectangle: left, top, width, height. */
  rect(rect: Rect): PathBuilder;
  /** The ellipse inscribed in a rectangle. */
  ellipse(rect: Rect): PathBuilder;
  /** A circle around `center`. */
  circle(center: Point, radius: number): PathBuilder;
}

/** The verbs of the flat encoding. */
const MOVE = 0;
const LINE = 1;
const CUBIC = 2;
const QUAD = 3;
const CLOSE = 4;
const RECT = 5;
const ELLIPSE = 6;
const CIRCLE = 7;

/** @internal Collects a path's verbs and numbers. */
export class PathEncoder implements PathBuilder {
  readonly #data: number[] = [];

  moveTo(x: number, y: number): this {
    this.#data.push(MOVE, x, y);
    return this;
  }

  lineTo(x: number, y: number): this {
    this.#data.push(LINE, x, y);
    return this;
  }

  cubicTo(x1: number, y1: number, x2: number, y2: number, x: number, y: number): this {
    this.#data.push(CUBIC, x1, y1, x2, y2, x, y);
    return this;
  }

  quadTo(x1: number, y1: number, x: number, y: number): this {
    this.#data.push(QUAD, x1, y1, x, y);
    return this;
  }

  close(): this {
    this.#data.push(CLOSE);
    return this;
  }

  rect(rect: Rect): this {
    this.#data.push(RECT, ...rect);
    return this;
  }

  ellipse(rect: Rect): this {
    this.#data.push(ELLIPSE, ...rect);
    return this;
  }

  circle(center: Point, radius: number): this {
    this.#data.push(CIRCLE, center[0], center[1], radius);
    return this;
  }

  finish(): Float64Array {
    return new Float64Array(this.#data);
  }
}
