// Type tests (ADR 0026, L1b): checked by `tsc --noEmit` over test/ (the package's typecheck);
// at run time they only assert that they ran. `code` narrows; a switch over every ErrorCode with
// a `never` default compiles, and stops compiling when codes.json gains a code until the case is
// handled here (the partial switch below shows the error that produces).
import {
  type ErrorCode,
  type ErrorKind,
  isLaminaPdfError,
  LaminaPdfError,
  type LaminaPdfWarning,
  type WarningCode,
} from "@laminapdf/sdk";
import { ERROR_CODES, KINDS } from "@laminapdf/sdk/explain";
import { describe, expect, expectTypeOf, it } from "vitest";

/** Every error code by hand, grouped by kind: a new code in codes.json fails here first. */
function kindOf(code: ErrorCode): ErrorKind {
  switch (code) {
    case "not-pdf":
    case "missing-root":
    case "reference-cycle":
    case "strict":
    case "type":
    case "invalid-structure":
    case "unexpected-eof":
    case "syntax":
    case "decompress":
    case "invalid-op-log":
    case "invalid-font":
    case "missing-glyph":
    case "invalid-image":
    case "invalid-icc-profile":
    case "render-failed":
      return "input";
    case "source":
    case "sink":
    case "fetch-failed":
    case "source-changed":
    case "render-backend-unavailable":
      return "io";
    case "encrypted-base":
    case "unsupported-security-handler":
    case "unsupported-filter":
    case "direct-page":
    case "unsupported-version":
    case "subset-failed":
    case "unsupported-layout-option":
    case "font-not-shapable":
    case "unsupported-image":
      return "unsupported";
    case "limit-exceeded":
    case "encoding-full":
    case "image-too-large":
    case "source-budget-exceeded":
    case "render-too-large":
      return "limit";
    case "password-required":
    case "wrong-password":
    case "would-invalidate-signatures":
      return "security";
    case "unknown-page":
    case "sink-not-empty":
    case "collection-index":
    case "unknown-axis":
    case "font-not-embeddable":
    case "glyph-out-of-range":
    case "unknown-font":
    case "invalid-axis-value":
    case "wrong-document":
    case "invalid-feature":
    case "invalid-text-range":
    case "font-mismatch":
    case "image-data-length":
    case "color-components":
    case "invalid-color":
    case "invalid-geometry":
    case "text-overflow":
    case "page-index-out-of-range":
    case "unknown-destination":
    case "invalid-page-labels":
    case "profile-version-conflict":
    case "missing-title":
    case "missing-lang":
    case "missing-alt-text":
    case "untagged-content":
    case "heading-level-skipped":
    case "unnumbered-heading-misused":
    case "invalid-structure-nesting":
    case "link-without-structure":
    case "link-without-contents":
    case "font-not-embedded":
    case "glyph-without-unicode":
    case "device-color-without-output-intent":
    case "embedded-file-forbidden":
    case "embedded-file-without-mime-type":
    case "invalid-argument":
    case "unknown-option":
    case "worker-version-mismatch":
      return "usage";
    case "data-pending":
    case "repaired-base":
    case "nested-transaction":
    case "no-transaction":
    case "saved-not-rebased":
    case "no-pending-save":
    case "not-rebased":
    case "requires-rewrite":
    case "job-finished":
    case "document-changed":
    case "builder-finished":
    case "already-initialized":
    case "not-initialized":
    case "text-module-not-loaded":
    case "extract-module-not-loaded":
    case "disposed":
      return "state";
    case "cancelled":
      return "cancelled";
    case "unset-object":
    case "worker-crashed":
    case "worker-failed":
      return "internal";
    default: {
      const unhandled: never = code;
      return unhandled;
    }
  }
}

/** A switch that misses codes: its `never` default must not compile. */
function partial(code: ErrorCode): string {
  switch (code) {
    case "wrong-password":
      return "ask again";
    default: {
      // @ts-expect-error: every other code is unhandled, so `code` is not `never`.
      const unhandled: never = code;
      return unhandled;
    }
  }
}

describe("LaminaPdfError types", () => {
  it("switches exhaustively over ErrorCode", () => {
    expect([kindOf("wrong-password"), partial("wrong-password")]).toEqual(["security", "ask again"]);
    // The hand-written grouping agrees with the generated kinds.
    expect(ERROR_CODES.filter((c) => kindOf(c) !== KINDS[c])).toEqual([]);
  });

  it("narrows code with isLaminaPdfError(e, code)", () => {
    const e: unknown = new LaminaPdfError({
      code: "wrong-password",
      kind: "security",
      message: "wrong password",
      docsUrl: "https://example.org/wrong-password.md",
    });
    if (isLaminaPdfError(e, "wrong-password")) {
      expectTypeOf(e.code).toEqualTypeOf<"wrong-password">();
      expectTypeOf(e.toJSON().code).toEqualTypeOf<"wrong-password">();
    }
    if (isLaminaPdfError(e)) expectTypeOf(e.code).toEqualTypeOf<ErrorCode>();
    expect(isLaminaPdfError(e, "wrong-password")).toBe(true);
  });

  it("refuses a code that is not listed", () => {
    // @ts-expect-error: "no-such-code" is not an ErrorCode.
    expect(isLaminaPdfError(new Error("x"), "no-such-code")).toBe(false);
  });

  it("types warnings by WarningCode", () => {
    expectTypeOf<LaminaPdfWarning["code"]>().toEqualTypeOf<WarningCode>();
    expectTypeOf<LaminaPdfError["code"]>().toEqualTypeOf<ErrorCode>();
    // @ts-expect-error: a warning code is not an error code.
    const notAnError: ErrorCode = "xref-repaired";
    expect(notAnError).toBe("xref-repaired");
  });
});
