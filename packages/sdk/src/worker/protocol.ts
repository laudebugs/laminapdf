// The worker client's wire protocol: what the page and the worker post to each other. Every
// message is structured-cloneable; errors and warnings travel as ADR 0026 objects (a cloned
// `Error` loses its fields), and handles (documents, pages) as numbers.
import type { Diagnostic } from "../errors.js";
import type { ErrorCode } from "../generated/codes.js";
import { VERSION } from "../version.js";

/**
 * Bumped whenever a message's shape changes (2: priorities, coalescing keys, extension events).
 *
 * @since 0.2.0
 */
export const PROTOCOL_VERSION = 2;

/** What both ends compare on connect. */
export interface Handshake {
  readonly protocol: number;
  readonly version: string;
}

export const HANDSHAKE: Handshake = Object.freeze({ protocol: PROTOCOL_VERSION, version: VERSION });

/** The page's first message: the client's handshake and how to `init()`. */
export interface Hello {
  readonly kind: "hello";
  readonly client: Handshake;
  readonly init?: unknown;
}

/** The worker's answer to `hello`: its own handshake, and whether `init()` worked. */
interface Welcome {
  readonly kind: "welcome";
  readonly worker: Handshake;
  readonly error?: Diagnostic<ErrorCode>;
}

/** A call: `method` on the handle `handle` (0 is the client itself). */
export interface Request {
  readonly kind: "call";
  readonly id: number;
  readonly handle: number;
  readonly method: string;
  readonly params: readonly unknown[];
  /** Higher runs first (default 0); FIFO within one priority. */
  readonly priority?: number;
  /** Calls with the same key are coalesced: they run once, and each caller gets its own answer. */
  readonly key?: string;
}

/** Stops call `id`: dropped if it hasn't started, cancelled at its next step if it is stepped. */
export interface Cancel {
  readonly kind: "cancel";
  readonly id: number;
}

type Response =
  | { readonly kind: "result"; readonly id: number; readonly ok: true; readonly result: unknown }
  | { readonly kind: "result"; readonly id: number; readonly ok: false; readonly error: Diagnostic<ErrorCode> };

/** Fire-and-forget news from the worker. */
type Event =
  | { readonly kind: "progress"; readonly id: number; readonly progress: unknown }
  | { readonly kind: "crashed"; readonly reason: string }
  /** An extension's own news (`emit()` in the worker, `on("event")` on the page). */
  | { readonly kind: "event"; readonly name: string; readonly data: unknown };

export type ToWorker = Hello | Request | Cancel;
export type FromWorker = Welcome | Response | Event;

/** A handle the worker made, as the page sees it: its number and what it is. */
export interface HandleRef {
  readonly $handle: number;
  readonly type: "document" | "page";
  /** Values known when the handle was made (a page's `id` and `index`). */
  readonly fields?: Readonly<Record<string, unknown>>;
}

export function isHandleRef(value: unknown): value is HandleRef {
  return typeof value === "object" && value !== null && typeof (value as { $handle?: unknown }).$handle === "number";
}

/** A `PageText` as data: the raw arrays the page rebuilds it from. */
export interface PageTextData {
  readonly $pageText: unknown;
}

export function isPageTextData(value: unknown): value is PageTextData {
  return typeof value === "object" && value !== null && "$pageText" in value;
}
