// LaminaPdfError and LaminaPdfWarning (ADR 0026 §1, §7): every error @laminapdf/sdk throws is a
// LaminaPdfError built from the §1 object the WASM throws, or raised here with the kind and hint
// codes.json gives the code.
import { docsUrl, type ErrorCode, type ErrorKind, RAISED_BY_PACKAGE, type WarningCode } from "./generated/codes.js";

export type { ErrorKind };

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
  /** A LaminaPdfId (`p12.0`). */
  readonly id?: string;
}

/** The ADR 0026 §1 object: what `LaminaPdfError.toJSON()` returns and the WASM throws. */
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
 * A warning: something Lamina PDF repaired or skipped (ADR 0003, ADR 0026 §1). The §1 object without
 * `didYouMean`; document text appears only in `data`.
 */
export type LaminaPdfWarning = Omit<Diagnostic<WarningCode>, "didYouMean">;

/**
 * The one error class of `@laminapdf/sdk` (ADR 0026 §7). `message`, `hint` and `didYouMean` never
 * hold document text (ADR 0026 §4); that goes in `data`, marked untrusted on the code's page at
 * `docsUrl`.
 *
 * @example
 * ```ts
 * try {
 *   Document.open(bytes, { password: "secret" });
 * } catch (e) {
 *   if (isLaminaPdfError(e, "wrong-password")) askAgain();
 *   else throw e;
 * }
 * ```
 */
export class LaminaPdfError<C extends ErrorCode = ErrorCode> extends Error {
  override readonly name = "LaminaPdfError";
  /** The stable code (`fixtures/codes.json`). */
  readonly code: C;
  /** What the caller can do about it. */
  readonly kind: ErrorKind;
  /** One actionable sentence for this code. */
  readonly hint?: string;
  /** For `usage` errors: the closest known names (Lamina PDF's own vocabulary). */
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
    // ADR 0026 §1: optional keys are omitted, and an empty list is no candidates at all.
    if (diagnostic.didYouMean !== undefined && diagnostic.didYouMean.length > 0)
      this.didYouMean = diagnostic.didYouMean;
    if (diagnostic.location !== undefined) this.location = diagnostic.location;
    if (diagnostic.data !== undefined) this.data = diagnostic.data;
  }

  /**
   * For `data-pending`: the byte ranges a range document needs fetched before the call can
   * succeed (`data.ranges`); `undefined` for other codes. {@link Document.ensure} fetches them.
   */
  get ranges(): readonly { readonly offset: number; readonly length: number }[] | undefined {
    if (this.code !== "data-pending") return undefined;
    const raw = this.data?.ranges;
    if (!Array.isArray(raw)) return [];
    return raw.flatMap((r: unknown) =>
      Array.isArray(r) && typeof r[0] === "number" && typeof r[1] === "number" ? [{ offset: r[0], length: r[1] }] : [],
    );
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
 * Whether `e` is a {@link LaminaPdfError}, with `code` when given.
 *
 * @param e - anything caught.
 * @param code - the code to match.
 * @returns true for a LaminaPdfError (with that code).
 */
export function isLaminaPdfError<C extends ErrorCode>(e: unknown, code?: C): e is LaminaPdfError<C> {
  return e instanceof LaminaPdfError && (code === undefined || e.code === code);
}

/** Whether `value` is the §1 object the WASM throws. */
export function isDiagnostic(value: unknown): value is Diagnostic<ErrorCode> {
  if (typeof value !== "object" || value === null || value instanceof Error) return false;
  const v = value as Record<string, unknown>;
  return typeof v.code === "string" && typeof v.kind === "string" && typeof v.docsUrl === "string";
}

/** A LaminaPdfError for a code `@laminapdf/sdk` raises itself (kind and hint from codes.json). */
export function packageError<C extends keyof typeof RAISED_BY_PACKAGE>(
  code: C,
  message: string,
  extra: Pick<Diagnostic<C>, "didYouMean" | "data"> & { readonly cause?: unknown } = {},
): LaminaPdfError<C> {
  const { kind, hint } = RAISED_BY_PACKAGE[code];
  const { cause, ...fields } = extra;
  return new LaminaPdfError<C>(
    { code, kind, message, hint, ...fields, docsUrl: docsUrl(code) },
    cause === undefined ? undefined : { cause },
  );
}

/**
 * The names in `known` within two edits of `given`, closest first; names at the same distance in
 * code-point order. Edits count code points, after lowercasing ASCII letters only.
 * laminapdf-wasm follows the same rule, and fixtures/did-you-mean.json pins both. `<` compares
 * UTF-16 code units, which is code-point order for the library's names (all ASCII).
 */
export function didYouMean(given: string, known: readonly string[]): string[] {
  const chars = (s: string): string[] => Array.from(s.replace(/[A-Z]/g, (c) => c.toLowerCase()));
  const distance = (a: readonly string[], b: readonly string[]): number => {
    let row = Array.from({ length: b.length + 1 }, (_, i) => i);
    for (let i = 0; i < a.length; i++) {
      const next = [i + 1];
      for (let j = 0; j < b.length; j++) {
        const cost = (row[j] ?? 0) + (a[i] === b[j] ? 0 : 1);
        next.push(Math.min(cost, (row[j + 1] ?? 0) + 1, (next[j] ?? 0) + 1));
      }
      row = next;
    }
    return row[b.length] ?? 0;
  };
  const target = chars(given);
  return known
    .map((k) => [distance(target, chars(k)), k] as const)
    .filter(([d]) => d <= 2)
    .sort(([dx, x], [dy, y]) => dx - dy || (x < y ? -1 : Number(x > y)))
    .map(([, k]) => k);
}

/**
 * Runs `f`, turning the §1 object the WASM throws into a {@link LaminaPdfError}. Other errors (a
 * `RangeError`, a trap) pass through unchanged.
 */
export function guard<T>(f: () => T): T {
  try {
    return f();
  } catch (e) {
    throw isDiagnostic(e) ? new LaminaPdfError(e) : e;
  }
}
