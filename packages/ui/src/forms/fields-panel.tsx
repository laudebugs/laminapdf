// The Fields rail tab (Paper P01): one row per field in the form's tab order, grouped by page —
// its tab stop, its name (the accessible label, else the field name) and its state (filled,
// required, locked). The page being filled shows its rows; other pages fold to one line until
// opened. A row focuses its field. It is a `nav` of links, the focused field's marked current.

import { type ReactNode, useState } from "react";
import { Button } from "../components/button.js";
import { Notice } from "../components/notice.js";
import { laterMessage } from "../later-messages.js";

/** One row of the Fields tab. */
export interface FieldRow {
  /** The widget the row focuses. */
  readonly widget: string;
  /** Its field's ID. */
  readonly field: string;
  /** The 0-based page that shows it. */
  readonly page: number;
  /** Its name as the person reads it. */
  readonly label: string;
  readonly filled: boolean;
  readonly required: boolean;
  /** Why it can't change, when a signature locks it. */
  readonly locked?: string;
}

/** What the last import left to say in the Fields tab (P03): a notice with its own dismiss. */
export interface FieldsNote {
  readonly tone: "neutral" | "warning";
  readonly title: string;
  readonly body: string;
}

/** Props of `FieldsPanel`. */
export interface FieldsPanelProps {
  /** A notice above the rows: what an import did or didn't match. */
  readonly note?: FieldsNote | undefined;
  /** The notice was dismissed. */
  readonly onDismissNote?: () => void;
  /** The rows, in tab order. */
  readonly rows: readonly FieldRow[];
  /** The field focused now. */
  readonly current: string | undefined;
  /** The page being filled: its rows show open. */
  readonly page: number;
  /** Page `index`'s name, as the page indicator shows it. */
  readonly pageName: (index: number) => string;
  /** A row was chosen: focus its widget. */
  readonly onFocus: (row: FieldRow) => void;
}

/** A row's state, in words. */
function wordsOf(row: FieldRow): string {
  if (row.locked !== undefined) return row.locked;
  if (row.filled) return laterMessage("form.filled");
  return row.required ? laterMessage("form.required") : laterMessage("form.empty");
}

/** A row's state, as its `data-state` says it for the styles. */
function stateKey(row: FieldRow): "locked" | "filled" | "required" | "empty" {
  if (row.locked !== undefined) return "locked";
  if (row.filled) return "filled";
  return row.required ? "required" : "empty";
}

/** One field's row: a link that focuses it. */
function Row({
  row,
  stop,
  current,
  onFocus,
}: {
  row: FieldRow;
  stop: number;
  current: boolean;
  onFocus: (row: FieldRow) => void;
}): ReactNode {
  return (
    <a
      href={`#${row.widget}`}
      className="paper-form-field-row"
      data-state={stateKey(row)}
      {...(current ? { "aria-current": "true" as const } : {})}
      onClick={(e) => {
        e.preventDefault();
        onFocus(row);
      }}
    >
      <span className="paper-form-field-stop">{stop}</span>
      <span className="paper-form-field-label">{row.label}</span>
      <span className="paper-form-field-state">{wordsOf(row)}</span>
    </a>
  );
}

/** The Fields tab's panel. */
export function FieldsPanel({
  rows,
  current,
  page,
  pageName,
  onFocus,
  note,
  onDismissNote,
}: FieldsPanelProps): ReactNode {
  const [opened, setOpened] = useState<ReadonlySet<number>>(() => new Set());
  const notice =
    note === undefined ? null : (
      <div className="paper-form-fields-note">
        <Notice tone={note.tone} title={note.title}>
          {note.body}
        </Notice>
        {onDismissNote === undefined ? null : (
          <Button size={26} onPress={onDismissNote}>
            {laterMessage("form.import.dismiss")}
          </Button>
        )}
      </div>
    );
  if (rows.length === 0) {
    return (
      <div className="paper-form-fields-empty">
        <p className="paper-form-fields-empty-title">{laterMessage("form.fields.empty")}</p>
        <p>{laterMessage("form.fields.emptyHelp")}</p>
      </div>
    );
  }
  const pages = [...new Set(rows.map((row) => row.page))];
  return (
    <nav className="paper-form-fields" aria-label={laterMessage("form.fields")}>
      {notice}
      {pages.map((index) => {
        const own = rows.filter((row) => row.page === index);
        const name = pageName(index);
        return (
          <section key={index} aria-label={laterMessage("form.fields.page", { page: name })}>
            <h4 className="paper-form-fields-page">{laterMessage("form.fields.page", { page: name })}</h4>
            {index === page || opened.has(index) || own.some((row) => row.field === current) ? (
              <ol>
                {own.map((row) => (
                  <li key={row.widget}>
                    <Row row={row} stop={rows.indexOf(row) + 1} current={row.field === current} onFocus={onFocus} />
                  </li>
                ))}
              </ol>
            ) : (
              <button
                type="button"
                className="paper-form-fields-more"
                onClick={() => setOpened((was) => new Set([...was, index]))}
              >
                {laterMessage("form.fields.morePages", { page: name })}
              </button>
            )}
          </section>
        );
      })}
    </nav>
  );
}
