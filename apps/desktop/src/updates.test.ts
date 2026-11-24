// Settings › Updates and the status bar's "Update ready" in each of S06's states, drawn to HTML in
// Node: up to date, checking, downloading, ready, a failed Check now, updates off, managed by an
// organisation, a package manager's install, and a nightly build.

import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import {
  type DesktopUpdates,
  UpdateReadyItem,
  UpdateSettings,
  type UpdateStatus,
  updateActions,
  updatesSummary,
} from "./updates.tsx";

const UP_TO_DATE: UpdateStatus = {
  state: { state: "idle" },
  current: "0.1.2",
  channel: "beta",
  nightly: false,
  enabled: true,
  off: null,
  managed: false,
  canCheck: true,
  stableAvailable: false,
  lastChecked: null,
};

function controls(status: UpdateStatus | undefined): DesktopUpdates {
  return {
    status,
    checkNow: vi.fn(),
    setEnabled: vi.fn(),
    setChannel: vi.fn(),
    resetId: vi.fn(),
    restart: vi.fn(),
    openLink: vi.fn(),
  };
}

/** Settings › Updates for `status`, as HTML. */
function settings(status: Partial<UpdateStatus>): string {
  return renderToStaticMarkup(createElement(UpdateSettings, { updates: controls({ ...UP_TO_DATE, ...status }) }));
}

