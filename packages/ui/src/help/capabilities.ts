// The capability page's data and filters (O05), apart from React: the matrix as `cargo xtask
// capabilities` writes it (docs/capabilities/matrix.json, which the app bundles at build time),
// and what a search, a group and the shown surfaces leave of it.

/** A status a test proves, weakest first. */
export type CapabilityStatus = "unsupported" | "read" | "write" | "round-trip";

/** A column of the matrix. */
export type CapabilitySurface = "core" | "pdf" | "web" | "desktop";

/** The columns, in the matrix's order. */
export const SURFACES: readonly CapabilitySurface[] = ["core", "pdf", "web", "desktop"];

/** One operation: its best proven status per surface, none (`null`), or a surface it doesn't target. */
export interface CapabilityOp {
  readonly id: string;
  readonly name: string;
  /** The editor round that targets it ("E1"). */
  readonly round: string;
  readonly target: CapabilityStatus;
  readonly surfaces: Readonly<Record<CapabilitySurface, CapabilityStatus | "n/a" | null>>;
}

/** A group of operations ("C · Page organisation & geometry"). */
export interface CapabilityGroup {
  readonly id: string;
  readonly name: string;
  readonly ops: readonly CapabilityOp[];
}

/** The matrix, version 1. */
export interface CapabilityMatrix {
  readonly version: 1;
  readonly groups: readonly CapabilityGroup[];
}

/** What the page shows: a search, one group or all, and the surfaces whose columns show. */
export interface CapabilityFilter {
  readonly query: string;
  /** A group's ID, or `"all"`. */
  readonly group: string;
  readonly surfaces: readonly CapabilitySurface[];
}

/** No search, every group, every column. */
export const NO_FILTER: CapabilityFilter = { query: "", group: "all", surfaces: SURFACES };

/** Whether `op` matches `query`: its name or ID contains it, ignoring case and accents. */
function matches(op: CapabilityOp, query: string): boolean {
  const fold = (text: string) =>
    text
      .normalize("NFD")
      .replace(/\p{Mn}/gu, "")
      .toLowerCase();
  const q = fold(query.trim());
  return q === "" || fold(op.name).includes(q) || fold(op.id).includes(q);
}

/** The groups and operations `filter` leaves, with no empty group, and how many of each. */
export function filterMatrix(
  matrix: CapabilityMatrix,
  filter: CapabilityFilter,
): { readonly groups: readonly CapabilityGroup[]; readonly ops: number } {
  const groups = matrix.groups
    .filter((group) => filter.group === "all" || group.id === filter.group)
    .map((group) => ({ ...group, ops: group.ops.filter((op) => matches(op, filter.query)) }))
    .filter((group) => group.ops.length > 0);
  return { groups, ops: groups.reduce((n, group) => n + group.ops.length, 0) };
}

/** A filter read back from storage, or the default for anything that isn't one. */
export function storedFilter(value: unknown, matrix: CapabilityMatrix): CapabilityFilter {
  if (typeof value !== "object" || value === null) return NO_FILTER;
  const v = value as Partial<Record<keyof CapabilityFilter, unknown>>;
  const group =
    typeof v.group === "string" && (v.group === "all" || matrix.groups.some((g) => g.id === v.group)) ? v.group : "all";
  const surfaces = Array.isArray(v.surfaces) ? SURFACES.filter((s) => (v.surfaces as unknown[]).includes(s)) : SURFACES;
  return { query: typeof v.query === "string" ? v.query : "", group, surfaces };
}
