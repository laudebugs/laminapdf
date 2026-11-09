// The network activity panel (R13): what left the device since the file opened. Document traffic
// by origin, the app's own files apart, blocked requests, which contexts were counted, what the
// count proves and how to check it yourself. The app counts and classifies; this only shows it.

import { type ReactNode, useId, useState } from "react";
import { Badge } from "../components/badge.js";
import { Button, IconButton } from "../components/button.js";
import { Notice } from "../components/notice.js";
import { message } from "../messages.js";

/** One origin's requests since the file opened. */
export interface NetworkOriginRow {
  readonly origin: string;
  readonly requests: number;
  readonly bytes: number;
  /** An origin neither the app's nor the open file's: nothing should have asked it for anything. */
  readonly unexpected: boolean;
}

/** One kind of app file: the WebAssembly modules, scripts, styles, fonts … */
export interface NetworkAppRow {
  /** Its purpose, as a `network.app.*` message names it: `webassembly`, `script`, `style`, `font`, `image`, `page`, `other`. */
  readonly purpose: string;
  readonly requests: number;
  readonly bytes: number;
}

/** A request a Content Security Policy blocked: nothing was sent. */
export interface NetworkBlockedRow {
  readonly blocked: string;
  readonly directive: string;
}

/** Props of `NetworkActivityPanel`. */
export interface NetworkActivityPanelProps {
  /** When the file opened, as the panel shows it ("09:12"). */
  since: string;
  /** Requests that weren't the app's own files, since the file opened. */
  documentRequests: number;
  documentRows: readonly NetworkOriginRow[];
  appRequests: number;
  appRows: readonly NetworkAppRow[];
  blocked: readonly NetworkBlockedRow[];
  /** The open file's host, when it came from a link. */
  fileHost?: string;
  /** Which contexts were counted: the page always, the document worker once its log arrived. */
  feeds: { readonly page: boolean; readonly worker: boolean; readonly serviceWorker: boolean };
  onClose: () => void;
  onCopy: () => void;
}

/** `bytes` in the units a reader expects. */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) return message("network.bytes", { value: String(bytes) });
  if (bytes < 1024 * 1024) return message("network.kb", { value: (bytes / 1024).toFixed(1) });
  return message("network.mb", { value: (bytes / (1024 * 1024)).toFixed(1) });
}

/** The summary's sentence and badge, from what was sent and where. */
function verdict(props: NetworkActivityPanelProps): { text: string; badge: ReactNode } {
  const unexpected = props.documentRows.filter((row) => row.unexpected);
  if (unexpected.length > 0) {
    return {
      text: message("network.verdict.unexpected"),
      badge: <Badge tone="warning">{message("network.badge.unexpected")}</Badge>,
    };
  }
  if (props.documentRequests === 0) {
    return {
      text: message("network.verdict.none"),
      badge: <Badge tone="positive">{message("network.badge.none")}</Badge>,
    };
  }
  return {
    text: message("network.verdict.host", { host: props.fileHost ?? "" }),
    badge: <Badge tone="accent">{message("network.badge.host")}</Badge>,
  };
}

/** The status-bar words for a count: "0 network requests for this document" and what it means. */
export function networkIndicatorText(
  documentRequests: number,
  unexpected: readonly string[],
  fileHost: string | undefined,
): { count: string; explanation: string } {
  const count = message(documentRequests === 1 ? "network.count.one" : "network.count", {
    count: String(documentRequests),
  });
  if (unexpected.length > 0)
    return { count, explanation: message("network.explain.unexpected", { origin: unexpected.join(", ") }) };
  if (documentRequests === 0) return { count, explanation: message("network.explain.none") };
  return { count, explanation: message("network.explain.host", { host: fileHost ?? "" }) };
}

