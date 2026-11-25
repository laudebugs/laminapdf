// Desktop session network activity panel: Mac App Store empty state, all four direct-build
// zero-request combinations, recorded entries (update check/download, crash report, ok/failed/refused),
// Copy report / Close actions, and accessibility audit.

import axe from "axe-core";
import { afterEach, describe, expect, it, vi } from "vitest";
import { page } from "vitest/browser";
import { type SessionNetworkEntry, SessionNetworkPanel, type SessionNetworkPanelProps } from "../../src/index.ts";
import { cleanup, renderIn } from "./render.tsx";

afterEach(() => cleanup());

const ENTRIES: readonly SessionNetworkEntry[] = [
  {
    kind: "update-check",
    host: "updates.laminapdf.com",
    timeMs: 1_700_000_000_000,
    outcome: "ok",
    bytes: 0,
  },
  {
    kind: "update-download",
    host: "updates.laminapdf.com",
    timeMs: 1_700_000_030_000,
    outcome: "failed",
    bytes: 1_572_864,
  },
  {
    kind: "crash-report",
    host: "updates.laminapdf.com",
    timeMs: 1_700_000_060_000,
    outcome: "refused",
    bytes: 4096,
  },
];

function show(overrides: Partial<SessionNetworkPanelProps> = {}) {
  const onClose = vi.fn();
  const onCopy = vi.fn();
  const host = renderIn(
    <SessionNetworkPanel
      storeBuild={false}
      entries={[]}
      updatesOn={false}
      crashReportsOn={false}
      onClose={onClose}
      onCopy={onCopy}
      {...overrides}
    />,
  );
  return { host, onClose, onCopy };
}

describe("SessionNetworkPanel", () => {
  it("renders the Mac App Store state with zero requests even if entries are passed", async () => {
    const { host, onClose, onCopy } = show({
      storeBuild: true,
      entries: ENTRIES,
      updatesOn: true,
      crashReportsOn: true,
    });
    await expect.element(page.getByText("This app has made no network request.")).toBeVisible();
    await expect
      .element(
        page.getByText(
          "Your PDFs stay on your computer. The Mac App Store handles updates and macOS handles crash reports, so this copy makes no requests of its own.",
        ),
      )
      .toBeVisible();
    await expect.element(page.getByRole("table")).not.toBeInTheDocument();

    await page.getByRole("button", { name: "Copy report" }).click();
    await page.getByRole("button", { name: "Close network activity" }).click();
    expect(onCopy).toHaveBeenCalledTimes(1);
    expect(onClose).toHaveBeenCalledTimes(1);

    const results = await axe.run(host);
    expect(results.violations).toEqual([]);
  });

  it("explains all four direct-build empty combinations of automatic updates and crash reports", async () => {
    show({ updatesOn: false, crashReportsOn: false });
    await expect
      .element(
        page.getByText(
          "Your PDFs stay on your computer. Automatic updates and crash reports are off, so the app has sent nothing.",
        ),
      )
      .toBeVisible();
    cleanup();

    show({ updatesOn: true, crashReportsOn: false });
    await expect
      .element(
        page.getByText(
          "Your PDFs stay on your computer. Automatic updates are on, and crash reports are off until you turn them on.",
        ),
      )
      .toBeVisible();
    cleanup();

    show({ updatesOn: false, crashReportsOn: true });
    await expect
      .element(
        page.getByText(
          "Your PDFs stay on your computer. Automatic updates are off, and the app sends a report only after a crash.",
        ),
      )
      .toBeVisible();
    cleanup();

    show({ updatesOn: true, crashReportsOn: true });
    await expect
      .element(
        page.getByText(
          "Your PDFs stay on your computer. Automatic updates are on, and the app sends a report only after a crash.",
        ),
      )
      .toBeVisible();
  });

  it("renders recorded entries with host, purpose, outcome, size and passes axe", async () => {
    const { host } = show({ entries: ENTRIES, updatesOn: true, crashReportsOn: true });
    await expect.element(page.getByText("App requests only")).toBeVisible();
    const table = page.getByRole("table", { name: "Outgoing requests" });
    await expect.element(table).toBeVisible();
    await expect.element(table.getByText("Update check · OK", { exact: false })).toBeVisible();
    await expect.element(table.getByText("Update download · Failed", { exact: false })).toBeVisible();
    await expect.element(table.getByText("1.5 MB")).toBeVisible();
    await expect.element(table.getByText("Crash report · Refused", { exact: false })).toBeVisible();
    await expect.element(table.getByText("4.0 KB")).toBeVisible();

    const results = await axe.run(host);
    expect(results.violations).toEqual([]);
  });
});
