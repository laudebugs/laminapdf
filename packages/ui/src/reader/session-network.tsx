// The desktop session network activity panel: outgoing requests recorded by the app's main
// process (update checks, update downloads and opt-in crash reports). Shares the docked layout,
// typography and table styles with `NetworkActivityPanel`.

import type { ReactNode } from "react";
import { Badge } from "../components/badge.js";
import { Button } from "../components/button.js";
import { message } from "../messages.js";
import { formatBytes } from "./bytes.js";
import { DockedPanel } from "./docked-panel.js";

/** Why the app's main process made an outgoing network request. */
export type SessionNetworkKind = "update-check" | "update-download" | "crash-report";

/** How an outgoing main-process request finished. */
export type SessionNetworkOutcome = "ok" | "failed" | "refused";

/** One outgoing request recorded for the current session: host, kind, time, outcome and size only. */
export interface SessionNetworkEntry {
  readonly kind: SessionNetworkKind;
  readonly host: string;
  readonly timeMs: number;
  readonly outcome: SessionNetworkOutcome;
  readonly bytes: number;
}

/** Props of `SessionNetworkPanel`. */
export interface SessionNetworkPanelProps {
  /** True in the Mac App Store build, whose own code makes no outgoing request. */
  readonly storeBuild: boolean;
  /** Outgoing requests recorded during this session, oldest first. */
  readonly entries: readonly SessionNetworkEntry[];
  /** Whether automatic updates are enabled in this direct build. */
  readonly updatesOn: boolean;
  /** Whether opt-in crash reports are enabled in this direct build. */
  readonly crashReportsOn: boolean;
  /** Closes the panel: its close button and Escape. */
  readonly onClose: () => void;
  /** Copies the plain-text report: the Copy report button. */
  readonly onCopy: () => void;
}

