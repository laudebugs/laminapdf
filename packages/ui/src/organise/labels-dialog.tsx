// The page labels editor (Paper O10): a table of ranges, one row each (From page · Style · Prefix ·
// Start at · remove), and a preview of the labels they give. The first row always starts on
// page 1. Apply turns the whole table into one labels op, so it is one undo step.

import { type ReactNode, useEffect, useRef, useState } from "react";
import { Button, IconButton } from "../components/button.js";
import { laterMessage } from "../later-messages.js";
import { Dialog } from "../overlays/dialog.js";
import {
  isValid,
  LABEL_STYLES,
  type LabelProblems,
  type LabelRangeValue,
  type LabelRow,
  type LabelStyleName,
  labelsOf,
  type PreviewEntry,
  previewOf,
  problemsOf,
  rangesOf,
  row,
  rowsOf,
  sharedLabel,
  startsOf,
} from "./labels.js";

/** Props of `PageLabelsDialog`. */
export interface PageLabelsDialogProps {
  readonly isOpen: boolean;
  readonly onClose: () => void;
  /** Every page's ID, in order. */
  readonly pages: readonly string[];
  /** The document's label ranges now (empty without labels). */
  readonly ranges: readonly LabelRangeValue[];
  /** Applies the table's ranges, in page order. */
  readonly onApply: (ranges: LabelRangeValue[]) => void;
}

const COLUMNS = ["from", "style", "prefix", "start"] as const;

/** One cell's name: "Range 3, prefix". */
const cellName = (n: number, column: (typeof COLUMNS)[number]) =>
  laterMessage("labels.field", { n, column: laterMessage(`labels.column.${column}`).toLowerCase() });

/** What is wrong with row `r`, as its error message. */
function errorOf(r: LabelRow, problems: LabelProblems, count: number): string | undefined {
  if (problems.clash?.keys.has(r.key)) return laterMessage("labels.clash", { page: problems.clash.page });
  if (problems.outside.has(r.key)) return laterMessage("labels.outside", { count });
  if (problems.badStart.has(r.key)) return laterMessage("labels.badStart");
  return undefined;
}

/** One range's row: From page (fixed on the first), Style, Prefix, Start at, and Remove. */
function RangeRow(props: {
  row: LabelRow;
  n: number;
  problems: LabelProblems;
  count: number;
  onChange: (patch: Partial<Omit<LabelRow, "key">>) => void;
  onRemove: () => void;
  onFocus: () => void;
}): ReactNode {
  const { row: r, n, problems } = props;
  const first = n === 1;
  const error = errorOf(r, problems, props.count);
  const described = error === undefined ? undefined : `labels-error-${r.key}`;
  const style = useRef<HTMLSelectElement>(null);
  useEffect(() => {
    // Focus starts on the first editable field: the first row's style.
    if (first) style.current?.focus();
  }, [first]);
  return (
    <tr data-row={r.key} data-invalid={error === undefined ? undefined : true}>
      <td>
        <input
          className="paper-input paper-labels-number"
          name="from"
          inputMode="numeric"
          aria-label={cellName(n, "from")}
          aria-invalid={problems.outside.has(r.key) || problems.clash?.keys.has(r.key) || undefined}
          aria-describedby={described}
          value={r.from}
          disabled={first}
          onFocus={props.onFocus}
          onChange={(e) => props.onChange({ from: e.target.value })}
        />
      </td>
      <td>
        <span className="paper-select">
          <select
            ref={style}
            className="paper-input"
            aria-label={cellName(n, "style")}
            value={r.style}
            onFocus={props.onFocus}
            onChange={(e) => props.onChange({ style: e.target.value as LabelStyleName })}
          >
            {LABEL_STYLES.map((s) => (
              <option key={s} value={s}>
                {laterMessage(`labels.style.${s}`)}
              </option>
            ))}
          </select>
        </span>
      </td>
      <td>
        <input
          className="paper-input"
          aria-label={cellName(n, "prefix")}
          value={r.prefix}
          onFocus={props.onFocus}
          onChange={(e) => props.onChange({ prefix: e.target.value })}
        />
      </td>
      <td>
        <input
          className="paper-input paper-labels-number"
          inputMode="numeric"
          aria-label={cellName(n, "start")}
          aria-invalid={problems.badStart.has(r.key) || undefined}
          aria-describedby={described}
          value={r.start}
          disabled={r.style === "none"}
          onFocus={props.onFocus}
          onChange={(e) => props.onChange({ start: e.target.value })}
        />
        {error === undefined ? null : (
          <span className="paper-labels-error" id={described}>
            {error}
          </span>
        )}
      </td>
      <td>
        {first ? null : (
          <IconButton icon="close" size={26} label={laterMessage("labels.remove", { n })} onPress={props.onRemove} />
        )}
      </td>
    </tr>
  );
}

