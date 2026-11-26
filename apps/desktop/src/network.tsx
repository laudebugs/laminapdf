// The desktop app's network activity panel and status-bar indicator: reads the main process's
// session request log (`network_activity` / `network://changed`), opens the docked inspector panel
// for `privacy.network-activity`, and copies the plain-text session report.

import {
  type CommandContext,
  Inspector,
  message,
  NetworkIndicator,
  type Platform,
  type SessionNetworkEntry,
  SessionNetworkPanel,
  type ShellLayout,
  sessionNetworkIndicatorText,
  sessionNetworkReport,
  type ToastQueue,
  useStoreState,
} from "@laminapdf/ui";
import { type ReactNode, useCallback, useEffect, useMemo, useState } from "react";
import type { CrashSetting } from "./crashes.tsx";
import type { DesktopShell } from "./engine/desktop-engine-client.ts";
import type { UpdateStatus } from "./updates.tsx";

/** The inspector content id of the network activity panel. */
export const NETWORK_PANEL = "network";

/** The main process's event broadcast whenever the session request log changes (`network.rs`). */
export const NETWORK_CHANGED_EVENT = "network://changed";

/** Snapshot handed to the webview by `network_activity` and `network://changed` (`network.rs`). */
export interface NetworkSnapshot {
  /** Monotonic sequence number incremented on every recorded request, rejecting stale snapshots. */
  readonly seq: number;
  /** True in the Mac App Store build, whose own code makes no outgoing request. */
  readonly storeBuild: boolean;
  /** Outgoing requests recorded during this session, oldest first. */
  readonly entries: readonly SessionNetworkEntry[];
}

const EMPTY_SNAPSHOT: NetworkSnapshot = { seq: 0, storeBuild: false, entries: [] };

/** Command handler for `privacy.network-activity`, enabled with or without a document. */
export function networkActions(layout: ShellLayout): NonNullable<CommandContext["actions"]> {
  return {
    "privacy.network-activity": () => layout.openInspector({ id: NETWORK_PANEL, selection: false }),
  };
}

/** Whether automatic updates are currently on for this direct build. */
export function areUpdatesOn(status: UpdateStatus | undefined): boolean {
  return status !== undefined && status.off === null;
}

/** Whether opt-in crash reports are currently on for this direct build. */
export function areCrashReportsOn(setting: CrashSetting | undefined): boolean {
  return setting?.available === true && setting.enabled;
}

/** Subscribes to `network://changed` and reads the current snapshot from `network_activity`. */
export function subscribeNetworkActivity(
  shell: DesktopShell,
  onSnapshot: (snapshot: NetworkSnapshot) => void,
  onError: (error: unknown) => void = reportError,
): () => void {
  let stop: (() => void) | undefined;
  let gone = false;
  let lastSeq = -1;
  const deliver = (snapshot: NetworkSnapshot) => {
    if (gone || snapshot.seq <= lastSeq) return;
    lastSeq = snapshot.seq;
    onSnapshot(snapshot);
  };
  const fail = (error: unknown) => {
    if (!gone) onError(error);
  };
  shell.listen<NetworkSnapshot>(NETWORK_CHANGED_EVENT, deliver).then((unlisten) => {
    if (gone) {
      unlisten();
      return;
    }
    stop = unlisten;
    shell.invoke<NetworkSnapshot>("network_activity").then(deliver, fail);
  }, fail);
  return () => {
    gone = true;
    stop?.();
  };
}

/** What the desktop network activity hook hands to the window. */
export interface DesktopNetwork {
  /** Command handler for `privacy.network-activity`. */
  readonly actions: NonNullable<CommandContext["actions"]>;
  /** The status-bar indicator that toggles the docked panel. */
  readonly indicator: ReactNode;
  /** The docked inspector panel while open, or `undefined`. */
  readonly inspector: ReactNode;
}

/**
 * Follows the main process's session network log and wires `privacy.network-activity`, the
 * status-bar indicator, and the docked inspector panel.
 */
export function useDesktopNetwork(
  shell: DesktopShell | undefined,
  layout: ShellLayout,
  platform: Platform,
  updateStatus: UpdateStatus | undefined,
  crashSetting: CrashSetting | undefined,
  toasts: ToastQueue,
): DesktopNetwork {
  const [snapshot, setSnapshot] = useState<NetworkSnapshot>(EMPTY_SNAPSHOT);
  useEffect(() => {
    if (shell === undefined) return;
    return subscribeNetworkActivity(shell, setSnapshot);
  }, [shell]);

  const shown = useStoreState(layout, (s) => s.inspector?.id === NETWORK_PANEL);
  const actions = useMemo(() => networkActions(layout), [layout]);
  const storeBuild = snapshot.storeBuild || updateStatus?.off === "app-store";
  const updatesOn = areUpdatesOn(updateStatus);
  const crashReportsOn = areCrashReportsOn(crashSetting);
  const { count, explanation } = sessionNetworkIndicatorText(storeBuild, snapshot.entries.length);

  const toggle = useCallback(() => {
    if (shown) layout.closeInspector();
    else layout.openInspector({ id: NETWORK_PANEL, selection: false });
  }, [shown, layout]);

  const copyReport = useCallback(() => {
    const text = sessionNetworkReport({
      storeBuild,
      entries: snapshot.entries,
      updatesOn,
      crashReportsOn,
    });
    void Promise.resolve()
      .then(() => platform.copyText(text))
      .then(
        () => toasts.add({ message: message("shell.copied") }),
        (error: unknown) => reportError(error),
      );
  }, [storeBuild, snapshot.entries, updatesOn, crashReportsOn, platform, toasts]);

  const indicator = <NetworkIndicator count={count} explanation={explanation} aria-expanded={shown} onPress={toggle} />;
  const inspector = shown ? (
    <Inspector layout={layout}>
      <SessionNetworkPanel
        storeBuild={storeBuild}
        entries={snapshot.entries}
        updatesOn={updatesOn}
        crashReportsOn={crashReportsOn}
        onClose={() => layout.closeInspector()}
        onCopy={copyReport}
      />
    </Inspector>
  ) : undefined;

  return { actions, indicator, inspector };
}

function reportError(error: unknown): void {
  console.error("network activity failed", error);
}
