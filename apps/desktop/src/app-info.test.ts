import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeAll, describe, expect, it } from "vitest";
import { loadAppInfo } from "./app-info.ts";

// Tauri's mocks install themselves on `window`, which Node lacks.
beforeAll(() => {
  Object.assign(globalThis, { window: globalThis });
});
afterEach(() => clearMocks());

describe("loadAppInfo", () => {
  it("calls the app_info command and returns its answer", async () => {
    const calls: string[] = [];
    const answer = { version: "0.2.0", os: "macos", arch: "aarch64", webviewVersion: "1" };
    mockIPC((command) => {
      calls.push(command);
      return answer;
    });
    await expect(loadAppInfo()).resolves.toEqual(answer);
    expect(calls).toEqual(["app_info"]);
  });
});
