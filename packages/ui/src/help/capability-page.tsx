// The capability page (O05): what Lamina PDF reads, writes and round-trips on each surface, as
// its tests prove, from the matrix the app bundles when it is built (no request). A search, a
// group and the shown columns filter it; they are kept per viewer in platform storage. One real
// table, a row group per group of operations, badges that say their status in words.

import { type ReactNode, useEffect, useId, useState } from "react";
import { Badge } from "../components/badge.js";
import { Button } from "../components/button.js";
import { EmptyState } from "../components/empty-state.js";
import { Select, TextInput } from "../components/field.js";
import { ToggleButton } from "../components/toggle.js";
import { countMessage, type MessageId, message } from "../messages.js";
import { usePlatform } from "../platform/context.js";
import {
  type CapabilityFilter,
  type CapabilityGroup,
  type CapabilityMatrix,
  type CapabilityStatus,
  type CapabilitySurface,
  filterMatrix,
  NO_FILTER,
  SURFACES,
  storedFilter,
} from "./capabilities.js";

/** Where the filters are kept. */
const STORAGE_KEY = "capabilities.filter";

const SURFACE_LABEL: Readonly<Record<CapabilitySurface, MessageId>> = {
  core: "capabilities.surface.core",
  pdf: "capabilities.surface.pdf",
  web: "capabilities.surface.web",
  desktop: "capabilities.surface.desktop",
};

const STATUS_LABEL: Readonly<Record<CapabilityStatus, MessageId>> = {
  "round-trip": "capabilities.status.roundTrip",
  write: "capabilities.status.write",
  read: "capabilities.status.read",
  unsupported: "capabilities.status.unsupported",
};

/** A status as its badge: round-trip positive with a check, write accent, read neutral, unsupported dashed. */
export function StatusBadge({ status }: { status: CapabilityStatus | "n/a" | null }): ReactNode {
  if (status === null) {
    return (
      <span className="paper-capability-none" title={message("capabilities.status.none")}>
        <span aria-hidden="true">—</span>
        <span className="paper-visually-hidden">{message("capabilities.status.none")}</span>
      </span>
    );
  }
  if (status === "n/a")
    return <span className="paper-capability-none">{message("capabilities.status.notTargeted")}</span>;
  const label = message(STATUS_LABEL[status]);
  switch (status) {
    case "round-trip":
      return (
        <Badge tone="positive" icon="check">
          {label}
        </Badge>
      );
    case "write":
      return <Badge tone="accent">{label}</Badge>;
    case "unsupported":
      return (
        <span className="paper-capability-unsupported">
          <Badge>{label}</Badge>
        </span>
      );
    default:
      return <Badge>{label}</Badge>;
  }
}

/** Props of `CapabilityPage`. */
export interface CapabilityPageProps {
  readonly matrix: CapabilityMatrix;
  /** The app's version, in the provenance line. */
  readonly version: string;
  /** Back's label: "Back to Q3-board-report.pdf", or back to the reader. */
  readonly backLabel: string;
  readonly onBack: () => void;
  /** "How statuses are tested": opens the matrix's own page. */
  readonly onHowTested: () => void;
}

/** The page's filters, kept per viewer in platform storage (O05). */
function useStoredFilter(matrix: CapabilityMatrix): [CapabilityFilter, (next: CapabilityFilter) => void] {
  const platform = usePlatform();
  const [filter, setFilterState] = useState<CapabilityFilter>(NO_FILTER);
  useEffect(() => {
    let live = true;
    platform.storage.get(STORAGE_KEY).then(
      (stored) => {
        if (live && stored !== undefined) setFilterState(storedFilter(stored, matrix));
      },
      () => undefined,
    );
    return () => {
      live = false;
    };
  }, [platform, matrix]);
  const setFilter = (next: CapabilityFilter) => {
    setFilterState(next);
    // Kept for this viewer only; where storage is blocked the filters last until the page closes.
    platform.storage.set(STORAGE_KEY, next).catch(() => undefined);
  };
  return [filter, setFilter];
}

