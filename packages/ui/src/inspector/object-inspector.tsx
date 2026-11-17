// The object inspector (O15): a full-canvas view in place of the pages until Back to pages, for
// power users and developers. A view bar takes the secondary toolbar's place (Back to pages, the
// Objects · Revisions · Fonts · Colour · Conformance tabs, the Read only badge and the file's
// version and counts); on Objects, a 300 px column holds Go to object and the object tree, and the
// main area the selected object's detail. Nothing here changes the file. Everything is read when
// it is first shown: the app reads the document (`onRead…`); this component holds only what the
// view shows, and each failed read offers Try again.

import { Icon } from "@laminapdf/icons/react";
import { type ReactNode, useCallback, useEffect, useMemo, useState } from "react";
import { Tab, TabList, TabPanel, Tabs } from "react-aria-components";
import { Badge } from "../components/badge.js";
import { TextInput } from "../components/field.js";
import { StableLabel } from "../components/stable-label.js";
import { laterCountMessage, laterMessage } from "../later-messages.js";
import {
  ColourTables,
  ConformanceFacts,
  FontsTable,
  type InspectColour,
  type InspectConformance,
  type InspectFont,
  type InspectRevision,
  RevisionsTable,
} from "./facts.js";
import { ObjectDetail } from "./object-detail.js";
import { ObjectTree } from "./object-tree.js";
import { ReadFailed } from "./read-failed.js";
import { StreamBlock, type StreamMode, type StreamState } from "./stream-block.js";
import {
  type InspectEntries,
  type InspectObjectData,
  type InspectReferrer,
  type InspectValue,
  pathOf,
  type ReadState,
  type TreeRow,
  treeRows,
} from "./values.js";

/** The inspector's tabs. */
export type InspectTab = "objects" | "revisions" | "fonts" | "colour" | "conformance";

/** The tabs whose facts are read when first shown. */
export type InspectFactsTab = Exclude<InspectTab, "objects">;

/** What the inspector reads as it opens: nothing that reads every object. */
export interface InspectSummary {
  /** The trailer, the tree's root. */
  readonly trailer: InspectValue;
  /** The version the file declares ("1.7"). */
  readonly version: string | undefined;
  /** How many objects exist. */
  readonly objects: number;
  /** How many revisions the file has. */
  readonly revisions: number;
}

/** What the inspector is asked to show ([`ObjectInspectorProps.request`]). */
export interface InspectRequest {
  /** The tab to show. */
  readonly tab: InspectTab;
  /** An object to go to, on Objects. */
  readonly goTo?: number | undefined;
  /** Counts the requests, so the same one asked again shows again. */
  readonly seq: number;
}

/** Each tab's facts, read when it is first shown. */
export interface InspectTabs {
  /** The Revisions tab's. */
  readonly revisions?: ReadState<readonly InspectRevision[]> | undefined;
  /** The Fonts tab's. */
  readonly fonts?: ReadState<readonly InspectFont[]> | undefined;
  /** The Colour tab's. */
  readonly colour?: ReadState<InspectColour> | undefined;
  /** The Conformance tab's. */
  readonly conformance?: ReadState<InspectConformance> | undefined;
}

/** Props of `ObjectInspector`. */
export interface ObjectInspectorProps {
  /** What the document is; the view says it is reading until then. */
  readonly summary: ReadState<InspectSummary>;
  /** Read the summary again, after a failure. */
  readonly onRetrySummary: () => void;
  /** The objects known so far, by number: their entries, or `null` for one the file lacks. */
  readonly entries: InspectEntries;
  /** The objects read or being read, by number. */
  readonly objects: ReadonlyMap<number, ReadState<InspectObjectData>>;
  /** Read object `num` (again, after a failure). */
  readonly onRead: (num: number) => void;
  /** What refers to each object, by number, once asked for. */
  readonly referrers: ReadonlyMap<number, ReadState<readonly InspectReferrer[]>>;
  /** Find what refers to object `num` (again, after a failure). */
  readonly onReadReferrers: (num: number) => void;
  /** Each tab's facts. */
  readonly tabs: InspectTabs;
  /** Read tab `tab`'s facts (again, after a failure). */
  readonly onReadTab: (tab: InspectFactsTab) => void;
  /** What was read of each stream, by `num:decoded` or `num:raw`. */
  readonly streams: ReadonlyMap<string, StreamState>;
  /** Read the first bytes of stream `num`, decoded or raw. */
  readonly onReadStream: (num: number, view: "decoded" | "raw") => void;
  /** The 0-based page an object is, if it is one. */
  readonly pageOf: (num: number) => number | undefined;
  /**
   * What it was last asked to show: a tab, and an object to go to ("Go to object…"). A new
   * `seq` shows them again while the inspector is open (Inspect objects asked twice).
   */
  readonly request?: InspectRequest | undefined;
  /** Back to pages. */
  readonly onBack: () => void;
  /** Show page: back to the pages, at page `page`. */
  readonly onShowPage: (page: number) => void;
  /** Copy object `num` as text. */
  readonly onCopy: (num: number) => void;
  /** Save stream `num`'s raw bytes. */
  readonly onSaveRaw: (num: number) => void;
}

