// The object inspector's tree (O15): the objects from the trailer down, as a WAI-ARIA tree on
// React Aria's Tree (arrow keys move, open and close; Home and End; typing finds a row), always
// virtualised, since a document can have hundreds of thousands of objects. A row's children are
// built only when it opens, and a reference's when its object has been read, so a cycle in the
// file (a page's /Parent) costs nothing until someone opens it.

import { Icon } from "@laminapdf/icons/react";
import type { ReactNode } from "react";
import {
  Button as AriaButton,
  Collection,
  type Key,
  ListLayout,
  Tree,
  TreeItem,
  TreeItemContent,
  Virtualizer,
} from "react-aria-components";
import { laterMessage } from "../later-messages.js";
import type { TreeRow } from "./values.js";

/** Props of `ObjectTree`. */
export interface ObjectTreeProps {
  /** The roots: the trailer, then each object gone to by number. */
  readonly rows: readonly TreeRow[];
  /** The rows that are open, by id. */
  readonly expanded: ReadonlySet<string>;
  /** The open rows changed. */
  readonly onExpandedChange: (expanded: ReadonlySet<string>) => void;
  /** The selected row's id. */
  readonly selected: string | undefined;
  /** A row was chosen (click, tap, Enter or the arrow keys). */
  readonly onSelect: (id: string) => void;
}

/** A row's height for the virtualiser, in CSS px: the tree row's target size. */
const ROW_HEIGHT = 28;

/** What a row shows: the chevron, its key or index, and what it is. */
function RowContent({
  row,
  open,
  hasChildren,
  level,
}: {
  row: TreeRow;
  open: boolean;
  hasChildren: boolean;
  level: number;
}): ReactNode {
  return (
    <div
      className="paper-object-row"
      data-loading={row.loading === true || undefined}
      // Each level steps in by 16 px, as the board's rows do.
      style={{ paddingInlineStart: `calc(${level - 1} * var(--paper-space-16))` }}
    >
      {hasChildren ? (
        <AriaButton slot="chevron" className="paper-object-toggle" excludeFromTabOrder>
          <Icon name={open ? "chevron-down" : "chevron-right"} />
        </AriaButton>
      ) : (
        <span className="paper-object-toggle" aria-hidden="true" />
      )}
      <span className="paper-object-key">{row.label}</span>
      {row.description === "" ? null : <span className="paper-object-note">{row.description}</span>}
    </div>
  );
}

/** One row, then its children. */
function Item({ row }: { row: TreeRow }): ReactNode {
  return (
    <TreeItem id={row.id} textValue={row.label} className="paper-object-item">
      <TreeItemContent>
        {({ hasChildItems, isExpanded, level }) => (
          <RowContent row={row} open={isExpanded} hasChildren={hasChildItems} level={level} />
        )}
      </TreeItemContent>
      {row.children === undefined ? null : (
        <Collection items={row.children}>{(child) => <Item row={child} />}</Collection>
      )}
    </TreeItem>
  );
}

/** The object tree, in the inspector's rail. */
export function ObjectTree({ rows, expanded, onExpandedChange, selected, onSelect }: ObjectTreeProps): ReactNode {
  return (
    <Virtualizer layout={ListLayout} layoutOptions={{ rowHeight: ROW_HEIGHT }}>
      <Tree
        aria-label={laterMessage("inspect.tree")}
        className="paper-object-tree"
        items={rows}
        expandedKeys={expanded as Set<Key>}
        onExpandedChange={(keys) => onExpandedChange(new Set([...keys].map(String)))}
        selectionMode="single"
        disallowEmptySelection
        selectedKeys={selected === undefined ? [] : [selected]}
        onSelectionChange={(keys) => {
          const [key] = keys === "all" ? [] : [...keys];
          if (key !== undefined) onSelect(String(key));
        }}
      >
        {(row) => <Item row={row} />}
      </Tree>
    </Virtualizer>
  );
}