/** The search, the group and the columns to show: a search landmark. */
function Filters({
  matrix,
  filter,
  onChange,
}: {
  matrix: CapabilityMatrix;
  filter: CapabilityFilter;
  onChange: (next: CapabilityFilter) => void;
}): ReactNode {
  return (
    <search className="paper-capability-filters" aria-label={message("capabilities.filters")}>
      <TextInput
        type="search"
        label={message("capabilities.search")}
        value={filter.query}
        onChange={(query) => onChange({ ...filter, query })}
      />
      <Select
        label={message("capabilities.group")}
        options={[
          { value: "all", label: message("capabilities.group.all") },
          ...matrix.groups.map((g) => ({ value: g.id, label: `${g.id} · ${g.name}` })),
        ]}
        value={filter.group}
        onChange={(group) => onChange({ ...filter, group })}
      />
      {/* biome-ignore lint/a11y/useSemanticElements: a group of toggle buttons, not one form value. */}
      <div className="paper-capability-show" role="group" aria-label={message("capabilities.show")}>
        <span className="paper-field-label" aria-hidden="true">
          {message("capabilities.show")}
        </span>
        {SURFACES.map((s) => (
          <ToggleButton
            key={s}
            isSelected={filter.surfaces.includes(s)}
            onChange={(on) =>
              onChange({ ...filter, surfaces: SURFACES.filter((x) => (x === s ? on : filter.surfaces.includes(x))) })
            }
          >
            {message(SURFACE_LABEL[s])}
          </ToggleButton>
        ))}
      </div>
    </search>
  );
}

/** The table: a row group per group of operations, a column per shown surface. */
function MatrixTable({
  groups,
  surfaces,
  caption,
}: {
  groups: readonly CapabilityGroup[];
  surfaces: readonly CapabilitySurface[];
  caption: string;
}): ReactNode {
  return (
    <div className="paper-capability-table-box">
      <table className="paper-capability-table">
        <caption>{caption}</caption>
        <thead>
          <tr>
            <th scope="col">{message("capabilities.column.id")}</th>
            <th scope="col">{message("capabilities.column.operation")}</th>
            {surfaces.map((s) => (
              <th key={s} scope="col">
                {message(SURFACE_LABEL[s])}
              </th>
            ))}
            <th scope="col">{message("capabilities.column.target")}</th>
          </tr>
        </thead>
        {groups.map((group) => (
          <tbody key={group.id}>
            <tr>
              <th scope="rowgroup" colSpan={surfaces.length + 3} className="paper-capability-group">
                {`${group.id} · ${group.name}`}
              </th>
            </tr>
            {group.ops.map((op) => (
              <tr key={op.id}>
                <td className="paper-capability-id">{op.id}</td>
                <th scope="row">{op.name}</th>
                {surfaces.map((s) => (
                  <td key={s}>
                    <StatusBadge status={op.surfaces[s]} />
                  </td>
                ))}
                <td>{message("capabilities.target", { round: op.round, status: message(STATUS_LABEL[op.target]) })}</td>
              </tr>
            ))}
          </tbody>
        ))}
      </table>
    </div>
  );
}

/** The capability page, filling the window. */
export function CapabilityPage({ matrix, version, backLabel, onBack, onHowTested }: CapabilityPageProps): ReactNode {
  const [filter, setFilter] = useStoredFilter(matrix);
  const headingId = useId();
  const total = matrix.groups.reduce((n, g) => n + g.ops.length, 0);
  const shown = filterMatrix(matrix, filter);
  const caption = message("capabilities.caption", {
    shown: shown.ops,
    total,
    groups: countMessage(shown.groups.length, { one: "capabilities.groups.one", many: "capabilities.groups.many" }),
  });
  return (
    <div className="paper-capability-page">
      <header className="paper-capability-bar">
        <Button variant="ghost" icon="back" onPress={onBack}>
          {backLabel}
        </Button>
      </header>
      <main className="paper-capability-content" aria-labelledby={headingId}>
        <h1 id={headingId} className="paper-capability-heading">
          {message("capabilities.title")}
        </h1>
        <p className="paper-capability-lede">{message("capabilities.lede")}</p>
        <dl className="paper-capability-legend">
          {(["round-trip", "write", "read", "unsupported"] as const).map((status) => (
            <div key={status}>
              <dt>
                <StatusBadge status={status} />
              </dt>
              <dd>{message(`${STATUS_LABEL[status]}.means` as MessageId)}</dd>
            </div>
          ))}
        </dl>
        <Filters matrix={matrix} filter={filter} onChange={setFilter} />
        {shown.ops === 0 ? (
          <EmptyState
            title={message("capabilities.empty", { query: filter.query.trim() })}
            actions={<Button onPress={() => setFilter(NO_FILTER)}>{message("capabilities.clear")}</Button>}
          />
        ) : (
          <MatrixTable
            groups={shown.groups}
            surfaces={SURFACES.filter((s) => filter.surfaces.includes(s))}
            caption={caption}
          />
        )}
        <p className="paper-capability-provenance">
          {message("capabilities.provenance", { version })}{" "}
          <Button variant="ghost" size={26} onPress={onHowTested}>
            {message("capabilities.howTested")}
          </Button>
        </p>
      </main>
    </div>
  );
}
