import axe from "axe-core";
import { afterEach, describe, expect, it } from "vitest";
import { userEvent } from "vitest/browser";
import { Demo } from "../../examples/ui-demo.tsx";
import { PlatformProvider } from "../../src/index.ts";
import { fakePlatform } from "../fakes.ts";
import { cleanup, renderIn } from "./render.tsx";

afterEach(cleanup);

const frame = () => new Promise((resolve) => requestAnimationFrame(resolve));

/** The demo with the picker answering `name`, in a viewport-tall host. */
function demo(name: string) {
  const file = new File(["%PDF-1.7\n"], name, { type: "application/pdf" });
  const platform = fakePlatform({
    picked: [{ name, size: file.size, read: async () => new Uint8Array([37, 80, 68, 70, 45]), writable: false }],
  });
  return renderIn(
    <div style={{ blockSize: "700px" }}>
      <PlatformProvider platform={platform}>
        <Demo platform={platform} debug />
      </PlatformProvider>
    </div>,
  );
}

/** Serious and critical axe violations on the whole page, overlays included. */
async function violations(): Promise<string[]> {
  const { violations: found } = await axe.run(document.body, { resultTypes: ["violations"] });
  return found.filter((v) => v.impact === "serious" || v.impact === "critical").map((v) => v.id);
}

async function choose() {
  await userEvent.click(
    [...document.querySelectorAll("button")].find((b) => b.textContent === "Choose a file") as HTMLElement,
  );
  await frame();
}

describe("the demo page", () => {
  it("passes axe on the home screen", async () => {
    demo("report.pdf");
    expect(await violations()).toEqual([]);
  });

  it("passes axe with a damaged document open and its repairs docked", async () => {
    demo("damaged.pdf");
    await choose();
    await expect.poll(() => document.querySelector("aside")).not.toBeNull();
    expect(document.querySelector(".paper-top-bar-name")?.textContent).toBe("damaged.pdf");
    expect(await violations()).toEqual([]);
  });

  it("passes axe while asking for a password, and opens with the right one", async () => {
    demo("locked.pdf");
    await choose();
    await expect.poll(() => document.querySelector("[role=dialog]")).not.toBeNull();
    expect(await violations()).toEqual([]);
    await userEvent.keyboard("lamina{Enter}");
    await expect.poll(() => document.querySelector("[role=dialog]")).toBeNull();
    expect(document.querySelector(".paper-page-input-field")).not.toBeNull();
  });

  it("passes axe on the failure page", async () => {
    demo("broken.pdf");
    await choose();
    await expect.poll(() => document.querySelector("[role=alert]")).not.toBeNull();
    expect(await violations()).toEqual([]);
  });
});