/** The lines a stream block shows first, and how many more Show more adds. */
const FIRST_LINES = 16;
const MORE_LINES = 200;

const TABS: readonly InspectTab[] = ["objects", "revisions", "fonts", "colour", "conformance"];

/** The value of `state` once read. */
function ready<T>(state: ReadState<T> | undefined): T | undefined {
  return state?.status === "ready" ? state.value : undefined;
}

/** The stream to show with object `object`: itself, or a page's first content stream. */
function streamOf(object: InspectObjectData, entries: InspectEntries): number | undefined {
  if (object.stream !== undefined) return object.entry.num;
  if (object.entry.typeName !== "Page" || object.value.type !== "dictionary") return undefined;
  const contents = object.value.entries.find(([key]) => key === "Contents")?.[1];
  const first = contents?.type === "array" ? contents.items[0] : contents;
  return first?.type === "reference" && entries.get(first.num)?.kind === "stream" ? first.num : undefined;
}

/** The view bar's summary: "PDF 1.7 · 412 objects · 2 revisions". */
function summaryText(summary: InspectSummary | undefined): string {
  if (summary === undefined) return laterMessage("inspect.loading");
  return laterMessage("inspect.summary", {
    version: summary.version ?? "?",
    objects: summary.objects.toLocaleString("en"),
    revisions: laterCountMessage(Math.max(summary.revisions, 1), {
      one: "inspect.revisions.one",
      many: "inspect.revisions.many",
    }),
  });
}

/** The state the Objects tab keeps: open rows, the selection, objects gone to, the stream's view. */
function useObjectsView(props: ObjectInspectorProps) {
  const { objects, onRead, entries } = props;
  const trailer = ready(props.summary)?.trailer;
  // The catalog and the page tree open at first, as the board draws them.
  const [expanded, setExpanded] = useState<ReadonlySet<string>>(
    () => new Set(["trailer", "trailer//Root", "trailer//Root//Pages"]),
  );
  const [selected, setSelected] = useState<string | undefined>("trailer");
  const [goneTo, setGoneTo] = useState<readonly number[]>([]);
  const [mode, setMode] = useState<StreamMode>("decoded");
  const [lines, setLines] = useState(FIRST_LINES);
  const values = useMemo(() => {
    const out = new Map<number, InspectValue>();
    for (const [num, state] of objects) if (state.status === "ready") out.set(num, state.value.value);
    return out;
  }, [objects]);
  const rows: readonly TreeRow[] = useMemo(
    () => treeRows(trailer, goneTo, { entries, values, pageOf: props.pageOf, expanded }),
    [trailer, goneTo, entries, values, props.pageOf, expanded],
  );
  const path = selected === undefined ? [] : pathOf(selected, rows);
  const object = path.at(-1)?.object;
  // An open reference and the selected object are read as they're needed.
  useEffect(() => {
    for (const id of [...expanded, selected]) {
      const num = id === undefined ? undefined : pathOf(id, rows).at(-1)?.object;
      if (num !== undefined && !objects.has(num) && entries.get(num) !== null) onRead(num);
    }
  }, [expanded, selected, rows, objects, entries, onRead]);
  const goTo = useCallback((num: number) => {
    setGoneTo((list) => [num, ...list.filter((n) => n !== num)]);
    setSelected(`object-${num}`);
    setLines(FIRST_LINES);
  }, []);
  const select = useCallback((id: string) => {
    setSelected(id);
    setLines(FIRST_LINES);
  }, []);
  return { expanded, setExpanded, selected, select, goTo, rows, path, object, mode, setMode, lines, setLines };
}

