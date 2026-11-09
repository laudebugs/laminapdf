// R07 in the chrome: the external-link confirmation (the full address with its host apart, Cancel
// focused, Open and Copy) and the top bar's Back and Forward, named by where they go.
import axe from "axe-core";
import { afterEach, describe, expect, it } from "vitest";
import { userEvent } from "vitest/browser";
import type { ReaderState } from "../../src/index.ts";
import {
  CommandRegistry,
  CommandsProvider,
  ExternalLinkDialog,
  ReaderTopBar,
  registerReaderCommands,
} from "../../src/index.ts";
import { FakeViewer, fakeFile, fakePlatform } from "../fakes.ts";
import { cleanup, renderIn } from "./render.tsx";

afterEach(cleanup);

const holding = (state: ReaderState) => ({ getState: () => state, subscribe: () => () => undefined });

const LINK = {
  href: "https://surveys.example.org/2026/capacity",
  parts: { before: "https://", host: "surveys.example.org", after: "/2026/capacity" },
};

function dialog() {
  const said: string[] = [];
  renderIn(
    <ExternalLinkDialog
      link={LINK}
      onOpen={(href) => said.push(`open ${href}`)}
      onCopy={(href) => said.push(`copy ${href}`)}
      onClose={() => said.push("close")}
    />,
  );
  const element = document.querySelector<HTMLElement>('[role="dialog"]');
  if (element === null) throw new Error("no dialog");
  return { said, element };
}

const button = (root: HTMLElement, name: string) =>
  [...root.querySelectorAll("button")].find((b) => b.textContent === name || b.getAttribute("aria-label") === name);

describe("ExternalLinkDialog", () => {
  it("shows the full address with the host in bold, Cancel focused", async () => {
    const { element } = dialog();
    expect(element.querySelector(".paper-external-link-address")?.textContent).toBe(LINK.href);
    expect(element.querySelector(".paper-external-link-address strong")?.textContent).toBe("surveys.example.org");
    await expect.poll(() => document.activeElement?.textContent).toBe("Cancel");
    const results = await axe.run(element);
    expect(results.violations.map((v) => [v.id, v.nodes.map((n) => n.failureSummary)])).toEqual([]);
  });

  it("opens the address, then closes", async () => {
    const { said, element } = dialog();
    await userEvent.click(button(element, "Open website") as HTMLElement);
    expect(said).toEqual([`open ${LINK.href}`, "close"]);
  });

  it("copies the address and stays open", async () => {
    const { said, element } = dialog();
    await userEvent.click(button(element, "Copy link") as HTMLElement);
    expect(said).toEqual([`copy ${LINK.href}`]);
  });

  it("closes on Cancel and Escape without opening anything", async () => {
    const { said, element } = dialog();
    await userEvent.click(button(element, "Cancel") as HTMLElement);
    await userEvent.keyboard("{Escape}");
    expect(said.every((s) => s === "close")).toBe(true);
    expect(said.length).toBeGreaterThan(0);
  });
});

function topBar(history: { back: string | undefined; forward: string | undefined }) {
  const ran: string[] = [];
  const registry = new CommandRegistry();
  registerReaderCommands(registry);
  const reader = {
    ...holding({ phase: "open", file: fakeFile("report.pdf"), pageCount: 24, mode: "select" }),
    openFromPicker: async () => {},
    submitPassword: async () => {},
  };
  const viewer = new FakeViewer();
  const context = {
    platform: fakePlatform(),
    reader,
    viewer,
    actions: {
      "nav.back": () => void ran.push("back"),
      "nav.forward": () => void ran.push("forward"),
    },
  };
  const host = renderIn(
    <CommandsProvider registry={registry} context={context} keys="other">
      <ReaderTopBar
        reader={reader}
        viewer={viewer}
        keys="other"
        history={history}
        more={{ items: [], onAction: () => {} }}
      />
    </CommandsProvider>,
  );
  return { host, ran };
}

describe("the top bar's history", () => {
  it("names Back by where it goes and runs nav.back", async () => {
    const { host, ran } = topBar({ back: "ii", forward: undefined });
    const group = host.querySelector('[role="group"][aria-label="History"]') as HTMLElement;
    const back = button(group, "Back to page ii") as HTMLButtonElement;
    expect(back.disabled).toBe(false);
    expect((button(group, "Forward") as HTMLButtonElement).disabled).toBe(true);
    await userEvent.click(back);
    expect(ran).toEqual(["back"]);
  });

  it("disables both with nowhere to go", () => {
    const { host } = topBar({ back: undefined, forward: undefined });
    const group = host.querySelector('[role="group"]') as HTMLElement;
    expect([...group.querySelectorAll("button")].map((b) => b.disabled)).toEqual([true, true]);
  });
});
