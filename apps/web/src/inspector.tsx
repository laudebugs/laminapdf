// The object inspector (O15) in the web reader, loaded with later.ts once a document is open: it
// reads the document through the worker only as the inspector shows things (the summary when it
// opens, a tab's facts when the tab is shown, an object as the tree or the detail needs it, what
// refers to it when its detail shows, a stream's first megabyte in the view chosen) and hands the
// reads to @laminapdf/ui's ObjectInspector. A read that fails says why and can be tried again.
// Nothing it does changes the file; the engine decodes a stream only as far as it is shown.

import type { ObjectEntry } from "@laminapdf/sdk";
import { formatBytes, type Platform, type ToastQueue, useStoreState } from "@laminapdf/ui";
import {
  type InspectEntries,
  type InspectFactsTab,
  type InspectObjectData,
  type InspectReferrer,
  type InspectSummary,
  type InspectTab,
  type InspectTabs,
  ObjectInspector,
  type ReadState,
  STREAM_CAP,
  type StreamState,
} from "@laminapdf/ui/dialogs";
import type { ViewerClient } from "@laminapdf/viewer";
import { type ReactNode, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { copier } from "./copy.ts";
import type { Engine } from "./session.ts";

/** What opens the inspector: a tab, and an object to go to. */
export interface InspectRequest {
  readonly tab: InspectTab;
  readonly goTo?: number | undefined;
  /** Counts the requests while the inspector is open, so one asked again shows again. */
  readonly seq?: number | undefined;
}

/** Props of `InspectorView`. */
interface InspectorViewProps {
  readonly engine: Engine;
  readonly platform: Platform;
  readonly toasts: ToastQueue;
  readonly request: InspectRequest;
  /** Back to pages. */
  readonly onBack: () => void;
  /** Show page: back to the pages, at page `page`. */
  readonly onShowPage: (page: number) => void;
}

/** The later of two versions ("1.7", "2.0"). */
function laterVersion(a: string | undefined, b: string | undefined): string | undefined {
  if (a === undefined) return b;
  if (b === undefined) return a;
  return Number.parseFloat(b) > Number.parseFloat(a) ? b : a;
}

/** The object number of a page ID (`p12.0`), the form the engine names pages by. */
function pageObject(id: string): number | undefined {
  const match = /^p(\d+)\.\d+$/.exec(id);
  return match === null ? undefined : Number(match[1]);
}

/** Why a read failed, in words. */
function reasonOf(error: unknown): string {
  const { message } = error as { message?: unknown };
  return typeof message === "string" ? message : String(error);
}

/** Why a stream couldn't be read: over the document's decode limit, or another failure. */
function streamProblem(error: unknown, limit: number | undefined): StreamState {
  if ((error as { code?: unknown }).code === "limit-exceeded") {
    return { status: "over-limit", limit: limit === undefined ? "?" : formatBytes(limit) };
  }
  return { status: "failed", reason: reasonOf(error) };
}

/** A map with `key` set to `value`. */
function withEntry<K, V>(map: ReadonlyMap<K, V>, key: K, value: V): ReadonlyMap<K, V> {
  return new Map(map).set(key, value);
}

/**
 * Reads under a key, at most once at a time: `run` marks the key, reads, keeps the outcome with
 * `keep`, and forgets the key after a failure, so Try again reads once more.
 */
function useReader(engine: Engine) {
  const asked = useRef(new Set<string>());
  return useCallback(
    <T,>(key: string, read: (client: ViewerClient) => Promise<T>, keep: (state: ReadState<T>) => void) => {
      if (asked.current.has(key)) return;
      asked.current.add(key);
      keep({ status: "reading" });
      void engine
        .view()
        .then(({ client }) => read(client))
        .then(
          (value) => keep({ status: "ready", value }),
          (error: unknown) => {
            asked.current.delete(key);
            keep({ status: "failed", reason: reasonOf(error) });
          },
        );
    },
    [engine],
  );
}

/** The entries `list` adds to `map`. */
function addEntries(map: InspectEntries, list: readonly ObjectEntry[]): InspectEntries {
  const out = new Map(map);
  for (const entry of list) out.set(entry.num, entry);
  return out;
}

/** Each tab's facts, read when the tab is first shown. */
function useTabs(run: ReturnType<typeof useReader>) {
  const [tabs, setTabs] = useState<InspectTabs>({});
  const onReadTab = useCallback(
    (tab: InspectFactsTab) =>
      run(
        `t${tab}`,
        (client) => client.inspectionTab(tab),
        (state) => {
          if (state.status !== "ready") return setTabs((t) => ({ ...t, [tab]: state }));
          const data = state.value;
          const value =
            data.tab === "revisions"
              ? data.revisions
              : data.tab === "fonts"
                ? data.fonts
                : data.tab === "colour"
                  ? data.colour
                  : data.conformance;
          setTabs((t) => ({ ...t, [tab]: { status: "ready", value } }));
        },
      ),
    [run],
  );
  return { tabs, onReadTab };
}

/** Everything read for the inspector, and the reads that fill it. */
function useReads(engine: Engine) {
  const run = useReader(engine);
  const [summary, setSummary] = useState<ReadState<InspectSummary>>({ status: "reading" });
  const [entries, setEntries] = useState<InspectEntries>(new Map());
  const [objects, setObjects] = useState<ReadonlyMap<number, ReadState<InspectObjectData>>>(new Map());
  const [referrers, setReferrers] = useState<ReadonlyMap<number, ReadState<readonly InspectReferrer[]>>>(new Map());
  const readSummary = useCallback(
    () =>
      run(
        "summary",
        (client) => client.inspectionSummary(),
        (state) => {
          if (state.status !== "ready") return setSummary(state);
          const s = state.value;
          setEntries((map) => addEntries(map, s.entries));
          const version = laterVersion(s.headerVersion, s.catalogVersion);
          setSummary({
            status: "ready",
            value: { trailer: s.trailer, version, objects: s.objects, revisions: s.revisions },
          });
        },
      ),
    [run],
  );
  useEffect(readSummary, [readSummary]);
  const onRead = useCallback(
    (num: number) =>
      run(
        `o${num}`,
        (client) => client.objectDetail(num),
        (state) => {
          if (state.status !== "ready") return setObjects((map) => withEntry(map, num, state));
          const detail = state.value;
          if (detail === undefined) return setEntries((map) => withEntry(map, num, null));
          setEntries((map) => addEntries(map, [...detail.children, detail.object.entry]));
          setObjects((map) => withEntry(map, num, { status: "ready", value: detail.object }));
        },
      ),
    [run],
  );
  const onReadReferrers = useCallback(
    (num: number) =>
      run(
        `r${num}`,
        (client) => client.objectReferrers(num),
        (state) => {
          if (state.status === "ready") setEntries((map) => addEntries(map, state.value.entries));
          const list: ReadState<readonly InspectReferrer[]> =
            state.status === "ready" ? { status: "ready", value: state.value.referrers } : state;
          setReferrers((map) => withEntry(map, num, list));
        },
      ),
    [run],
  );
  const { tabs, onReadTab } = useTabs(run);
  return { summary, readSummary, entries, objects, onRead, referrers, onReadReferrers, tabs, onReadTab };
}

/** The first bytes of each stream, read as the stream block asks. */
function useStreams(engine: Engine) {
  const [streams, setStreams] = useState<ReadonlyMap<string, StreamState>>(new Map());
  const asked = useRef(new Set<string>());
  const limit = useStoreState(engine, (s) => s.attachmentLimit);
  const onReadStream = useCallback(
    (num: number, view: "decoded" | "raw") => {
      const key = `${num}:${view}`;
      if (asked.current.has(key)) return;
      asked.current.add(key);
      const set = (state: StreamState) => setStreams((map) => withEntry(map, key, state));
      set({ status: "reading" });
      void engine
        .view()
        .then(({ client }) => client.streamRead(num, view, STREAM_CAP))
        .then(
          (read) => set(read === undefined ? { status: "failed", reason: "" } : { status: "read", ...read }),
          (error: unknown) => {
            // A failure other than the limit can be read again by choosing the view again.
            if ((error as { code?: unknown }).code !== "limit-exceeded") asked.current.delete(key);
            set(streamProblem(error, limit));
          },
        );
    },
    [engine, limit],
  );
  return { streams, onReadStream };
}

/** The object inspector over the open document. */
export function InspectorView({
  engine,
  platform,
  toasts,
  request,
  onBack,
  onShowPage,
}: InspectorViewProps): ReactNode {
  const reads = useReads(engine);
  const { streams, onReadStream } = useStreams(engine);
  const pages = useStoreState(engine, (s) => s.pages);
  const pageIndex = useMemo(() => {
    const map = new Map<number, number>();
    pages.forEach((page, index) => {
      const num = pageObject(String(page.id));
      if (num !== undefined) map.set(num, index);
    });
    return map;
  }, [pages]);
  const pageOf = useCallback((num: number) => pageIndex.get(num), [pageIndex]);
  const copy = copier(platform, toasts);
  const saveRaw = (num: number) => {
    void (async () => {
      const { client } = await engine.view();
      const whole = await client.streamRead(num, "raw", Number.MAX_SAFE_INTEGER);
      if (whole !== undefined) await platform.saveAs(whole.bytes, `object-${num}.bin`);
    })().catch(reportError);
  };
  const copyObject = (num: number) => {
    void engine
      .view()
      .then(({ client }) => client.objectDetail(num))
      .then((detail) => {
        if (detail !== undefined) copy(detail.syntax);
      }, reportError);
  };
  return (
    <div className="web-inspector">
      <ObjectInspector
        summary={reads.summary}
        onRetrySummary={reads.readSummary}
        entries={reads.entries}
        objects={reads.objects}
        onRead={reads.onRead}
        referrers={reads.referrers}
        onReadReferrers={reads.onReadReferrers}
        tabs={reads.tabs}
        onReadTab={reads.onReadTab}
        streams={streams}
        onReadStream={onReadStream}
        pageOf={pageOf}
        request={{ tab: request.tab, goTo: request.goTo, seq: request.seq ?? 0 }}
        onBack={onBack}
        onShowPage={onShowPage}
        onCopy={copyObject}
        onSaveRaw={saveRaw}
      />
    </div>
  );
}
