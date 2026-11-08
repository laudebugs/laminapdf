// Touch gestures, which the viewer owns (UI/UX plan §5.4): the root has `touch-action: none`, so the
// browser neither scrolls nor zooms the page under a finger, and these do instead. One finger pans,
// with momentum after a flick; two fingers pinch, keeping the document point under them where it
// started; a double tap zooms in, or back to the page width. A finger held still passes through, so
// a long press still selects text. Mouse and pen input is left to the browser.

/** A position in viewport CSS px, or a time-stamped one. */
interface At {
  readonly x: number;
  readonly y: number;
}
interface Sample extends At {
  /** ms */
  readonly t: number;
}

/** How far a finger moves (CSS px) before a touch is a pan rather than a tap or a long press. */
export const SLOP = 10;
/** A finger held still this long (ms) is a long press, left to the browser's text selection. */
export const LONG_PRESS_MS = 500;
/** The most time (ms) and distance (CSS px) between two taps of a double tap. */
export const DOUBLE_TAP = { ms: 300, distance: 24 } as const;
/** How long a tap may last (ms). */
const TAP_MS = 250;
/** Momentum's time constant (ms): the speed falls to about a third each this long, as on iOS. */
const MOMENTUM_MS = 325;
/** Momentum stops below this speed (CSS px per ms). */
const MOMENTUM_STOP = 0.02;
/** The last stretch of a pan (ms) whose movement sets the flick's speed. */
const VELOCITY_WINDOW = 100;

/** Whether tap `b` is the second tap of a double tap whose first was `a`. */
export function isDoubleTap(a: Sample, b: Sample): boolean {
  return b.t - a.t <= DOUBLE_TAP.ms && Math.hypot(b.x - a.x, b.y - a.y) <= DOUBLE_TAP.distance;
}

/** The speed (CSS px per ms) a pan ends with: its movement over the last {@link VELOCITY_WINDOW} ms. */
export function releaseVelocity(samples: readonly Sample[]): At {
  const last = samples.at(-1);
  if (last === undefined) return { x: 0, y: 0 };
  const first = samples.find((s) => last.t - s.t <= VELOCITY_WINDOW) ?? last;
  const dt = last.t - first.t;
  return dt <= 0 ? { x: 0, y: 0 } : { x: (last.x - first.x) / dt, y: (last.y - first.y) / dt };
}

/**
 * One frame of momentum `dt` ms long at speed `v`: how far it moves and its speed after, or
 * undefined once it has slowed to a stop. The distance is the speed's integral over the frame.
 */
export function momentumStep(v: At, dt: number): { readonly move: At; readonly v: At } | undefined {
  if (Math.hypot(v.x, v.y) < MOMENTUM_STOP) return undefined;
  const decay = Math.exp(-dt / MOMENTUM_MS);
  const travelled = MOMENTUM_MS * (1 - decay);
  return { move: { x: v.x * travelled, y: v.y * travelled }, v: { x: v.x * decay, y: v.y * decay } };
}

/** The zoom percentage of a pinch that started at `percent` with the fingers `from` apart and is `to` apart now. */
export function pinchPercent(percent: number, from: number, to: number): number {
  return from <= 0 ? percent : percent * (to / from);
}

/** What the gestures change: the viewer, seen through the calls they need. */
export interface TouchHost<Point> {
  /** The zoom percentage now. */
  percent(): number;
  /** Whether the zoom is the page width now, so a double tap zooms in rather than back to it. */
  fitsWidth(): boolean;
  /** Zooms to `percent`, or to the page width, keeping viewport position `at` where it is. */
  zoom(to: number | "fit-width", at: At): void;
  /** The document point at viewport position `at`, and where a document point is now. */
  pointAt(at: At): Point | undefined;
  positionOf(point: Point): At | undefined;
  scrollBy(dx: number, dy: number): void;
  /** A pointer event's position in viewport CSS px. */
  local(e: { readonly clientX: number; readonly clientY: number }): At;
  frame(callback: (now: number) => void): number;
  cancelFrame(id: number): void;
}

type Gesture<Point> =
  | { readonly kind: "pending"; readonly start: Sample }
  | { readonly kind: "pan"; last: At; readonly samples: Sample[] }
  | { readonly kind: "pinch"; readonly percent: number; readonly distance: number; readonly point: Point | undefined }
  | { readonly kind: "passthrough" };

/** The touch gestures over one viewer; feed it the root's pointer events. */
export class TouchGestures<Point> {
  readonly #host: TouchHost<Point>;
  readonly #fingers = new Map<number, At>();
  #gesture: Gesture<Point> | undefined;
  #lastTap: Sample | undefined;
  #longPress: ReturnType<typeof setTimeout> | undefined;
  #momentum: number | undefined;

  constructor(host: TouchHost<Point>) {
    this.#host = host;
  }