/** The panel's document-traffic table. */
function DocumentTable({ rows }: { rows: readonly NetworkOriginRow[] }): ReactNode {
  return (
    <table className="paper-network-table">
      <caption className="paper-network-overline">{message("network.document.title")}</caption>
      <thead>
        <tr>
          <th scope="col">{message("network.column.origin")}</th>
          <th scope="col">{message("network.column.requests")}</th>
          <th scope="col">{message("network.column.received")}</th>
        </tr>
      </thead>
      <tbody>
        {rows.map((row) => (
          <tr key={row.origin} data-unexpected={row.unexpected || undefined}>
            <th scope="row">
              {/* R13 names a site by its host: https:// is every one's scheme, and http:// is refused. */}
              <span className="paper-network-origin">{row.origin.replace(/^https:\/\//, "")}</span>
              <span className="paper-network-purpose">
                {message(row.unexpected ? "network.purpose.unexpected" : "network.purpose.document")}
              </span>
            </th>
            <td>{row.requests}</td>
            <td>{formatBytes(row.bytes)}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

/** The app files table, behind a disclosure. */
function AppFiles({ rows, total, open }: { rows: readonly NetworkAppRow[]; total: number; open: boolean }): ReactNode {
  const [shown, setShown] = useState(open);
  const id = useId();
  return (
    <section className="paper-network-section">
      <button
        type="button"
        className="paper-network-disclosure"
        aria-expanded={shown}
        aria-controls={id}
        onClick={() => setShown((now) => !now)}
      >
        {message("network.app.title", { count: String(total) })}
      </button>
      <p className="paper-network-note">{message("network.app.note")}</p>
      <table id={id} className="paper-network-table" hidden={!shown}>
        <caption className="paper-visually-hidden">{message("network.app.caption")}</caption>
        <thead>
          <tr>
            <th scope="col">{message("network.column.files")}</th>
            <th scope="col">{message("network.column.requests")}</th>
            <th scope="col">{message("network.column.received")}</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr key={row.purpose}>
              <th scope="row">{message(`network.app.${row.purpose}` as Parameters<typeof message>[0])}</th>
              <td>{row.requests}</td>
              <td>{formatBytes(row.bytes)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  );
}

/** The network activity panel, docked in the inspector. */
export function NetworkActivityPanel(props: NetworkActivityPanelProps): ReactNode {
  const { since, documentRequests, documentRows, appRows, appRequests, blocked, feeds, onClose, onCopy } = props;
  const title = useId();
  const [howTo, setHowTo] = useState(false);
  const { text, badge } = verdict(props);
  const unexpected = documentRows.filter((row) => row.unexpected).map((row) => row.origin);
  return (
    <aside
      className="paper-network"
      aria-labelledby={title}
      onKeyDown={(event) => {
        if (event.key !== "Escape") return;
        event.stopPropagation();
        onClose();
      }}
    >
      <header className="paper-repair-header">
        <div>
          <h2 id={title} className="paper-repair-title">
            {message("network.title")}
          </h2>
          <p className="paper-repair-file">{message("network.since", { time: since })}</p>
        </div>
        <IconButton icon="close" label={message("network.close")} onPress={onClose} />
      </header>
      {unexpected.length === 0 ? null : (
        <Notice tone="warning" title={message("network.unexpected.title")}>
          {message("network.unexpected.body", { origin: unexpected.join(", ") })}
        </Notice>
      )}
      <div className="paper-network-summary">
        <span className="paper-network-number">{documentRequests}</span>
        <span>{message("network.summary.label")}</span>
      </div>
      <p className="paper-network-note">{text}</p>
      <div>{badge}</div>
      {documentRows.length === 0 ? null : <DocumentTable rows={documentRows} />}
      <AppFiles rows={appRows} total={appRequests} open={documentRequests === 0} />
      {blocked.length === 0 ? null : (
        <section className="paper-network-section">
          <h3 className="paper-network-overline">{message("network.blocked.title")}</h3>
          <ul className="paper-repair-list">
            {blocked.map((row, index) => (
              // Blocked requests can repeat; their order is their identity.
              // biome-ignore lint/suspicious/noArrayIndexKey: rows never reorder
              <li key={index} className="paper-repair-row">
                <span className="paper-network-origin">{row.blocked || message("network.blocked.inline")}</span>
                <span className="paper-network-purpose">
                  <Badge>{message("network.blocked.badge")}</Badge> {row.directive}
                </span>
              </li>
            ))}
          </ul>
        </section>
      )}
      <section className="paper-network-section">
        <h3 className="paper-network-overline">{message("network.feeds.title")}</h3>
        <ul className="paper-repair-list">
          <li>{message(feeds.page ? "network.feed.page" : "network.feed.page.off")}</li>
          <li>{message(feeds.worker ? "network.feed.worker" : "network.feed.worker.off")}</li>
          <li>{message(feeds.serviceWorker ? "network.feed.sw" : "network.feed.sw.off")}</li>
        </ul>
      </section>
      <section className="paper-network-section">
        <h3 className="paper-network-overline">{message("network.proves.title")}</h3>
        <p className="paper-network-note">{message("network.proves.body")}</p>
        <p className="paper-network-note">{message("network.proves.limit")}</p>
      </section>
      <div className="paper-repair-actions">
        <Button icon="copy" onPress={onCopy}>
          {message("network.copy")}
        </Button>
        <Button variant="ghost" aria-expanded={howTo} onPress={() => setHowTo((now) => !now)}>
          {message("network.howTo")}
        </Button>
      </div>
      {howTo ? (
        <ol className="paper-network-steps">
          <li>{message("network.howTo.step1")}</li>
          <li>{message("network.howTo.step2")}</li>
          <li>{message("network.howTo.step3")}</li>
        </ol>
      ) : null}
    </aside>
  );
}
