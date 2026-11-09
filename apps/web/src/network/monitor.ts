// The reader's network log (R13): every request the page and the document worker made, merged
// into one store the status bar and the network panel read. The page's requests come from
// `watchNetwork` installed by page-feed.ts before the app starts; the worker's from its
// `network` events and its `networkLog`, merged by their index so none is counted twice.
// `summarize` turns the log into what the chrome shows: document traffic since the file opened,
// by origin, apart from the app's own files.

import type { NetworkRecord, PolicyViolation, WorkerNetworkLog } from "@laminapdf/viewer";
import { classifyRequest, originOf, purposeOf, type RequestGroup, type RequestPurpose } from "./classify.ts";

/** What the store holds. */
export interface NetworkState {
  /** Every request, page and worker, in the order heard. */
  readonly requests: readonly NetworkRecord[];
  /** Every request a Content Security Policy blocked. */
  readonly violations: readonly PolicyViolation[];
  /** Whether the document worker's log has been read: until then, its feed is missing. */
  readonly workerFeed: boolean;
  /** When the open document was opened (ms since the epoch); undefined while none is open. */
  readonly openedAt: number | undefined;
  /** The open document's address when it came from a `#url=` link. */
  readonly documentUrl: string | undefined;
  /**
   * Every document opened, in order: each request is classified against the one opening when it
   * was made, so an earlier linked file's requests stay document traffic once another file opens.
   */
  readonly openings: readonly { readonly at: number; readonly url: string | undefined }[];
}

/** One origin's requests since the document opened. */
export interface OriginRow {
  readonly origin: string;
  readonly group: Exclude<RequestGroup, "app">;
  readonly requests: number;
  readonly bytes: number;
}

/** One kind of app file. */
export interface AppRow {
  readonly purpose: RequestPurpose;
  readonly requests: number;
  readonly bytes: number;
}

/** What the status bar and the panel show. */
export interface NetworkSummary {
  /** Every request since the page loaded, whatever its group. */
  readonly total: number;
  /** Requests that aren't the app's own files, since the document opened. */
  readonly documentRequests: number;
  readonly documentRows: readonly OriginRow[];
  /** The origins neither the app nor the open document's host. */
  readonly unexpected: readonly string[];
  /** The app's own files since the page loaded. */
  readonly appRequests: number;
  readonly appRows: readonly AppRow[];
  readonly blocked: readonly PolicyViolation[];
}

/** Adds `bytes` and one request to the row for `key` in `rows`, made by `make` the first time. */
function tally<K, R extends { requests: number; bytes: number }>(
  rows: Map<K, R>,
  key: K,
  bytes: number,
  make: () => R,
): void {
  const row = rows.get(key) ?? make();
  row.requests += 1;
  row.bytes += bytes;
  rows.set(key, row);
}

/** The summary of `state` for a reader served from `appOrigin`. */
export function summarize(state: NetworkState, appOrigin: string): NetworkSummary {
  /** The document opening when a request made at `at` was made. */
  const documentAt = (at: number) => {
    let url: string | undefined;
    for (const opening of state.openings) if (opening.at <= at) url = opening.url;
    return url;
  };
  const documentRows = new Map<
    string,
    { origin: string; group: OriginRow["group"]; requests: number; bytes: number }
  >();
  const appRows = new Map<RequestPurpose, AppRow & { requests: number; bytes: number }>();
  let total = 0;
  for (const record of state.requests) {
    const origin = originOf(record.url);
    if (origin === undefined) continue;
    total += 1;
    const group = classifyRequest(record.url, { appOrigin, documentUrl: documentAt(record.at) });
    if (group === "app") {
      const purpose = purposeOf(record.url, record.initiator, group);
      tally(appRows, purpose, record.bytes, () => ({ purpose, requests: 0, bytes: 0 }));
    } else if (state.openedAt !== undefined && record.at >= state.openedAt) {
      tally(documentRows, origin, record.bytes, () => ({ origin, group, requests: 0, bytes: 0 }));
    } else if (group === "unexpected") {
      // An unexpected request is shown even when no document is open: nothing should make it.
      tally(documentRows, origin, record.bytes, () => ({ origin, group, requests: 0, bytes: 0 }));
    }
  }
  const rows = [...documentRows.values()].sort((a, b) => b.requests - a.requests || a.origin.localeCompare(b.origin));
  return {
    total,
    documentRequests: rows.reduce((sum, row) => sum + row.requests, 0),
    documentRows: rows,
    unexpected: rows.filter((row) => row.group === "unexpected").map((row) => row.origin),
    appRequests: [...appRows.values()].reduce((sum, row) => sum + row.requests, 0),
    appRows: [...appRows.values()].sort((a, b) => b.requests - a.requests),
    blocked: state.violations,
  };
}

/** The reader's network log, as an external store for React (`useStoreState`). */
export class NetworkMonitor {
  readonly #listeners = new Set<() => void>();
  /** Worker records and violations already merged, by `<worker>:<index>`. */
  readonly #seen = new Set<string>();
  #state: NetworkState = {
    requests: [],
    violations: [],
    workerFeed: false,
    openedAt: undefined,
    documentUrl: undefined,
    openings: [],
  };

  getState = (): NetworkState => this.#state;

  subscribe = (listener: () => void): (() => void) => {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  };

  /** A request the page made. */
  pageRequest(record: NetworkRecord): void {
    this.#set({ requests: [...this.#state.requests, record] });
  }

  /** A request the page's policy blocked. */
  pageViolation(violation: PolicyViolation): void {
    this.#set({ violations: [...this.#state.violations, violation] });
  }

  /** A request the document worker made, the `index`th of worker `worker`'s log. */
  workerRequest(worker: string, index: number, record: NetworkRecord): void {
    if (this.#first(`r:${worker}:${index}`)) this.#set({ requests: [...this.#state.requests, record] });
  }

  /** A request the document worker's policy blocked. */
  workerViolation(worker: string, index: number, violation: PolicyViolation): void {
    if (this.#first(`v:${worker}:${index}`)) this.#set({ violations: [...this.#state.violations, violation] });
  }

  /** The document worker's whole log: what its events haven't brought yet joins the store. */
  workerLog(log: WorkerNetworkLog): void {
    const requests = log.requests.filter((_, i) => this.#first(`r:${log.worker}:${i}`));
    const violations = log.violations.filter((_, i) => this.#first(`v:${log.worker}:${i}`));
    this.#set({
      requests: [...this.#state.requests, ...requests],
      violations: [...this.#state.violations, ...violations],
      workerFeed: true,
    });
  }

  /** A document began opening at `at`: from a `#url=` link at `documentUrl`, or a local file. */
  documentOpening(at: number, documentUrl: string | undefined): void {
    this.#set({ openedAt: at, documentUrl, openings: [...this.#state.openings, { at, url: documentUrl }] });
  }

  /** The document closed: the status bar has nothing to count until the next one opens. */
  documentClosed(): void {
    this.#set({ openedAt: undefined, documentUrl: undefined });
  }

  #first(key: string): boolean {
    if (this.#seen.has(key)) return false;
    this.#seen.add(key);
    return true;
  }

  #set(next: Partial<NetworkState>): void {
    this.#state = { ...this.#state, ...next };
    for (const listener of this.#listeners) listener();
  }
}
