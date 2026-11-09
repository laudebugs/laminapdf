// Every network request a JavaScript context makes, reported as it happens: the page's and the
// document worker's (ADR 0011's privacy guarantees, R13's network panel). A context can request
// in two ways that leave different traces, so the watcher uses both:
//
// - calls the page's or the worker's code makes (`fetch`, `XMLHttpRequest`, `WebSocket`,
//   `navigator.sendBeacon`) are wrapped, so each is reported when it starts, failed or not, and a
//   WebSocket, which leaves no timing entry, is counted too;
// - everything the browser loads by itself (scripts, module imports, styles, fonts, images, the
//   page) is read from the Resource Timing entries, whose `initiatorType` tells them apart from
//   the wrapped calls, so nothing is counted twice.
//
// Installed before any other code of the context runs, so no code path can capture an unwrapped
// `fetch` first. A request the context never makes (a browser extension's) is out of its sight.

import type { NetworkRecord, NetworkSource, PolicyViolation } from "../protocol.js";

/** The entries a wrapped call also leaves in Resource Timing, read from the call instead. */
const WRAPPED_INITIATORS = new Set(["fetch", "xmlhttprequest", "beacon"]);

/** The `blockedURI`s of violations that block no request (CSP3 §5.3's keywords for them). */
const NOT_REQUESTS = new Set(["", "inline", "eval", "wasm-eval", "trusted-types-policy", "trusted-types-sink"]);

/** What `watchNetwork` reports to. */
export interface NetworkSink {
  /** A request was made: called when it starts, failed or not, or when its timing entry arrives. */
  request(record: NetworkRecord): void;
  /** The scope's Content Security Policy blocked a request, so nothing was sent. */
  violation(violation: PolicyViolation): void;
}

/** The global scope parts the watcher wraps and reads (a Window or a WorkerGlobalScope). */
type Scope = typeof globalThis & {
  XMLHttpRequest?: typeof XMLHttpRequest;
  WebSocket?: typeof WebSocket;
  PerformanceObserver?: typeof PerformanceObserver;
  location?: { href: string };
  navigator?: Navigator & { sendBeacon?: Navigator["sendBeacon"] };
  addEventListener?: (type: string, listener: (event: Event) => void) => void;
  removeEventListener?: (type: string, listener: (event: Event) => void) => void;
};

/** `input`'s absolute URL, against the scope's address. */
function urlOf(input: RequestInfo | URL | string, base: string | undefined): string {
  const raw = typeof input === "string" ? input : input instanceof URL ? input.href : input.url;
  try {
    return new URL(raw, base).href;
  } catch {
    return raw;
  }
}

/** A `Content-Length` header's bytes, or 0 when there is none or it isn't a number. */
function lengthOf(header: string | null): number {
  const length = Number(header);
  return Number.isFinite(length) && length > 0 ? length : 0;
}

/**
 * Starts reporting every request `scope` makes to `sink`, as `source`. Returns the function that
 * stops it: the scope's own `fetch`, `XMLHttpRequest`, `WebSocket` and `sendBeacon` put back, the
 * observers disconnected and the violation listener removed.
 */
export function watchNetwork(scope: Scope, sink: NetworkSink, source: NetworkSource): () => void {
  const base = scope.location?.href;
  const report: Report = (record, at = Date.now()) => sink.request({ ...record, source, at });
  const restore: (() => void)[] = [];

  const originalFetch = scope.fetch;
  if (typeof originalFetch === "function") {
    scope.fetch = async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = urlOf(input, base);
      const method = (init?.method ?? (input instanceof Request ? input.method : "GET")).toUpperCase();
      // Stamped when the call starts: a request belongs to the file that was opening then.
      const at = Date.now();
      try {
        const response = await originalFetch.call(scope, input, init);
        const bytes = lengthOf(response.headers.get("Content-Length"));
        report({ url, method, initiator: "fetch", bytes, failed: false }, at);
        return response;
      } catch (e) {
        report({ url, method, initiator: "fetch", bytes: 0, failed: true }, at);
        throw e;
      }
    };
    restore.push(() => {
      scope.fetch = originalFetch;
    });
  }

  restore.push(...wrapXhr(scope, base, report), ...wrapWebSocket(scope, base, report));
  restore.push(...wrapBeacon(scope, base, report), ...observeResources(scope, report));
  const onViolation = (event: Event) => {
    const e = event as SecurityPolicyViolationEvent;
    // Inline code, eval and Trusted Types violations block no request: only a blocked URL is one.
    if (NOT_REQUESTS.has(e.blockedURI)) return;
    sink.violation({ directive: e.effectiveDirective, blockedUri: e.blockedURI, source, at: Date.now() });
  };
  scope.addEventListener?.("securitypolicyviolation", onViolation);
  restore.push(() => scope.removeEventListener?.("securitypolicyviolation", onViolation));
  return () => {
    for (const undo of restore) undo();
  };
}