  /** A pointer went down; true when the gestures took it. */
  down(e: PointerEvent): boolean {
    if (e.pointerType !== "touch") return false;
    this.#stopMomentum();
    const at = this.#host.local(e);
    this.#fingers.set(e.pointerId, at);
    if (this.#fingers.size === 2) this.#startPinch();
    else if (this.#fingers.size === 1) this.#startPending(at, e.timeStamp);
    return true;
  }

  move(e: PointerEvent): void {
    if (!this.#fingers.has(e.pointerId)) return;
    const at = this.#host.local(e);
    this.#fingers.set(e.pointerId, at);
    const g = this.#gesture;
    if (g === undefined || g.kind === "passthrough") return;
    if (g.kind === "pinch") {
      this.#pinch(g);
      return;
    }
    if (g.kind === "pending") {
      if (Math.hypot(at.x - g.start.x, at.y - g.start.y) < SLOP) return;
      clearTimeout(this.#longPress);
      this.#gesture = { kind: "pan", last: g.start, samples: [g.start] };
    }
    const pan = this.#gesture;
    if (pan?.kind !== "pan") return;
    this.#host.scrollBy(pan.last.x - at.x, pan.last.y - at.y);
    pan.last = at;
    pan.samples.push({ ...at, t: e.timeStamp });
    if (pan.samples.length > 32) pan.samples.splice(0, 16);
  }

  up(e: PointerEvent): void {
    if (!this.#fingers.delete(e.pointerId)) return;
    clearTimeout(this.#longPress);
    const g = this.#gesture;
    const left = [...this.#fingers.values()][0];
    if (g?.kind === "pinch") {
      // The finger that stays pans on from where it is, so the page never jumps.
      this.#gesture = left === undefined ? undefined : { kind: "pan", last: left, samples: [] };
      return;
    }
    if (this.#fingers.size > 0) return;
    this.#gesture = undefined;
    if (e.type === "pointercancel") return;
    if (g?.kind === "pan") this.#fling(releaseVelocity(g.samples));
    else if (g?.kind === "pending" && e.timeStamp - g.start.t <= TAP_MS) this.#tap(g.start);
  }

  /** Stops what is running (momentum, a long-press timer); the viewer calls it on destroy. */
  stop(): void {
    this.#stopMomentum();
    clearTimeout(this.#longPress);
    this.#fingers.clear();
    this.#gesture = undefined;
  }

  #startPending(at: At, t: number): void {
    const start = { ...at, t };
    this.#gesture = { kind: "pending", start };
    clearTimeout(this.#longPress);
    this.#longPress = setTimeout(() => {
      if (this.#gesture?.kind === "pending") this.#gesture = { kind: "passthrough" };
    }, LONG_PRESS_MS);
  }

  #startPinch(): void {
    clearTimeout(this.#longPress);
    const s = this.#spread();
    if (s === undefined) return;
    this.#gesture = {
      kind: "pinch",
      percent: this.#host.percent(),
      distance: s.distance,
      point: this.#host.pointAt(s.mid),
    };
  }

  #pinch(g: Extract<Gesture<Point>, { kind: "pinch" }>): void {
    const s = this.#spread();
    if (s === undefined) return;
    this.#host.zoom(pinchPercent(g.percent, g.distance, s.distance), s.mid);
    // The zoom kept the midpoint's own point still; the fingers may also have moved together, so
    // the point that started under them is scrolled back under them.
    const now = g.point === undefined ? undefined : this.#host.positionOf(g.point);
    if (now !== undefined) this.#host.scrollBy(now.x - s.mid.x, now.y - s.mid.y);
  }

  #spread(): { readonly distance: number; readonly mid: At } | undefined {
    const [a, b] = [...this.#fingers.values()];
    if (a === undefined || b === undefined) return undefined;
    return { distance: Math.hypot(a.x - b.x, a.y - b.y), mid: { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 } };
  }

  #tap(tap: Sample): void {
    const first = this.#lastTap;
    if (first !== undefined && isDoubleTap(first, tap)) {
      this.#lastTap = undefined;
      this.#host.zoom(this.#host.fitsWidth() ? this.#host.percent() * 2 : "fit-width", tap);
      return;
    }
    this.#lastTap = tap;
  }

  #fling(v: At): void {
    let speed = { x: -v.x, y: -v.y };
    let before: number | undefined;
    const step = (now: number) => {
      const dt = before === undefined ? 16 : now - before;
      before = now;
      const next = momentumStep(speed, dt);
      if (next === undefined) {
        this.#momentum = undefined;
        return;
      }
      this.#host.scrollBy(next.move.x, next.move.y);
      speed = next.v;
      this.#momentum = this.#host.frame(step);
    };
    this.#momentum = this.#host.frame(step);
  }

  #stopMomentum(): void {
    if (this.#momentum !== undefined) this.#host.cancelFrame(this.#momentum);
    this.#momentum = undefined;
  }
}
