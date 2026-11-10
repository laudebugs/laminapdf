import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, beforeAll, describe, expect, it } from "vitest";
import sample from "../test/app-info.json" with { type: "json" };
import { type AppInfo, loadAppInfo } from "./app-info.ts";

// The Rust side serialises its AppInfo to the same sample (src-tauri/src/commands.rs), and this
// assignment fails to compile if the TypeScript type stops matching it.
const ANSWER: AppInfo = sample;

// Tauri's mocks install themselves on `window`, which Node lacks.
beforeAll(() => {
  Object.assign(globalThis, { window: globalThis });
});
afterEach(() => clearMocks());

describe("loadAppInfo", () => {
  it("returns the app_info command's answer", async () => {
    mockIPC((command) => (command === "app_info" ? ANSWER : undefined));
    await expect(loadAppInfo()).resolves.toEqual(ANSWER);
  });
});
