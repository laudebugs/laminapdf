/** Machine-readable error categories. Every failure the library raises is a {@link PdfError}. */
export type PdfErrorCode =
  /** Malformed syntax (bad token, unexpected token, unterminated construct). */
  | "Syntax"
  /** Input ended before a construct was complete. */
  | "UnexpectedEof"
  /** Array/dict nesting deeper than the limit (256). */
  | "DepthLimit"
  /** A reference chain or page tree loops back on itself. */
  | "Cycle"
  /** Neither the cross-reference data nor repair produced a usable document. */
  | "Xref"
  /** A filter (Flate, predictor) failed to decode. */
  | "Filter"
  /** A filter we do not implement in this spike. */
  | "UnsupportedFilter"
  /** A configured resource limit (decompressed size, ...) was exceeded. */
  | "LimitExceeded"
  /** Structurally valid syntax with the wrong shape (e.g. /Pages is not a dict). */
  | "Invalid";

export class PdfError extends Error {
  readonly code: PdfErrorCode;
  /** Byte offset in the input the error relates to, when known. */
  readonly offset: number | undefined;

  constructor(code: PdfErrorCode, message: string, offset?: number) {
    super(offset === undefined ? `${code}: ${message}` : `${code} at ${offset}: ${message}`);
    this.name = "PdfError";
    this.code = code;
    this.offset = offset;
  }
}

export function isPdfError(e: unknown): e is PdfError {
  return e instanceof PdfError;
}
