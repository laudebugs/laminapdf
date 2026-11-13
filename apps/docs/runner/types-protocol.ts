// The messages between the "Edit and run" editor (types.ts) and its TypeScript worker
// (types-worker.ts, todo Q4t). Each request names the version of the editor's code it asks about,
// and carries that code when the worker hasn't been sent it yet, so the worker answers about the
// code the reader sees; positions are offsets into that code.

/** What the editor asks. */
export type TypesQuery =
  | { readonly kind: "complete"; readonly pos: number }
  | { readonly kind: "details"; readonly pos: number; readonly name: string; readonly source?: string }
  | { readonly kind: "hover"; readonly pos: number }
  | { readonly kind: "lint" };

/** A request: a query about the code at `version`, which comes with it unless it came with the last request. */
export type TypesRequest = TypesQuery & {
  readonly id: number;
  readonly version: number;
  readonly code?: string;
};

/** One completion: TypeScript's name, kind and sort order for it. */
export interface TypesCompletion {
  readonly label: string;
  /** TypeScript's `ScriptElementKind`, such as `method` or `property`. */
  readonly kind: string;
  readonly sortText: string;
  /** The module an auto-import would come from: never set, since auto-imports are off. */
  readonly source?: string;
}

/** A completion's signature and the first paragraph of its documentation. */
export interface TypesDetails {
  readonly signature: string;
  readonly documentation: string;
}

/** What's at a position: the range of the name and its signature and documentation. */
export interface TypesHover extends TypesDetails {
  readonly from: number;
  readonly to: number;
}

/** A type error or a syntax error. */
export interface TypesDiagnostic {
  readonly from: number;
  readonly to: number;
  readonly severity: "error" | "warning" | "info";
  readonly message: string;
}

/** The answer to request `id`: `null` where TypeScript has nothing to say. */
export type TypesReply =
  | { readonly id: number; readonly kind: "complete"; readonly items: readonly TypesCompletion[] }
  | { readonly id: number; readonly kind: "details"; readonly details: TypesDetails | null }
  | { readonly id: number; readonly kind: "hover"; readonly hover: TypesHover | null }
  | { readonly id: number; readonly kind: "lint"; readonly diagnostics: readonly TypesDiagnostic[] };