/** The preview: page → label pairs, the range being edited marked, and the runs left out named. */
function LabelsPreview({ preview, editing }: { preview: readonly PreviewEntry[]; editing: number | undefined }) {
  return (
    <ol className="paper-labels-preview" aria-label={laterMessage("labels.preview")}>
      {preview.map((entry) =>
        entry.kind === "page" ? (
          <li key={`p${entry.page}`} data-editing={entry.key !== undefined && entry.key === editing ? true : undefined}>
            <span className="paper-labels-page">{entry.page}</span>
            <span aria-hidden="true"> → </span>
            <span className="paper-labels-label">{entry.label}</span>
          </li>
        ) : (
          <li key={`g${entry.from}`} className="paper-labels-gap">
            {laterMessage("labels.gap", {
              from: entry.from,
              to: entry.to,
              first: entry.firstLabel,
              last: entry.lastLabel,
            })}
          </li>
        ),
      )}
    </ol>
  );
}

/** The table's rows as state, filled from the document's ranges each time a read of them arrives. */
function useRows(isOpen: boolean, ranges: readonly LabelRangeValue[], pages: readonly string[]) {
  const [rows, setRows] = useState<LabelRow[]>([]);
  const added = useRef<number | undefined>(undefined);
  const table = useRef<HTMLTableElement>(null);
  // Filled when it opens with a read, and only then: a re-render of the reader keeps what was typed.
  const [seeded, setSeeded] = useState<readonly LabelRangeValue[] | undefined>(undefined);
  const now = isOpen ? ranges : undefined;
  if (now !== seeded) {
    setSeeded(now);
    if (now !== undefined) setRows(rowsOf(now, pages));
  }
  useEffect(() => {
    // After Add range, focus goes to the new row's From field.
    if (added.current === undefined) return;
    table.current?.querySelector<HTMLInputElement>(`[data-row="${added.current}"] input[name="from"]`)?.focus();
    added.current = undefined;
  });
  const add = () => {
    const last = Number(rows.at(-1)?.from ?? "1");
    const next = row(Math.min(pages.length, (Number.isFinite(last) ? last : 1) + 1));
    added.current = next.key;
    setRows((all) => [...all, next]);
  };
  const change = (key: number, patch: Partial<Omit<LabelRow, "key">>) =>
    setRows((all) => all.map((r) => (r.key === key ? { ...r, ...patch } : r)));
  const remove = (key: number) => setRows((all) => all.filter((r) => r.key !== key));
  return { rows, table, add, change, remove };
}

/** The labels editor. */
export function PageLabelsDialog(props: PageLabelsDialogProps): ReactNode {
  const { pages } = props;
  const { rows, table, add, change, remove } = useRows(props.isOpen, props.ranges, pages);
  const [editing, setEditing] = useState<number | undefined>(undefined);
  const count = pages.length;
  const problems = problemsOf(rows, count);
  const labels = labelsOf(rows, count);
  const shared = sharedLabel(labels.map((l) => l.label));
  return (
    <Dialog
      isOpen={props.isOpen}
      onOpenChange={(open) => {
        if (!open) props.onClose();
      }}
      title={laterMessage("labels.title")}
      closeLabel={laterMessage("labels.close")}
      size="wide"
      actions={
        <>
          <Button onPress={props.onClose}>{laterMessage("labels.cancel")}</Button>
          <Button
            variant="primary"
            isDisabled={!isValid(problems)}
            onPress={() => {
              props.onApply(rangesOf(rows, pages));
              props.onClose();
            }}
          >
            {laterMessage("labels.apply")}
          </Button>
        </>
      }
    >
      <p className="paper-labels-intro">{laterMessage("labels.intro")}</p>
      <table className="paper-labels-table" ref={table}>
        <thead>
          <tr>
            {COLUMNS.map((c) => (
              <th key={c} scope="col">
                {laterMessage(`labels.column.${c}`)}
              </th>
            ))}
            <th scope="col">
              <span className="paper-visually-hidden">{laterMessage("labels.column.remove")}</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {rows.map((r, i) => (
            <RangeRow
              key={r.key}
              row={r}
              n={i + 1}
              problems={problems}
              count={count}
              onChange={(patch) => change(r.key, patch)}
              onRemove={() => remove(r.key)}
              onFocus={() => setEditing(r.key)}
            />
          ))}
        </tbody>
      </table>
      <Button variant="ghost" icon="plus" onPress={add}>
        {laterMessage("labels.add")}
      </Button>
      <LabelsPreview preview={previewOf(labels, startsOf(rows, count))} editing={editing} />
      {shared === undefined ? null : (
        <p className="paper-labels-shared">
          {laterMessage("labels.shared", { first: shared.first, second: shared.second, label: shared.label })}
        </p>
      )}
    </Dialog>
  );
}
