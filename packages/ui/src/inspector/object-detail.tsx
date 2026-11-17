// The object inspector's detail (O15): where the object sits (a breadcrumb), its title and context
// line (its page, the revision that last wrote it, the object stream that holds it), Show page and
// Copy as text, its keys as a table whose references are links, and what refers to it. A
// reference to an object the file doesn't have shows in the danger colour, "14 0 R · missing".

import type { ReactNode } from "react";
import { Button } from "../components/button.js";
import { laterMessage } from "../later-messages.js";
import { ReadFailed } from "./read-failed.js";
import {
  type InspectEntries,
  type InspectObjectData,
  type InspectReferrer,
  type InspectValue,
  objectKind,
  type ReadState,
  refText,
  typeLabel,
} from "./values.js";

/** Props of `ObjectDetail`. */
export interface ObjectDetailProps {
  /** The object shown. */
  readonly object: InspectObjectData;
  /** The objects known so far, by number: for the words about each reference. */
  readonly entries: InspectEntries;
  /** The revisions the file has. */
  readonly revisions: number;
  /** The path to it in the tree: each step's id and label. */
  readonly path: readonly { readonly id: string; readonly label: string }[];
  /** A step of the path was chosen. */
  readonly onPath: (id: string) => void;
  /** A reference was chosen: go to that object. */
  readonly onObject: (num: number) => void;
  /** The 0-based page the object is, if it is one. */
  readonly page: number | undefined;
  /** Show page: back to the pages, at this one. */
  readonly onShowPage: (page: number) => void;
  /** Copy as text. */
  readonly onCopy: () => void;
  /** What refers to it, once read. */
  readonly referrers: ReadState<readonly InspectReferrer[]> | undefined;
  /** Read what refers to it again, after a failure. */
  readonly onRetryReferrers: () => void;
  /** The stream section, for a stream (or a page's content stream). */
  readonly stream?: ReactNode;
}

/** The most items of an array shown in a table cell; the rest are counted. */
const MAX_INLINE_ITEMS = 12;

/** A reference as a link, or in the danger colour when the file has no such object. */
function RefLink({
  num,
  generation,
  entries,
  onObject,
}: {
  num: number;
  generation: number;
  entries: InspectEntries;
  onObject: (num: number) => void;
}): ReactNode {
  const text = refText(num, generation);
  if (entries.get(num) === null) {
    return <span className="paper-object-missing">{`${text} · ${laterMessage("inspect.kind.missing")}`}</span>;
  }
  return (
    <button type="button" className="paper-object-link" onClick={() => onObject(num)}>
      {text}
    </button>
  );
}

/** A value in a table cell: references as links, arrays with their first items. */
function ValueCell({
  value,
  entries,
  onObject,
}: {
  value: InspectValue;
  entries: InspectEntries;
  onObject: (num: number) => void;
}): ReactNode {
  if (value.type === "reference") {
    return <RefLink num={value.num} generation={value.generation} entries={entries} onObject={onObject} />;
  }
  if (value.type === "array" && value.items.some((item) => item.type === "reference")) {
    const shown = value.items.slice(0, MAX_INLINE_ITEMS);
    const more = value.items.length - shown.length;
    return (
      <>
        {"["}
        {shown.map((item, i) => (
          // biome-ignore lint/suspicious/noArrayIndexKey: an item's place is its identity in the array.
          <span key={i} className="paper-object-item-gap">
            <ValueCell value={item} entries={entries} onObject={onObject} />
          </span>
        ))}
        {more > 0 ? ` ${laterMessage("inspect.more", { count: more })}` : null}
        {"]"}
      </>
    );
  }
  return <>{inlineText(value)}</>;
}

/** A value with no references on one line. */
function inlineText(value: InspectValue): string {
  switch (value.type) {
    case "string":
      return `(${value.text})`;
    case "name":
      return `/${value.value}`;
    case "array":
      return `[${value.items.map(inlineText).join(" ")}]`;
    case "dictionary":
      return `<< ${value.entries.map(([key]) => `/${key}`).join(" ")} >>`;
    case "reference":
      return refText(value.num, value.generation);
    case "null":
      return "null";
    default:
      return String(value.value);
  }
}

/** The rows of the keys table: a dictionary's entries, an array's items, or the one value. */
function keyRows(value: InspectValue): readonly (readonly [string, InspectValue])[] {
  if (value.type === "dictionary") return value.entries.map(([key, item]) => [`/${key}`, item] as const);
  if (value.type === "array") return value.items.map((item, i) => [`[${i}]`, item] as const);
  return [[laterMessage("inspect.value"), value]];
}