/** The text of `html`, its tags dropped and its apostrophes unescaped. */
function text(html: string): string {
  return html
    .replace(/<[^>]+>/g, " ")
    .replace(/&#x27;/g, "'")
    .replace(/\s+/g, " ");
}

/** Whether the button labelled `label` in `html` is disabled. */
function disabled(html: string, label: string): boolean {
  const button = [...html.matchAll(/<button([^>]*)>(.*?)<\/button>/g)].find((m) => text(m[2] ?? "").trim() === label);
  if (button === undefined) throw new Error(`no ${label} button`);
  return /\sdisabled(=|\s|$)/.test(button[1] ?? "");
}

describe("Settings › Updates", () => {
  it("says the app is up to date and offers Check now, the switch and Beta", () => {
    const html = settings({});
    expect(disabled(html, "Check now")).toBe(false);
    expect(text(html)).toContain("You have Lamina PDF 0.1.2");
    expect(text(html)).toContain("Check now");
    expect(text(html)).toContain("Not checked yet");
    expect(text(html)).toContain("Reset anonymous ID");
    expect(html).toContain('value="beta"');
  });

  it("says up to date only once a check has answered", () => {
    expect(text(settings({ lastChecked: 1_794_710_891 }))).toContain("Lamina PDF 0.1.2 is up to date");
    expect(text(settings({}))).not.toContain("up to date");
  });

  it("shows Stable disabled, with why, until the first stable release", () => {
    expect(text(settings({}))).toContain("Stable (not released yet)");
    expect(text(settings({ stableAvailable: true }))).not.toContain("not released yet");
  });

  it("shows the download's progress and turns Check now off while it runs", () => {
    const html = settings({ state: { state: "downloading", version: "0.2.0", percent: 62 } });
    expect(text(html)).toContain("Downloading Lamina PDF 0.2.0");
    expect(html).toContain('role="progressbar"');
    expect(html).toContain('aria-valuenow="62"');
    expect(disabled(html, "Check now")).toBe(true);
  });

  it("offers Restart to update once the update is ready", () => {
    const html = settings({ state: { state: "ready", version: "0.2.0", notes: null } });
    expect(text(html)).toContain("Lamina PDF 0.2.0 is ready");
    expect(text(html)).toContain("Restart to update");
  });

  it("says when Check now couldn't reach the server", () => {
    expect(text(settings({ state: { state: "failed" } }))).toContain("Couldn't reach the update server");
  });

  it("says when the user turned updates off, and keeps Check now", () => {
    const html = settings({ enabled: false, off: "user" });
    expect(text(html)).toContain("Automatic updates are off");
    expect(disabled(html, "Check now")).toBe(false);
  });

  it("disables everything and says so when an organisation manages updates", () => {
    const html = settings({ off: "policy", managed: true, canCheck: false });
    expect(text(html)).toContain("Managed by your organisation");
    expect(text(html)).toContain("Your organisation has turned updates off");
    expect(disabled(html, "Check now")).toBe(true);
  });

  it("names the pinned channel when an organisation chose it", () => {
    expect(text(settings({ managed: true, channel: "beta" }))).toContain(
      "Your organisation keeps this install on the Beta channel",
    );
  });

  it("hands a package manager's install to the package manager and hides the controls", () => {
    const html = settings({ off: "package-manager" });
    expect(text(html)).toContain("Updates come from your package manager");
    expect(text(html)).not.toContain("Automatic updates");
    expect(text(html)).not.toContain("Check now");
  });

  it("shows a nightly build's channel as Nightly, read-only", () => {
    const html = settings({ nightly: true, channel: "nightly" });
    expect(text(html)).toContain("Nightly builds are a separate install");
    expect(html).not.toContain('role="radiogroup"');
  });
});

describe("the status bar's Update ready", () => {
  it("appears only when an update is ready", () => {
    const item = (status: UpdateStatus) =>
      renderToStaticMarkup(createElement(UpdateReadyItem, { updates: controls(status) }));
    // Only the empty live region, there before the words it will announce.
    expect(text(item(UP_TO_DATE)).trim()).toBe("");
    expect(item(UP_TO_DATE)).toContain('aria-live="polite"');
    expect(text(item({ ...UP_TO_DATE, state: { state: "downloading", version: "0.2.0", percent: 10 } })).trim()).toBe(
      "",
    );
    const ready = item({ ...UP_TO_DATE, state: { state: "ready", version: "0.2.0", notes: null } });
    expect(text(ready)).toContain("Update ready");
    expect(ready).toContain('aria-live="polite"');
    // The live region is the first element in both states, so React keeps the same one.
    expect(item(UP_TO_DATE).startsWith('<span class="paper-visually-hidden" aria-live="polite">')).toBe(true);
    expect(ready.startsWith('<span class="paper-visually-hidden" aria-live="polite">')).toBe(true);
  });
});

describe("updatesSummary", () => {
  it("sums the section up in one line", () => {
    expect(updatesSummary(UP_TO_DATE)).toBe("Automatic, Beta channel");
    expect(updatesSummary({ ...UP_TO_DATE, off: "user", enabled: false })).toBe("Automatic updates off");
    expect(updatesSummary({ ...UP_TO_DATE, managed: true })).toBe("Managed by your organisation");
    expect(updatesSummary({ ...UP_TO_DATE, off: "package-manager" })).toBe("From your package manager");
    expect(updatesSummary(undefined)).toBe("");
  });
});

describe("the Mac App Store build", () => {
  const store: UpdateStatus = { ...UP_TO_DATE, enabled: false, off: "app-store", canCheck: false };
  const actionsFor = (status: UpdateStatus | undefined, inShell = true) =>
    Object.keys(updateActions(inShell, controls(status), vi.fn()));

  it("offers no action that checks for or installs an update", () => {
    expect(actionsFor(store)).toEqual([]);
    expect(actionsFor({ ...store, state: { state: "ready", version: "0.2.0", notes: null } })).toEqual([]);
  });

  it("shows no Check now, channel or automatic-update control in Settings › Updates", () => {
    const html = settings(store);
    expect(text(html)).toContain("Updates come from the App Store");
    expect(html).not.toContain("<button");
    expect(html).not.toContain("<input");
    expect(html).not.toContain('role="switch"');
  });

  it("still offers Check for updates, and Restart to update once one is ready, in the direct build", () => {
    expect(actionsFor(UP_TO_DATE)).toEqual(["app.check-for-updates"]);
    expect(actionsFor({ ...UP_TO_DATE, state: { state: "ready", version: "0.2.0", notes: null } })).toEqual([
      "app.check-for-updates",
      "app.restart-to-update",
    ]);
    expect(text(settings({}))).toContain("Check now");
  });

  it("offers nothing outside the shell or before the main process answers", () => {
    expect(actionsFor(UP_TO_DATE, false)).toEqual([]);
    expect(actionsFor(undefined)).toEqual([]);
  });
});