/** Reports a request; `at` defaults to now. */
type Report = (record: Omit<NetworkRecord, "source" | "at">, at?: number) => void;

/** `XMLHttpRequest`: the method and URL from `open`, reported once the request ends. */
function wrapXhr(scope: Scope, base: string | undefined, report: Report): (() => void)[] {
  const Xhr = scope.XMLHttpRequest;
  if (Xhr === undefined) return [];
  const { open, send } = Xhr.prototype;
  const seen = new WeakMap<XMLHttpRequest, { method: string; url: string }>();
  Xhr.prototype.open = function (this: XMLHttpRequest, method: string, url: string | URL, ...rest: unknown[]) {
    seen.set(this, { method: method.toUpperCase(), url: urlOf(url, base) });
    return (open as (...args: unknown[]) => void).call(this, method, url, ...rest);
  } as typeof open;
  Xhr.prototype.send = function (this: XMLHttpRequest, body?: Document | XMLHttpRequestBodyInit | null) {
    const call = seen.get(this);
    if (call !== undefined) {
      const at = Date.now();
      this.addEventListener("loadend", () => {
        const bytes = lengthOf(this.getResponseHeader("Content-Length"));
        report({ ...call, initiator: "xmlhttprequest", bytes, failed: this.status === 0 }, at);
      });
    }
    return send.call(this, body);
  };
  return [
    () => {
      Xhr.prototype.open = open;
      Xhr.prototype.send = send;
    },
  ];
}

/** `WebSocket`: each connection attempt, which Resource Timing never lists. */
function wrapWebSocket(scope: Scope, base: string | undefined, report: Report): (() => void)[] {
  const Socket = scope.WebSocket;
  if (Socket === undefined) return [];
  const Wrapped = new Proxy(Socket, {
    construct(target, args: [string | URL, (string | string[])?]) {
      report({ url: urlOf(args[0], base), method: "GET", initiator: "websocket", bytes: 0, failed: false });
      return Reflect.construct(target, args);
    },
  });
  scope.WebSocket = Wrapped;
  return [
    () => {
      scope.WebSocket = Socket;
    },
  ];
}

/** `navigator.sendBeacon` (the page only): each beacon, sent or refused. */
function wrapBeacon(scope: Scope, base: string | undefined, report: Report): (() => void)[] {
  const nav = scope.navigator;
  const beacon = nav?.sendBeacon;
  if (nav === undefined || typeof beacon !== "function") return [];
  nav.sendBeacon = (url: string | URL, data?: BodyInit | null) => {
    const sent = beacon.call(nav, url, data);
    report({ url: urlOf(url, base), method: "POST", initiator: "beacon", bytes: 0, failed: !sent });
    return sent;
  };
  return [
    () => {
      nav.sendBeacon = beacon;
    },
  ];
}

/** What the browser loaded by itself: Resource Timing entries (and the page's own load), buffered. */
function observeResources(scope: Scope, report: Report): (() => void)[] {
  const Observer = scope.PerformanceObserver;
  if (Observer === undefined) return [];
  const types = new Set(Observer.supportedEntryTypes ?? []);
  // A document navigates once, but Chromium reports its navigation entry again when the load ends.
  let navigated = false;
  const take = (entry: PerformanceEntry) => {
    if (entry.entryType === "navigation") {
      if (navigated) return;
      navigated = true;
    }
    const timing = entry as PerformanceResourceTiming;
    if (WRAPPED_INITIATORS.has(timing.initiatorType)) return;
    if (!/^https?:/.test(timing.name)) return;
    const bytes = timing.transferSize || timing.encodedBodySize || 0;
    // When the request started, not when the observer heard of it: a buffered entry comes late.
    const at = Math.round((scope.performance?.timeOrigin ?? Date.now()) + entry.startTime);
    report(
      { url: timing.name, method: "GET", initiator: timing.initiatorType || entry.entryType, bytes, failed: false },
      at,
    );
  };
  const observers: PerformanceObserver[] = [];
  for (const type of ["navigation", "resource"]) {
    if (!types.has(type)) continue;
    const observer = new Observer((list) => {
      for (const entry of list.getEntries()) take(entry);
    });
    observer.observe({ type, buffered: true });
    observers.push(observer);
  }
  return observers.map((observer) => () => observer.disconnect());
}
