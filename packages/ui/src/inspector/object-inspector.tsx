// The object inspector (O15): a full-canvas view in place of the pages until Back to pages, for
// power users and developers. A view bar takes the secondary toolbar's place (Back to pages, the
// Objects · Revisions · Fonts · Colour · Conformance tabs, the Read only badge and the file's
// version and counts); on Objects, a 300 px column holds Go to object and the object tree, and the
// main area the selected object's detail. Nothing here changes the file. The app reads the
// document (`onRead`, `onReadStream`); this component holds only what the view shows.

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
import { StreamBlock, type StreamMode, type StreamState } from "./stream-block.js";
import {
  type InspectObjectData,
  type InspectObjectEntry,
  type InspectValue,
  pathOf,
  type TreeRow,
  treeRows,
} from "./values.js";

/** The inspector's tabs. */
export type InspectTab = "objects" | "revisions" | "fonts" | "colour" | "conformance";

/** What the inspector reads of the document when it opens. */
export interface InspectOverview {
  /** The trailer, the tree's root. */
  readonly trailer: InspectValue;
  readonly conformance: InspectConformance;
  readonly objects: readonly InspectObjectEntry[];
  readonly revisions: readonly InspectRevision[];
  readonly fonts: readonly InspectFont[];
  readonly colour: InspectColour;
  /** The version the file declares ("1.7"). */
  readonly version: string | undefined;
}

