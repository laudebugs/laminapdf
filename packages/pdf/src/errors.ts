// LaminaError and LaminaWarning (ADR 0026 §1, §7): every error @lamina/pdf throws is a
// LaminaError built from the §1 object the WASM throws, or raised here with the kind and hint
// codes.json gives the code.
import { DOCS_URL, type ErrorCode, RAISED_BY_PACKAGE, type WarningCode } from "./codes.generated.js";

/** The nine kinds of ADR 0026 §2: what the caller can do about an error. */
export type ErrorKind =
  | "input"
  | "io"
  | "unsupported"
  | "limit"
  | "security"
  | "usage"
  | "state"
  | "cancelled"
  | "internal";

/** Where a problem is (ADR 0026 §1.1); each field has one meaning, all lamina-made values. */
export interface Location {
  /** Byte offset in the input file. */
  readonly offset?: number;
  /** Object number of the indirect object the problem is in or at. */
  readonly object?: number;
  /** Byte offset in the decoded data of `object`. */
  readonly streamOffset?: number;
  /** 0-based page index (ADR 0027 §6). */
  readonly pageIndex?: number;
  /** A LaminaId (`p12.0`). */
  readonly id?: string;
}

/** The ADR 0026 §1 object: what `LaminaError.toJSON()` returns and the WASM throws. */
export interface Diagnostic<C extends string = ErrorCode> {
  readonly code: C;
  readonly kind: ErrorKind;
  readonly message: string;
  readonly hint?: string;
  readonly didYouMean?: readonly string[];
  readonly location?: Location;
  readonly data?: Readonly<Record<string, unknown>>;
  readonly docsUrl: string;
}

/**
 * A warning: something lamina repaired or skipped (ADR 0003, ADR 0026 §1). The §1 object without
 * `didYouMean`; document text appears only in `data`.
 */
export type LaminaWarning = Omit<Diagnostic<WarningCode>, "didYouMean">;

/**
 * The one error class of `@lamina/pdf` (ADR 0026 §7). `message`, `hint` and `didYouMean` never
 * hold document text (ADR 0026 §4); that goes in `data`, marked untrusted on the code's page at
 * `docsUrl`.
 *
 * @example
 * ```ts
 * try {
 *   Document.open(bytes, { password: "secret" });
 * } catch (e) {
 *   if (isLaminaError(e, "wrong-password")) askAgain();
 *   else throw e;
 * }
 * ```
 */
export class LaminaError<C extends ErrorCode = ErrorCode> extends Error {
  override readonly name = "LaminaError";
  /** The stable code (`fixtures/codes.json`). */
  readonly code: C;
  /** What the caller can do about it. */
  readonly kind: ErrorKind;
  /** One actionable sentence for this code. */
  readonly hint?: string;
  /** For `usage` errors: the closest known names (lamina's own vocabulary). */
  readonly didYouMean?: readonly string[];
  /** Where in the input or the document. */
  readonly location?: Location;
  /** Code-specific fields, documented on the code's page (the only place document text appears). */
  readonly data?: Readonly<Record<string, unknown>>;
  /** The code's page. */
  readonly docsUrl: string;

  /**
   * @param diagnostic - the ADR 0026 §1 object.
   * @param options - `cause`, when wrapping another error.
   */
  constructor(diagnostic: Diagnostic<C>, options?: { readonly cause?: unknown }) {
    super(diagnostic.message, options);
    this.code = diagnostic.code;
    this.kind = diagnostic.kind;
    this.docsUrl = diagnostic.docsUrl;
    if (diagnostic.hint !== undefined) this.hint = diagnostic.hint;
    if (diagnostic.didYouMean !== undefined) this.didYouMean = diagnostic.didYouMean;
    if (diagnostic.location !== undefined) this.location = diagnostic.location;
    if (diagnostic.data !== undefined) this.data = diagnostic.data;
  }

  /** The ADR 0026 §1 object (for logs, workers and `--json` output); optional fields omitted. */
  toJSON(): Diagnostic<C> {
    return {
      code: this.code,
      kind: this.kind,
      message: this.message,
      ...(this.hint === undefined ? {} : { hint: this.hint }),
      ...(this.didYouMean === undefined ? {} : { didYouMean: this.didYouMean }),
      ...(this.location === undefined ? {} : { location: this.location }),
      ...(this.data === undefined ? {} : { data: this.data }),
      docsUrl: this.docsUrl,
    };
  }
}

/**
 * Whether `e` is a {@link LaminaError}, with `code` when given.
 *
 * @param e - anything caught.
 * @param code - the code to match.
 * @returns true for a LaminaError (with that code).
 */
export function isLaminaError<C extends ErrorCode>(e: unknown, code?: C): e is LaminaError<C> {
  return e instanceof LaminaError && (code === undefined || e.code === code);
}

/** Whether `value` is the §1 object the WASM throws. */
export function isDiagnostic(value: unknown): value is Diagnostic<ErrorCode> {
  if (typeof value !== "object" || value === null || value instanceof Error) return false;
  const v = value as Record<string, unknown>;
  return typeof v.code === "string" && typeof v.kind === "string" && typeof v.docsUrl === "string";
}

/** A LaminaError for a code `@lamina/pdf` raises itself (kind and hint from codes.json). */
export function packageError<C extends keyof typeof RAISED_BY_PACKAGE>(code: C, message: string): LaminaError<C> {
  const { kind, hint } = RAISED_BY_PACKAGE[code];
  return new LaminaError<C>({ code, kind, message, hint, docsUrl: `${DOCS_URL.base}${code}${DOCS_URL.suffix}` });
}

/**
 * Runs `f`, turning the §1 object the WASM throws into a {@link LaminaError}. Other errors (a
 * `RangeError`, a trap) pass through unchanged.
 */
export function guard<T>(f: () => T): T {
  try {
    return f();
  } catch (e) {
    throw isDiagnostic(e) ? new LaminaError(e) : e;
  }
}