/** Formats `timeMs` as a short clock time ("09:12"). */
function clock(timeMs: number): string {
  return new Date(timeMs).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

/** Human-readable purpose label for `kind`. */
function kindLabel(kind: SessionNetworkKind): string {
  return message(`network.session.kind.${kind}`);
}

/** Human-readable result label for `outcome`. */
function outcomeLabel(outcome: SessionNetworkOutcome): string {
  return message(`network.session.outcome.${outcome}`);
}

/** Status-bar text for the desktop network indicator. */
export function sessionNetworkIndicatorText(
  storeBuild: boolean,
  count: number,
): { readonly count: string; readonly explanation: string } {
  const shown = storeBuild ? 0 : count;
  const label = message(shown === 1 ? "network.session.count.one" : "network.session.count", {
    count: String(shown),
  });
  if (storeBuild) return { count: label, explanation: message("network.session.explain.store") };
  if (shown === 0) return { count: label, explanation: message("network.session.explain.none") };
  return { count: label, explanation: message("network.session.explain.app") };
}

/** Plain-text report of the session's outgoing requests, for Copy report. */
export function sessionNetworkReport(
  props: Pick<SessionNetworkPanelProps, "storeBuild" | "entries" | "updatesOn" | "crashReportsOn">,
): string {
  if (props.storeBuild) {
    return [
      "Lamina PDF network activity (this session)",
      message("network.session.verdict.store"),
      "Outgoing requests: 0",
    ].join("\n");
  }
  const lines = [
    "Lamina PDF network activity (this session)",
    `Outgoing requests: ${props.entries.length}`,
    `Automatic updates: ${props.updatesOn ? "on" : "off"}`,
    `Crash reports: ${props.crashReportsOn ? "on" : "off"}`,
    ...props.entries.map((entry) => {
      const size = entry.bytes > 0 ? formatBytes(entry.bytes) : "—";
      return `  ${clock(entry.timeMs)}  ${entry.host}  ${kindLabel(entry.kind)}  ${size}  ${outcomeLabel(entry.outcome)}`;
    }),
  ];
  return lines.join("\n");
}

/** Summary verdict, badge and explanatory note for the panel's current state. */
function summaryParts(
  props: Pick<SessionNetworkPanelProps, "storeBuild" | "entries" | "updatesOn" | "crashReportsOn">,
): {
  readonly count: number;
  readonly countLabel: string;
  readonly verdict: string;
  readonly badge: ReactNode;
  readonly note: string | undefined;
} {
  if (props.storeBuild) {
    return {
      count: 0,
      countLabel: message("network.session.summary.none"),
      verdict: message("network.session.verdict.store"),
      badge: <Badge tone="positive">{message("network.session.badge.store")}</Badge>,
      note: message("network.session.note.store"),
    };
  }
  if (props.entries.length === 0) {
    const noteKey =
      props.updatesOn && props.crashReportsOn
        ? "network.session.note.bothOn"
        : props.updatesOn
          ? "network.session.note.updatesOn"
          : props.crashReportsOn
            ? "network.session.note.crashesOn"
            : "network.session.note.bothOff";
    return {
      count: 0,
      countLabel: message("network.session.summary.none"),
      verdict: message("network.session.verdict.none"),
      badge: <Badge tone="positive">{message("network.session.badge.none")}</Badge>,
      note: message(noteKey),
    };
  }
  return {
    count: props.entries.length,
    countLabel: message(props.entries.length === 1 ? "network.session.summary.one" : "network.session.summary.many"),
    verdict: message("network.session.verdict.app"),
    badge: <Badge tone="accent">{message("network.session.badge.app")}</Badge>,
    note: undefined,
  };
}

/** Table of outgoing main-process requests recorded in this session. */
function RequestTable({ entries }: { readonly entries: readonly SessionNetworkEntry[] }): ReactNode {
  return (
    <table className="paper-network-table">
      <caption className="paper-repair-overline">{message("network.session.table.title")}</caption>
      <thead>
        <tr>
          <th scope="col">{message("network.column.origin")}</th>
          <th scope="col">{message("network.session.column.time")}</th>
          <th scope="col">{message("network.session.column.size")}</th>
        </tr>
      </thead>
      <tbody>
        {entries.map((entry, index) => (
          // Requests can share a millisecond timestamp; arrival order is their identity.
          // biome-ignore lint/suspicious/noArrayIndexKey: append-only log in arrival order
          <tr key={`${entry.timeMs}-${index}`}>
            <th scope="row">
              <span className="paper-network-origin">{entry.host}</span>
              <span className="paper-network-purpose">
                {`${kindLabel(entry.kind)} · ${outcomeLabel(entry.outcome)}`}
              </span>
            </th>
            <td>{clock(entry.timeMs)}</td>
            <td>{entry.bytes > 0 ? formatBytes(entry.bytes) : "—"}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

/** The desktop app's network activity panel, docked in the inspector. */
export function SessionNetworkPanel(props: SessionNetworkPanelProps): ReactNode {
  const { storeBuild, entries, onClose, onCopy } = props;
  const shownEntries = storeBuild ? [] : entries;
  const summary = summaryParts({ ...props, entries: shownEntries });
  return (
    <DockedPanel
      title={message("network.title")}
      subtitle={message("network.session.subtitle")}
      closeLabel={message("network.close")}
      onClose={onClose}
    >
      <div className="paper-network-summary">
        <span className="paper-network-number">{summary.count}</span>
        <span>{summary.countLabel}</span>
      </div>
      <p className="paper-network-note">{summary.verdict}</p>
      <div>{summary.badge}</div>
      {summary.note === undefined ? null : <p className="paper-network-note">{summary.note}</p>}
      {shownEntries.length === 0 ? null : <RequestTable entries={shownEntries} />}
      <section className="paper-network-section">
        <h3 className="paper-repair-overline">{message("network.session.records.title")}</h3>
        <p className="paper-network-note">
          {message(storeBuild ? "network.session.records.store" : "network.session.records.direct")}
        </p>
      </section>
      <div className="paper-repair-actions">
        <Button icon="copy" onPress={onCopy}>
          {message("network.copy")}
        </Button>
      </div>
    </DockedPanel>
  );
}
