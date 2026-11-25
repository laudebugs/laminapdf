// Desktop network activity: command action, IPC subscription, and settings-to-panel helpers.

import { ShellLayout } from "@laminapdf/ui";
import { describe, expect, it, vi } from "vitest";
import type { DesktopShell } from "./engine/desktop-engine-client.ts";
import {
  areCrashReportsOn,
  areUpdatesOn,
  NETWORK_CHANGED_EVENT,
  NETWORK_PANEL,
  type NetworkSnapshot,
  networkActions,
  subscribeNetworkActivity,
} from "./network.tsx";
import type { UpdateStatus } from "./updates.tsx";

function status(overrides: Partial<UpdateStatus> = {}): UpdateStatus {
  return {
    state: { state: "idle" },
    current: "0.1.0",
    channel: "beta",
    nightly: false,
    enabled: true,
    off: null,
    managed: false,
    canCheck: true,
    stableAvailable: false,
    lastChecked: null,
    ...overrides,
  };
}

describe("desktop network activity", () => {
  it("opens the network inspector panel when privacy.network-activity runs", () => {
    const layout = new ShellLayout();
    const actions = networkActions(layout);
    expect(layout.getState().inspector).toBeUndefined();
    actions["privacy.network-activity"]?.(undefined);
    expect(layout.getState().inspector).toEqual({ id: NETWORK_PANEL, selection: false });
  });

  it("derives automatic update and crash report flags accurately", () => {
    expect(areUpdatesOn(undefined)).toBe(false);
    expect(areUpdatesOn(status({ off: null }))).toBe(true);
    expect(areUpdatesOn(status({ off: "user", enabled: false }))).toBe(false);
    expect(areUpdatesOn(status({ off: "app-store", enabled: false }))).toBe(false);

    expect(areCrashReportsOn(undefined)).toBe(false);
    expect(areCrashReportsOn({ available: true, enabled: true })).toBe(true);
    expect(areCrashReportsOn({ available: true, enabled: false })).toBe(false);
    expect(areCrashReportsOn({ available: false, enabled: true })).toBe(false);
  });

  it("loads the current snapshot and follows network://changed events until unsubscribed", async () => {
    let handler: ((snapshot: NetworkSnapshot) => void) | undefined;
    const unlisten = vi.fn();
    const initial: NetworkSnapshot = {
      storeBuild: false,
      entries: [
        {
          kind: "update-check",
          host: "updates.laminapdf.com",
          timeMs: 1_700_000_000_000,
          outcome: "ok",
          bytes: 0,
        },
      ],
    };
    const shell = {
      invoke: vi.fn(async () => initial),
      listen: vi.fn(async (_event: string, cb: (snapshot: NetworkSnapshot) => void) => {
        handler = cb;
        return unlisten;
      }),
    } as unknown as DesktopShell;

    const seen: NetworkSnapshot[] = [];
    const stop = subscribeNetworkActivity(shell, (snap) => seen.push(snap));
    await vi.waitFor(() => expect(seen).toHaveLength(1));
    expect(shell.invoke).toHaveBeenCalledWith("network_activity");
    expect(shell.listen).toHaveBeenCalledWith(NETWORK_CHANGED_EVENT, expect.any(Function));
    expect(seen[0]).toEqual(initial);

    const updated: NetworkSnapshot = {
      storeBuild: false,
      entries: [
        ...initial.entries,
        {
          kind: "crash-report",
          host: "updates.laminapdf.com",
          timeMs: 1_700_000_060_000,
          outcome: "ok",
          bytes: 2048,
        },
      ],
    };
    handler?.(updated);
    expect(seen).toHaveLength(2);
    expect(seen[1]).toEqual(updated);

    stop();
    expect(unlisten).toHaveBeenCalledTimes(1);
  });
});
