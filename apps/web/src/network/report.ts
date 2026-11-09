// The network panel's Copy report: the summary as plain text, one line per origin and per kind of
// app file, for an issue or a message.

import type { NetworkSummary } from "./monitor.ts";

/** The plain-text report of `summary`, for a file opened at `since`. */
export function networkReport(summary: NetworkSummary, since: string): string {
  const lines = [
    `Lamina PDF network activity since ${since}`,
    `Document requests: ${summary.documentRequests}`,
    ...summary.documentRows.map(
      (row) =>
        `  ${row.origin}: ${row.requests} requests, ${row.bytes} bytes${row.group === "unexpected" ? " (unexpected)" : ""}`,
    ),
    `App files: ${summary.appRequests} requests`,
    ...summary.appRows.map((row) => `  ${row.purpose}: ${row.requests} requests, ${row.bytes} bytes`),
    `Blocked by the page's policy: ${summary.blocked.length}`,
    ...summary.blocked.map((v) => `  ${v.directive}: ${v.blockedUri || "inline"} (${v.source})`),
    `Total requests since the page loaded: ${summary.total}`,
  ];
  return lines.join("\n");
}