/** Go to object: a number, then Enter. */
function GoToObject({ onGoTo }: { onGoTo: (num: number) => void }): ReactNode {
  const [text, setText] = useState("");
  const [error, setError] = useState<string | undefined>(undefined);
  const submit = () => {
    const typed = text.trim();
    const num = /^\d+$/.test(typed) ? Number(typed) : Number.NaN;
    if (Number.isSafeInteger(num) && num > 0) {
      setError(undefined);
      onGoTo(num);
    } else {
      setError(laterMessage("inspect.goTo.prompt"));
    }
  };
  return (
    <div className="paper-inspect-goto">
      <TextInput
        label={laterMessage("inspect.goTo")}
        placeholder={laterMessage("inspect.goTo.placeholder")}
        inputMode="numeric"
        value={text}
        onChange={setText}
        onKeyDown={(e) => {
          if (e.key === "Enter") submit();
        }}
        {...(error === undefined ? {} : { errorMessage: error })}
      />
    </div>
  );
}

/** The stream shown with the selected object, read in the view chosen as it's needed. */
function useStream(props: ObjectInspectorProps, object: InspectObjectData | undefined, mode: StreamMode) {
  const { objects, streams, onReadStream, onRead, entries } = props;
  const num = object === undefined ? undefined : streamOf(object, entries);
  const view = mode === "decoded" ? "decoded" : "raw";
  const state = num === undefined ? undefined : streams.get(`${num}:${view}`);
  // A stream that can't be decoded (past the limit, or broken) shows its raw bytes instead.
  const failed = state?.status === "over-limit" || state?.status === "failed";
  const raw = num === undefined || !failed ? undefined : streams.get(`${num}:raw`);
  useEffect(() => {
    // A page's content stream is an object of its own, read for its filters and length.
    if (num !== undefined && !objects.has(num)) onRead(num);
    if (num !== undefined && state === undefined) onReadStream(num, view);
    if (num !== undefined && failed && raw === undefined) onReadStream(num, "raw");
  }, [num, view, state, failed, raw, objects, onRead, onReadStream]);
  const decoded = num === undefined ? undefined : streams.get(`${num}:decoded`);
  const decodedLength = decoded?.status === "read" ? (decoded.total ?? ("longer" as const)) : undefined;
  return { num, state, raw, data: num === undefined ? undefined : ready(objects.get(num))?.stream, decodedLength };
}

/** The props the detail and the tab share. */
type TabProps = ObjectInspectorProps & { view: ReturnType<typeof useObjectsView> };

/** The stream block for the object shown, once its stream's first bytes are read. */
function streamBlock(props: TabProps, num: number, stream: ReturnType<typeof useStream>): ReactNode {
  const streamNum = stream.num;
  if (streamNum === undefined || stream.data === undefined || stream.state === undefined) return null;
  const { view } = props;
  return (
    <StreamBlock
      num={streamNum}
      content={streamNum !== num}
      filters={stream.data.filters}
      length={stream.data.length}
      decodedLength={stream.decodedLength}
      mode={view.mode}
      onMode={view.setMode}
      state={stream.state}
      raw={stream.raw}
      lines={view.lines}
      onMore={() => view.setLines((n) => n + MORE_LINES)}
      onSaveRaw={() => props.onSaveRaw(streamNum)}
    />
  );
}

/** The selected object's detail, or why it isn't shown. */
function Detail(props: TabProps): ReactNode {
  const { view, objects, entries, referrers, onReadReferrers } = props;
  const num = view.object;
  const state = num === undefined ? undefined : objects.get(num);
  const object = ready(state);
  const stream = useStream(props, object, view.mode);
  // What refers to it is found once its detail shows.
  useEffect(() => {
    if (object !== undefined && !referrers.has(object.entry.num)) onReadReferrers(object.entry.num);
  }, [object, referrers, onReadReferrers]);
  if (num === undefined) return <p className="paper-inspect-note">{laterMessage("inspect.pick")}</p>;
  if (entries.get(num) === null) {
    return <p className="paper-inspect-note">{laterMessage("inspect.goTo.missing", { text: String(num) })}</p>;
  }
  if (state?.status === "failed") return <ReadFailed reason={state.reason} onRetry={() => props.onRead(num)} />;
  if (object === undefined) return <p className="paper-inspect-note">{laterMessage("inspect.loading")}</p>;
  return (
    <ObjectDetail
      object={object}
      entries={entries}
      revisions={Math.max(ready(props.summary)?.revisions ?? 1, 1)}
      path={view.path}
      onPath={view.select}
      onObject={view.goTo}
      page={props.pageOf(num)}
      onShowPage={props.onShowPage}
      onCopy={() => props.onCopy(num)}
      referrers={referrers.get(num)}
      onRetryReferrers={() => onReadReferrers(num)}
      stream={streamBlock(props, num, stream)}
    />
  );
}

