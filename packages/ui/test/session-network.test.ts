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
      explanation: "· Always offline",
    });
    const report = sessionNetworkReport({
      storeBuild: true,
      entries: [],
      updatesOn: false,
      crashReportsOn: false,
    });
    expect(report).toContain("This app has made no network request.");
    expect(report).toContain("Network requests: 0");
    expect(report).not.toContain("Automatic updates");
    expect(report).not.toContain("Crash reports");
  });

  it("clamps invalid indicator counts to zero", () => {
    expect(sessionNetworkIndicatorText(false, -2)).toEqual({
      count: "0 network requests",
      explanation: "· Nothing sent",
    });
    expect(sessionNetworkIndicatorText(false, Number.NaN)).toEqual({
      count: "0 network requests",
      explanation: "· Nothing sent",
    });
    expect(sessionNetworkIndicatorText(false, Number.POSITIVE_INFINITY)).toEqual({
      count: "0 network requests",
      explanation: "· Nothing sent",
    });
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
    expect(offReport).toContain("Automatic updates: off\nCrash reports: off\nOutgoing requests: 0");

    const entries: SessionNetworkEntry[] = [
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
    const directReport = sessionNetworkReport({
      storeBuild: false,
      entries,
      updatesOn: true,
      crashReportsOn: true,
    });
    const lines = directReport.split("\n");
    expect(lines[1]).toBe("Automatic updates: on");
    expect(lines[2]).toBe("Crash reports: on");
    expect(lines[3]).toBe("Outgoing requests: 3");
    expect(lines[4]).toContain("updates.laminapdf.com");
    expect(lines[4]).toContain("Update check");
    expect(lines[4]).toContain("—");
    expect(lines[4]).toContain("OK");
    expect(lines[5]).toContain("Update download");
    expect(lines[5]).toContain("1.5 MB");
    expect(lines[5]).toContain("Failed");
    expect(lines[6]).toContain("Crash report");
    expect(lines[6]).toContain("4.0 KB");
    expect(lines[6]).toContain("Refused");
  });

  it("handles invalid timestamps and non-finite byte counts without NaN or Invalid Date", () => {
    const report = sessionNetworkReport({
      storeBuild: false,
      entries: [
        {
          kind: "update-check",
          host: "updates.laminapdf.com",
          timeMs: 0,
          outcome: "ok",
          bytes: Number.POSITIVE_INFINITY,
        },
        {
          kind: "crash-report",
          host: "updates.laminapdf.com",
          timeMs: Number.MAX_SAFE_INTEGER,
          outcome: "failed",
          bytes: Number.NaN,
        },
      ],
      updatesOn: true,
      crashReportsOn: false,
    });
    expect(report).not.toContain("Invalid Date");
    expect(report).not.toContain("Infinity");
    expect(report).not.toContain("NaN");
    expect(report).toContain("  —  updates.laminapdf.com  Update check  —  OK");
    expect(report).toContain("  —  updates.laminapdf.com  Crash report  —  Failed");
  });
});