/** Props of `ObjectInspector`. */
export interface ObjectInspectorProps {
  /** What the document is, once read; the view says it is reading until then. */
  readonly overview: InspectOverview | undefined;
  /** The objects read so far, by number. */
  readonly objects: ReadonlyMap<number, InspectObjectData>;
  /** Read object `num` (its value, what refers to it). */
  readonly onRead: (num: number) => void;
  /** What was read of each stream, by `num:decoded` or `num:raw`. */
  readonly streams: ReadonlyMap<string, StreamState>;
  /** Read the first bytes of stream `num`, decoded or raw. */
  readonly onReadStream: (num: number, view: "decoded" | "raw") => void;
  /** The 0-based page an object is, if it is one. */
  readonly pageOf: (num: number) => number | undefined;
  /** The tab it opens on. */
  readonly initialTab?: InspectTab;
  /** An object to go to as it opens ("Go to object…"). */
  readonly goTo?: number | undefined;
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

/** The stream to show with object `object`: itself, or a page's first content stream. */
function streamOf(object: InspectObjectData, entries: ReadonlyMap<number, InspectObjectEntry>): number | undefined {
  if (object.stream !== undefined) return object.entry.num;
  if (object.entry.typeName !== "Page" || object.value.type !== "dictionary") return undefined;
  const contents = object.value.entries.find(([key]) => key === "Contents")?.[1];
  const first = contents?.type === "array" ? contents.items[0] : contents;
  return first?.type === "reference" && entries.get(first.num)?.kind === "stream" ? first.num : undefined;
}

/** The view bar's summary: "PDF 1.7 · 412 objects · 2 revisions". */
function summary(overview: InspectOverview | undefined): string {
  if (overview === undefined) return laterMessage("inspect.loading");
  return laterMessage("inspect.summary", {
    version: overview.version ?? "?",
    objects: overview.objects.length.toLocaleString("en"),
    revisions: laterCountMessage(Math.max(overview.revisions.length, 1), {
      one: "inspect.revisions.one",
      many: "inspect.revisions.many",
    }),
  });
}

/** The state the Objects tab keeps: open rows, the selection, objects gone to, the stream's view. */
function useObjectsView(props: ObjectInspectorProps) {
  const { overview, objects, onRead } = props;
  // The catalog and the page tree open at first, as the board draws them.
  const [expanded, setExpanded] = useState<ReadonlySet<string>>(
    () => new Set(["trailer", "trailer//Root", "trailer//Root//Pages"]),
  );
  const [selected, setSelected] = useState<string | undefined>("trailer");
  const [goneTo, setGoneTo] = useState<readonly number[]>([]);
  const [mode, setMode] = useState<StreamMode>("decoded");
  const [lines, setLines] = useState(FIRST_LINES);
  const entries = useMemo(() => new Map((overview?.objects ?? []).map((e) => [e.num, e])), [overview]);
  const values = useMemo(() => new Map([...objects].map(([num, o]) => [num, o.value])), [objects]);
  const rows: readonly TreeRow[] = useMemo(
    () => treeRows(overview?.trailer, goneTo, { entries, values, pageOf: props.pageOf, expanded }),
    [overview, goneTo, entries, values, props.pageOf, expanded],
  );
  const path = selected === undefined ? [] : pathOf(selected, rows);
  const object = path.at(-1)?.object;
  // An open reference and the selected object are read as they're needed.
  useEffect(() => {
    for (const id of [...expanded, selected]) {
      const num = id === undefined ? undefined : pathOf(id, rows).at(-1)?.object;
      if (num !== undefined && !objects.has(num) && entries.has(num)) onRead(num);
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
  return { expanded, setExpanded, selected, select, goTo, rows, path, object, entries, mode, setMode, lines, setLines };
}

/** Go to object: a number, then Enter. */
function GoToObject({
  entries,
  onGoTo,
}: {
  entries: ReadonlyMap<number, InspectObjectEntry>;
  onGoTo: (num: number) => void;
}): ReactNode {
  const [text, setText] = useState("");
  const [error, setError] = useState<string | undefined>(undefined);
  const submit = () => {
    const num = Number.parseInt(text.trim(), 10);
    if (Number.isInteger(num) && entries.has(num)) {
      setError(undefined);
      onGoTo(num);
    } else {
      setError(laterMessage("inspect.goTo.missing", { text: text.trim() }));
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
function useStream(
  props: ObjectInspectorProps,
  object: InspectObjectData | undefined,
  entries: ReadonlyMap<number, InspectObjectEntry>,
  mode: StreamMode,
) {
  const { objects, streams, onReadStream, onRead } = props;
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
  return {
    num,
    state,
    raw,
    data: num === undefined ? undefined : objects.get(num)?.stream,
    decodedLength: decoded?.status === "read" ? decoded.total : undefined,
  };
}

/** The Objects tab: the rail with Go to object and the tree, and the selected object's detail. */
function ObjectsTab(props: ObjectInspectorProps & { view: ReturnType<typeof useObjectsView> }): ReactNode {
  const { view, objects, goTo } = props;
  // "Go to object…" from the palette opens here, once the objects are known.
  useEffect(() => {
    if (goTo !== undefined && view.entries.has(goTo)) view.goTo(goTo);
  }, [goTo, view.entries, view.goTo]);
  const object = view.object === undefined ? undefined : objects.get(view.object);
  const stream = useStream(props, object, view.entries, view.mode);
  const streamNum = stream.num;
  return (
    <div className="paper-inspect-objects">
      <aside className="paper-inspect-rail" aria-label={laterMessage("inspect.tree")}>
        <GoToObject entries={view.entries} onGoTo={view.goTo} />
        <ObjectTree
          rows={view.rows}
          expanded={view.expanded}
          onExpandedChange={view.setExpanded}
          selected={view.selected}
          onSelect={view.select}
        />
      </aside>
      <main className="paper-inspect-main" aria-label={laterMessage("inspect.detail")}>
        {object === undefined ? (
          <p className="paper-inspect-note">
            {laterMessage(view.object === undefined ? "inspect.pick" : "inspect.loading")}
          </p>
        ) : (
          <ObjectDetail
            object={object}
            entries={view.entries}
            revisions={Math.max(props.overview?.revisions.length ?? 1, 1)}
            path={view.path}
            onPath={view.select}
            onObject={view.goTo}
            page={props.pageOf(object.entry.num)}
            onShowPage={props.onShowPage}
            onCopy={() => props.onCopy(object.entry.num)}
            stream={
              streamNum === undefined || stream.data === undefined || stream.state === undefined ? null : (
                <StreamBlock
                  num={streamNum}
                  content={streamNum !== object.entry.num}
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
              )
            }
          />
        )}
      </main>
    </div>
  );
}

/** A tab's panel on the other tabs: a table or two, scrolling, and focusable so the keys scroll it. */
function FactsPanel({ label, children }: { label: string; children: ReactNode }): ReactNode {
  return (
    // biome-ignore lint/a11y/noNoninteractiveTabindex: a scrolling region with no control in it takes focus to scroll.
    <section className="paper-inspect-main paper-inspect-facts-panel" aria-label={label} tabIndex={0}>
      {children}
    </section>
  );
}

/** The object inspector, in place of the pages. */
export function ObjectInspector(props: ObjectInspectorProps): ReactNode {
  const [tab, setTab] = useState<InspectTab>(props.initialTab ?? "objects");
  const view = useObjectsView(props);
  const { overview } = props;
  useEffect(() => {
    if (props.goTo !== undefined) setTab("objects");
  }, [props.goTo]);
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
        <span className="paper-inspect-summary">{summary(overview)}</span>
      </div>
      <TabPanel id="objects" className="paper-inspect-panel">
        <ObjectsTab {...props} view={view} />
      </TabPanel>
      <TabPanel id="revisions" className="paper-inspect-panel">
        <FactsPanel label={laterMessage("inspect.tab.revisions")}>
          {overview === undefined ? null : <RevisionsTable revisions={overview.revisions} />}
        </FactsPanel>
      </TabPanel>
      <TabPanel id="fonts" className="paper-inspect-panel">
        <FactsPanel label={laterMessage("inspect.tab.fonts")}>
          {overview === undefined ? null : <FontsTable fonts={overview.fonts} />}
        </FactsPanel>
      </TabPanel>
      <TabPanel id="colour" className="paper-inspect-panel">
        <FactsPanel label={laterMessage("inspect.tab.colour")}>
          {overview === undefined ? null : <ColourTables colour={overview.colour} />}
        </FactsPanel>
      </TabPanel>
      <TabPanel id="conformance" className="paper-inspect-panel">
        <FactsPanel label={laterMessage("inspect.tab.conformance")}>
          {overview === undefined ? null : <ConformanceFacts conformance={overview.conformance} />}
        </FactsPanel>
      </TabPanel>
    </Tabs>
  );
}