/** The Objects tab: the rail with Go to object and the tree, and the selected object's detail. */
function ObjectsTab(props: TabProps): ReactNode {
  const { view } = props;
  return (
    <div className="paper-inspect-objects">
      <aside className="paper-inspect-rail" aria-label={laterMessage("inspect.tree")}>
        <GoToObject onGoTo={view.goTo} />
        {props.summary.status === "failed" ? (
          <ReadFailed reason={props.summary.reason} onRetry={props.onRetrySummary} />
        ) : (
          <ObjectTree
            rows={view.rows}
            expanded={view.expanded}
            onExpandedChange={view.setExpanded}
            selected={view.selected}
            onSelect={view.select}
          />
        )}
      </aside>
      <main className="paper-inspect-main" aria-label={laterMessage("inspect.detail")}>
        <Detail {...props} />
      </main>
    </div>
  );
}

/** A tab's panel on the other tabs, reading its facts when first shown; focusable so the keys scroll it. */
function FactsPanel<T>({
  tab,
  state,
  onRead,
  children,
}: {
  tab: InspectFactsTab;
  state: ReadState<T> | undefined;
  onRead: (tab: InspectFactsTab) => void;
  children: (value: T) => ReactNode;
}): ReactNode {
  useEffect(() => {
    if (state === undefined) onRead(tab);
  }, [state, tab, onRead]);
  let body: ReactNode = <p className="paper-inspect-note">{laterMessage("inspect.loading")}</p>;
  if (state?.status === "failed") body = <ReadFailed reason={state.reason} onRetry={() => onRead(tab)} />;
  if (state?.status === "ready") body = children(state.value);
  return (
    <section
      className="paper-inspect-main paper-inspect-facts-panel"
      aria-label={laterMessage(`inspect.tab.${tab}`)}
      // biome-ignore lint/a11y/noNoninteractiveTabindex: a scrolling region with no control in it takes focus to scroll.
      tabIndex={0}
    >
      {body}
    </section>
  );
}

/** The object inspector, in place of the pages. */
export function ObjectInspector(props: ObjectInspectorProps): ReactNode {
  const { request } = props;
  const [tab, setTab] = useState<InspectTab>(request?.tab ?? "objects");
  const view = useObjectsView(props);
  const { tabs, onReadTab } = props;
  // Each request shows its tab and object, the same one asked again included.
  const seq = request?.seq;
  // biome-ignore lint/correctness/useExhaustiveDependencies: a new request is a new `seq`; the request read here is that one.
  useEffect(() => {
    if (request === undefined) return;
    setTab(request.tab);
    if (request.goTo !== undefined) view.goTo(request.goTo);
  }, [seq, view.goTo]);
  return (
    <Tabs className="paper-object-inspector" selectedKey={tab} onSelectionChange={(key) => setTab(key as InspectTab)}>
      <div className="paper-inspect-bar">
        <button type="button" className="paper-inspect-back" onClick={props.onBack}>
          <Icon name="chevron-left" size={16} />
          {laterMessage("inspect.back")}
        </button>
        <span className="paper-inspect-divider" aria-hidden="true" />
        <TabList aria-label={laterMessage("inspect.tabs")} className="paper-tab-list">
          {TABS.map((id) => (
            <Tab key={id} id={id} className="paper-tab">
              <StableLabel>{laterMessage(`inspect.tab.${id}`)}</StableLabel>
            </Tab>
          ))}
        </TabList>
        <span className="paper-inspect-spacer" />
        <Badge>{laterMessage("inspect.readOnly")}</Badge>
        <span className="paper-inspect-summary">{summaryText(ready(props.summary))}</span>
      </div>
      <TabPanel id="objects" className="paper-inspect-panel">
        <ObjectsTab {...props} view={view} />
      </TabPanel>
      <TabPanel id="revisions" className="paper-inspect-panel">
        <FactsPanel tab="revisions" state={tabs.revisions} onRead={onReadTab}>
          {(revisions) => <RevisionsTable revisions={revisions} />}
        </FactsPanel>
      </TabPanel>
      <TabPanel id="fonts" className="paper-inspect-panel">
        <FactsPanel tab="fonts" state={tabs.fonts} onRead={onReadTab}>
          {(fonts) => <FontsTable fonts={fonts} />}
        </FactsPanel>
      </TabPanel>
      <TabPanel id="colour" className="paper-inspect-panel">
        <FactsPanel tab="colour" state={tabs.colour} onRead={onReadTab}>
          {(colour) => <ColourTables colour={colour} />}
        </FactsPanel>
      </TabPanel>
      <TabPanel id="conformance" className="paper-inspect-panel">
        <FactsPanel tab="conformance" state={tabs.conformance} onRead={onReadTab}>
          {(conformance) => <ConformanceFacts conformance={conformance} />}
        </FactsPanel>
      </TabPanel>
    </Tabs>
  );
}
