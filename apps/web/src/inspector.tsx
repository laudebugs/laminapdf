// The object inspector (O15) in the web reader, loaded with later.ts once a document is open: it
// reads the document through the worker (the overview when it opens, an object as the tree or the
// detail needs it, a stream's first megabyte in the view chosen) and hands the reads to
// @laminapdf/ui's ObjectInspector. Nothing it does changes the file; a stream that decodes past the
// document's limits is refused by the engine and shown raw, with Save raw stream….

import type { CosValue } from "@laminapdf/sdk";
import { formatBytes, type Platform, type ToastQueue, useStoreState } from "@laminapdf/ui";
import {
  type InspectObjectData,
  type InspectOverview,
  type InspectTab,
  ObjectInspector,
  STREAM_CAP,
  type StreamState,
} from "@laminapdf/ui/dialogs";
import { type ReactNode, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { copier } from "./copy.ts";
import type { Engine } from "./session.ts";

/** What opens the inspector: a tab, and an object to go to. */
export interface InspectRequest {
  readonly tab: InspectTab;
  readonly goTo?: number | undefined;
}

/** Props of `InspectorView`. */
export interface InspectorViewProps {
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

/** Why a stream couldn't be read: over the document's decode limit, or another failure. */
function streamProblem(error: unknown, limit: number | undefined): StreamState {
  const { code, message } = error as { code?: unknown; message?: unknown };
  if (code === "limit-exceeded") {
    return { status: "over-limit", limit: limit === undefined ? "?" : formatBytes(limit) };
  }
  return { status: "failed", reason: typeof message === "string" ? message : String(error) };
}

/** The document's overview, read once when the inspector opens. */
function useOverview(engine: Engine): InspectOverview | undefined {
  const [overview, setOverview] = useState<InspectOverview | undefined>(undefined);
  useEffect(() => {
    let live = true;
    void engine
      .view()
      .then(({ client }) => client.inspectionOverview())
      .then(
        (o) => {
          if (!live) return;
          const version = laterVersion(o.conformance.headerVersion, o.conformance.catalogVersion);
          setOverview({ ...o, trailer: o.trailer as CosValue, version });
        },
        (error: unknown) => reportError(error),
      );
    return () => {
      live = false;
    };
  }, [engine]);
  return overview;
}

/** The objects and streams read so far, and the reads that fill them. */
function useReads(engine: Engine) {
  const [objects, setObjects] = useState<ReadonlyMap<number, InspectObjectData & { syntax: string }>>(new Map());
  const [streams, setStreams] = useState<ReadonlyMap<string, StreamState>>(new Map());
  const asked = useRef(new Set<string>());
  const limit = useStoreState(engine, (s) => s.attachmentLimit);
  const onRead = useCallback(
    (num: number) => {
      if (asked.current.has(`o${num}`)) return;
      asked.current.add(`o${num}`);
      void engine
        .view()
        .then(({ client }) => client.objectDetail(num))
        .then((detail) => {
          if (detail === undefined) return;
          const object = { ...detail.object, referrers: detail.referrers, syntax: detail.syntax };
          setObjects((map) => new Map(map).set(num, object));
        }, reportError);
    },
    [engine],
  );
  const onReadStream = useCallback(
    (num: number, view: "decoded" | "raw") => {
      const key = `${num}:${view}`;
      if (asked.current.has(key)) return;
      asked.current.add(key);
      const set = (state: StreamState) => setStreams((map) => new Map(map).set(key, state));
      set({ status: "reading" });
      void engine
        .view()
        .then(({ client }) => client.streamRead(num, view, STREAM_CAP))
        .then(
          (read) => set(read === undefined ? { status: "failed", reason: "" } : { status: "read", ...read }),
          (error: unknown) => set(streamProblem(error, limit)),
        );
    },
    [engine, limit],
  );
  return { objects, streams, onRead, onReadStream };
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
  const overview = useOverview(engine);
  const reads = useReads(engine);
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
  return (
    <div className="web-inspector">
      <ObjectInspector
        overview={overview}
        objects={reads.objects}
        onRead={reads.onRead}
        streams={reads.streams}
        onReadStream={reads.onReadStream}
        pageOf={pageOf}
        initialTab={request.tab}
        goTo={request.goTo}
        onBack={onBack}
        onShowPage={onShowPage}
        onCopy={(num) => {
          const syntax = reads.objects.get(num)?.syntax;
          if (syntax !== undefined) copy(syntax);
        }}
        onSaveRaw={saveRaw}
      />
    </div>
  );
}
