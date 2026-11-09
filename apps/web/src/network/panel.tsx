// The network indicator in the status bar and its panel in the inspector (R13), over the
// reader's network log. Shown while a document is open, and whenever a request went to a site
// nobody expected, open or not.

import {
  Inspector,
  NetworkActivityPanel,
  NetworkIndicator,
  networkIndicatorText,
  type Platform,
  type ShellLayout,
  useStoreState,
} from "@laminapdf/ui";
import type { ReactNode } from "react";
import type { NetworkMonitor } from "./monitor.ts";
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

/** The network indicator and, while it is open, the panel. */
export function useNetworkPanel(
  layout: ShellLayout,
  platform: Platform,
  network: NetworkMonitor,
): { indicator: ReactNode; inspector: ReactNode } {
  const state = useStoreState(network, (s) => s);
  const shown = useStoreState(layout, (s) => s.inspector?.id === NETWORK_PANEL);
  const summary = summarize(state, location.origin);
  if (state.openedAt === undefined && summary.unexpected.length === 0)
    return { indicator: undefined, inspector: undefined };
  const fileHost = hostOf(state.documentUrl);
  const { count, explanation } = networkIndicatorText(summary.documentRequests, summary.unexpected, fileHost);
  const toggle = () =>
    shown ? layout.closeInspector() : layout.openInspector({ id: NETWORK_PANEL, selection: false });
  const indicator = (
    <NetworkIndicator
      key="network"
      count={count}
      explanation={explanation}
      warning={summary.unexpected.length > 0}
      aria-expanded={shown}
      onPress={toggle}
    />
  );
  if (!shown) return { indicator, inspector: undefined };
  const since = clock(state.openedAt ?? Date.now());
  const inspector = (
    <Inspector layout={layout}>
      <NetworkActivityPanel
        since={since}
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
  return { indicator, inspector };
}
