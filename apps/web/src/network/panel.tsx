// The network indicator in the status bar and its panel in the inspector (R13), over the
// reader's network log. Shown while a document is open, and whenever a request went to a site
// nobody expected, open or not. Each part subscribes to the log itself, so a request re-renders
// the indicator and the panel, never the whole reader.

import {
  Inspector,
  NetworkActivityPanel,
  NetworkIndicator,
  networkIndicatorText,
  type Platform,
  type ShellLayout,
  useStoreState,
} from "@laminapdf/ui";
import { type ReactNode, useMemo } from "react";
import type { NetworkMonitor, NetworkState, NetworkSummary } from "./monitor.ts";
import { summarize } from "./monitor.ts";
import { networkReport } from "./report.ts";

/** The inspector content id of the network panel. */
export const NETWORK_PANEL = "network";

/** `ms` since the epoch as the panel's "09:12". */
function clock(ms: number): string {
  return new Date(ms).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

/** The host of `url`, when it parses. */
function hostOf(url: string | undefined): string | undefined {
  if (url === undefined) return undefined;
  try {
    return new URL(url).host;
  } catch {
    return undefined;
  }
}

/** The log's summary, worked out again only when the log changes. */
function useSummary(network: NetworkMonitor): { state: NetworkState; summary: NetworkSummary } {
  const state = useStoreState(network, (s) => s);
  // Every change makes a new state object, though its arrays grow in place.
  const summary = useMemo(() => summarize(state, location.origin), [state]);
  return { state, summary };
}

/** Whether the indicator shows: while a document is open, or once an unexpected site was asked. */
function shows(network: NetworkMonitor): boolean {
  const state = network.getState();
  return state.openedAt !== undefined || summarize(state, location.origin).unexpected.length > 0;
}

/** The status bar's count, which opens and closes the panel. */
function NetworkStatus({ layout, network }: { layout: ShellLayout; network: NetworkMonitor }): ReactNode {
  const { state, summary } = useSummary(network);
  const shown = useStoreState(layout, (s) => s.inspector?.id === NETWORK_PANEL);
  const { count, explanation } = networkIndicatorText(
    summary.documentRequests,
    summary.unexpected,
    hostOf(state.documentUrl),
  );
  return (
    <NetworkIndicator
      count={count}
      explanation={explanation}
      warning={summary.unexpected.length > 0}
      aria-expanded={shown}
      onPress={() => (shown ? layout.closeInspector() : layout.openInspector({ id: NETWORK_PANEL, selection: false }))}
    />
  );
}

/** The panel, in the inspector. */
function NetworkInspector({
  layout,
  platform,
  network,
}: {
  layout: ShellLayout;
  platform: Platform;
  network: NetworkMonitor;
}): ReactNode {
  const { state, summary } = useSummary(network);
  const fileHost = hostOf(state.documentUrl);
  const since = state.openedAt === undefined ? undefined : clock(state.openedAt);
  return (
    <Inspector layout={layout}>
      <NetworkActivityPanel
        {...(since === undefined ? {} : { since })}
        documentRequests={summary.documentRequests}
        documentRows={summary.documentRows.map((row) => ({ ...row, unexpected: row.group === "unexpected" }))}
        appRequests={summary.appRequests}
        appRows={summary.appRows}
        blocked={summary.blocked.map((v) => ({ blocked: v.blockedUri, directive: v.directive }))}
        {...(fileHost === undefined ? {} : { fileHost })}
        feeds={{ page: true, worker: state.workerFeed, serviceWorker: false }}
        onClose={() => layout.closeInspector()}
        onCopy={() => void platform.copyText(networkReport(summary, since))}
      />
    </Inspector>
  );
}

/** The network indicator and, while it is open, the panel. */
export function useNetworkPanel(
  layout: ShellLayout,
  platform: Platform,
  network: NetworkMonitor,
): { indicator: ReactNode; inspector: ReactNode } {
  // Two booleans, so the reader re-renders only when the indicator or the panel comes or goes.
  const visible = useStoreState(network, () => shows(network));
  const shown = useStoreState(layout, (s) => s.inspector?.id === NETWORK_PANEL);
  if (!visible) return { indicator: undefined, inspector: undefined };
  const indicator = <NetworkStatus key="network" layout={layout} network={network} />;
  if (!shown) return { indicator, inspector: undefined };
  return { indicator, inspector: <NetworkInspector layout={layout} platform={platform} network={network} /> };
}
