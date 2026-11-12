// Which exports of a TypeDoc project lack a `@since` tag: the "Added in" version every public
// item of @laminapdf/sdk carries (docs/guidelines/typescript.md, "Added in"). The reference shows
// the tag as "Since"; check-api-docs.mjs fails the check on any export this returns.

/** The package whose exports must each carry `@since`. */
export const SINCE_PACKAGE = "@laminapdf/sdk";

/**
 * The comments that document `reflection`: its own, else its signatures' (functions and methods
 * keep their comment on the signature) or its accessors'.
 *
 * @param {any} reflection a TypeDoc `DeclarationReflection`
 * @returns {any[]}
 */
function commentsOf(reflection) {
  return [
    reflection.comment,
    ...(reflection.signatures ?? []).map((/** @type {any} */ s) => s.comment),
    reflection.getSignature?.comment,
    reflection.setSignature?.comment,
  ].filter((c) => c !== undefined);
}

/**
 * Whether `reflection` has a `@since` tag with a version, `@since 0.2.0`.
 *
 * @param {any} reflection
 */
export function hasSince(reflection) {
  return commentsOf(reflection).some((/** @type {any} */ comment) =>
    (comment.blockTags ?? []).some(
      (/** @type {any} */ tag) =>
        tag.tag === "@since" &&
        /^\d+\.\d+\.\d+$/.test(
          (tag.content ?? [])
            .map((/** @type {any} */ p) => p.text)
            .join("")
            .trim(),
        ),
    ),
  );
}

/**
 * The reflections of `kinds` in `project` without a `@since` tag, as "file:line name", sorted.
 * Inherited members are left out: their declaration carries the tag.
 *
 * @param {{ getReflectionsByKind(kind: number): any[] }} project
 * @param {number} kinds a TypeDoc `ReflectionKind` mask
 * @returns {string[]}
 */
export function missingSince(project, kinds) {
  return project
    .getReflectionsByKind(kinds)
    .filter((r) => r.inheritedFrom === undefined && !hasSince(r))
    .map((r) => {
      const source = r.sources?.[0];
      return `${source ? `${source.fileName}:${source.line}` : "?"} ${r.getFullName?.() ?? r.name}`;
    })
    .sort();
}
