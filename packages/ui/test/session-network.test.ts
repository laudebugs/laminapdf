import { describe, expect, it } from "vitest";
import {
  type SessionNetworkEntry,
  sessionNetworkIndicatorText,
  sessionNetworkReport,
} from "../src/reader/session-network.tsx";

describe("session network panel helpers", () => {
  it("reports the Mac App Store empty state without implying any outgoing request", () => {
    expect(sessionNetworkIndicatorText(true, 3)).toEqual({
      count: "0 network requests",
      explanation: "· No network requests",
    });
    const report = sessionNetworkReport({
      storeBuild: true,
      entries: [],
      updatesOn: false,
      crashReportsOn: false,
    });
    expect(report).toContain("This app has made no network request.");
    expect(report).toContain("Outgoing requests: 0");
    expect(report).not.toContain("Automatic updates");
    expect(report).not.toContain("Crash reports");
  });

  it("reports direct build states with updates and crash reports off or with entries", () => {
    expect(sessionNetworkIndicatorText(false, 0)).toEqual({
      count: "0 network requests",
      explanation: "· Nothing sent",
    });
    expect(sessionNetworkIndicatorText(false, 1)).toEqual({
      count: "1 network request",
      explanation: "· App only",
    });
    expect(sessionNetworkIndicatorText(false, 2)).toEqual({
      count: "2 network requests",
      explanation: "· App only",
    });

    const offReport = sessionNetworkReport({
      storeBuild: false,
      entries: [],
      updatesOn: false,
      crashReportsOn: false,
    });
    expect(offReport).toContain("Outgoing requests: 0");
    expect(offReport).toContain("Automatic updates: off");
    expect(offReport).toContain("Crash reports: off");

    const entries: SessionNetworkEntry[] = [
      {
        kind: "update-check",
        host: "updates.laminapdf.com",
        timeMs: 1_700_000_000_000,
        outcome: "ok",
        bytes: 0,
      },
      {
        kind: "crash-report",
        host: "updates.laminapdf.com",
        timeMs: 1_700_000_060_000,
        outcome: "ok",
        bytes: 4096,
      },
    ];
    const directReport = sessionNetworkReport({
      storeBuild: false,
      entries,
      updatesOn: true,
      crashReportsOn: true,
    });
    expect(directReport).toContain("Outgoing requests: 2");
    expect(directReport).toContain("updates.laminapdf.com");
    expect(directReport).toContain("Update check");
    expect(directReport).toContain("Crash report");
    expect(directReport).toContain("4.0 KB");
    expect(directReport).toContain("OK");
  });
});