/** The context line: its page, the revision that last wrote it, where it is stored. */
function contextLine(object: InspectObjectData, revisions: number, page: number | undefined): string {
  const parts: string[] = [];
  if (page !== undefined) parts.push(laterMessage("inspect.page", { page: page + 1 }));
  if (object.revision !== undefined) {
    parts.push(laterMessage("inspect.lastChanged", { revision: object.revision + 1, count: revisions }));
  }
  const at = object.entry.location;
  if (at.kind === "objectStream") parts.push(laterMessage("inspect.inObjectStream", { stream: at.stream }));
  if (at.kind === "offset") parts.push(laterMessage("inspect.atOffset", { offset: at.offset.toLocaleString("en") }));
  if (at.kind === "created") parts.push(laterMessage("inspect.created"));
  return parts.join(" · ");
}

/** What refers to the object, each with the object's kind and the key that holds the reference. */
function ReferencedBy({
  num,
  referrers,
  entries,
  onObject,
  onRetry,
}: {
  num: number;
  referrers: ReadState<readonly InspectReferrer[]> | undefined;
  entries: InspectEntries;
  onObject: (num: number) => void;
  onRetry: () => void;
}): ReactNode {
  const id = `inspect-refby-${num}`;
  const list = referrers?.status === "ready" ? referrers.value : undefined;
  return (
    <section className="paper-inspect-section" aria-labelledby={id}>
      <h3 id={id} className="paper-inspect-heading">
        {laterMessage("inspect.referencedBy")}
      </h3>
      {referrers?.status === "failed" ? (
        <ReadFailed reason={referrers.reason} onRetry={onRetry} />
      ) : list === undefined ? (
        <p className="paper-inspect-note">{laterMessage("inspect.referencedBy.reading")}</p>
      ) : list.length === 0 ? (
        <p className="paper-inspect-note">{laterMessage("inspect.referencedBy.none")}</p>
      ) : (
        <ul className="paper-inspect-referrers">
          {list.map((r) => (
            <li key={`${r.from ?? "trailer"}${r.path}`}>
              {r.from === undefined ? (
                <span className="paper-object-mono">{laterMessage("inspect.trailer")}</span>
              ) : (
                <RefLink
                  num={r.from}
                  generation={entries.get(r.from)?.generation ?? 0}
                  entries={entries}
                  onObject={onObject}
                />
              )}
              <span className="paper-inspect-note">
                {` · ${r.from === undefined ? "" : `${objectKind(entries.get(r.from))}, `}${r.path}`}
              </span>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

/** One object: its path, title and context, its keys, what refers to it and its stream. */
export function ObjectDetail(props: ObjectDetailProps): ReactNode {
  const { object, entries, page } = props;
  const { num } = object.entry;
  return (
    <div className="paper-inspect-detail">
      <nav aria-label={laterMessage("inspect.path")}>
        <ol className="paper-inspect-path">
          {props.path.map((step, i) => (
            <li key={step.id} {...(i === props.path.length - 1 ? { "aria-current": "page" } : {})}>
              {i === props.path.length - 1 ? (
                step.label
              ) : (
                <button type="button" className="paper-object-link" onClick={() => props.onPath(step.id)}>
                  {step.label}
                </button>
              )}
            </li>
          ))}
        </ol>
      </nav>
      <div className="paper-inspect-header">
        <div className="paper-inspect-title">
          <h2>{laterMessage("inspect.title", { num, kind: objectKind(object.entry) })}</h2>
          <span className="paper-inspect-note">{contextLine(object, props.revisions, page)}</span>
        </div>
        <div className="paper-inspect-actions">
          {page === undefined ? null : (
            <Button size={30} onPress={() => props.onShowPage(page)}>
              {laterMessage("inspect.showPage")}
            </Button>
          )}
          <Button size={30} onPress={props.onCopy}>
            {laterMessage("inspect.copy")}
          </Button>
        </div>
      </div>
      <table className="paper-inspect-table paper-inspect-keys">
        <caption className="paper-inspect-hidden">{laterMessage("inspect.keysOf", { num })}</caption>
        <thead>
          <tr>
            <th scope="col">{laterMessage("inspect.key")}</th>
            <th scope="col">{laterMessage("inspect.value")}</th>
            <th scope="col">{laterMessage("inspect.type")}</th>
          </tr>
        </thead>
        <tbody>
          {keyRows(object.value).map(([key, value]) => (
            <tr key={key}>
              <td className="paper-object-mono">{key}</td>
              <td className="paper-object-mono">
                <ValueCell value={value} entries={entries} onObject={props.onObject} />
              </td>
              <td>{typeLabel(value)}</td>
            </tr>
          ))}
        </tbody>
      </table>
      <ReferencedBy
        num={num}
        referrers={props.referrers}
        entries={entries}
        onObject={props.onObject}
        onRetry={props.onRetryReferrers}
      />
      {props.stream}
    </div>
  );
}
