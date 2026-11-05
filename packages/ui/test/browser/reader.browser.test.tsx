import axe from "axe-core";
import { useState } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { userEvent } from "vitest/browser";
import {
  type MenuEntry,
  PasswordDialog,
  PlatformProvider,
  ReaderController,
  ReaderShell,
  ReaderTopBar,
  type ReaderTopBarProps,
  useGatedEntry,
} from "../../src/index.ts";
import { FakeViewer, fakeFile, fakePlatform } from "../fakes.ts";
import { cleanup, focused, renderIn } from "./render.tsx";

afterEach(cleanup);

const frame = () => new Promise((resolve) => requestAnimationFrame(resolve));
const MORE: MenuEntry<"open" | "rotate">[] = [
  { id: "open", label: "Open another file…" },
  { id: "rotate", label: "Rotate clockwise", icon: "rotate-right" },
];

/** A controller that opened `report.pdf` (24 pages), or is still on its way there. */
async function openedController(phase: "open" | "opening" | "needs-password" = "open") {
  const never = new Promise<{ pageCount: number }>(() => undefined);
  const opener = () => {
    if (phase === "opening") return never;
    if (phase === "needs-password") return Promise.reject(Object.assign(new Error(), { code: "password-required" }));
    return Promise.resolve({ pageCount: 24 });
  };
  const controller = new ReaderController(fakePlatform(), opener);
  const done = controller.open(fakeFile("Q3-board-report.pdf"));
  if (phase !== "opening") await done;
  return controller;
}

function TopBar(props: Partial<ReaderTopBarProps<"open" | "rotate">> & { reader: ReaderController }) {
  return <ReaderTopBar keys="other" more={{ items: MORE, onAction: () => {} }} {...props} />;
}

const input = () => document.querySelector(".paper-page-input-field") as HTMLInputElement;
const button = (name: string) =>
  [...document.querySelectorAll("button")].find((b) => b.getAttribute("aria-label") === name || b.textContent === name);

describe("ReaderTopBar", () => {
  it("shows the file name, and Loading pages or Locked only while they apply", async () => {
    const host = renderIn(<TopBar reader={await openedController("opening")} />);
    expect(host.querySelector(".paper-top-bar-name")?.textContent).toBe("Q3-board-report.pdf");
    expect(host.querySelector(".paper-top-bar-note")?.textContent).toBe("Loading pages…");
    cleanup();
    const locked = renderIn(<TopBar reader={await openedController("needs-password")} />);
    expect(locked.querySelector(".paper-top-bar-note")?.textContent).toBe("Locked");
    cleanup();
    const open = renderIn(<TopBar reader={await openedController()} viewer={new FakeViewer()} />);
    expect(open.querySelector(".paper-top-bar-note")).toBeNull();
  });

  it("shows the shell's home content, and no document controls, without a document", () => {
    const controller = new ReaderController(fakePlatform(), async () => ({ pageCount: 1 }));
    const host = renderIn(<TopBar reader={controller} home={<span>lamina</span>} viewer={new FakeViewer()} />);
    expect(host.querySelector(".paper-top-bar-file")?.textContent).toBe("lamina");
    expect(host.querySelector(".paper-page-input")).toBeNull();
  });

  it("toggles the sidebar with aria-pressed", async () => {
    const onChange = vi.fn();
    renderIn(<TopBar reader={await openedController()} sidebar={{ isOpen: true, onChange }} />);
    const toggle = button("Sidebar") as HTMLButtonElement;
    expect(toggle.getAttribute("aria-pressed")).toBe("true");
    await userEvent.click(toggle);
    expect(onChange).toHaveBeenCalledWith(false);
  });

  it("shows Commands with its shortcut only when the shell gives a palette", async () => {
    const onPress = vi.fn();
    const shortcut = { mac: "Mod+K", other: "Mod+K" };
    const host = renderIn(<TopBar reader={await openedController()} commands={{ onPress, shortcut }} />);
    const commands = host.querySelector(".paper-commands-button") as HTMLButtonElement;
    expect(commands.textContent).toBe("CommandsCtrl K");
    await userEvent.click(commands);
    expect(onPress).toHaveBeenCalledOnce();
  });

  it("runs More's items, which open from the keyboard", async () => {
    const onAction = vi.fn();
    renderIn(<TopBar reader={await openedController()} more={{ items: MORE, onAction }} />);
    (button("More") as HTMLButtonElement).focus();
    await userEvent.keyboard("{Enter}");
    await frame();
    await userEvent.keyboard("{ArrowDown}{Enter}");
    expect(onAction).toHaveBeenCalledWith("rotate");
  });

  it("passes axe", async () => {
    const host = renderIn(
      <TopBar
        reader={await openedController()}
        viewer={new FakeViewer()}
        sidebar={{ isOpen: false, onChange: () => {} }}
        commands={{ onPress: () => {}, shortcut: { mac: "Mod+K", other: "Mod+K" } }}
      />,
    );
    const { violations } = await axe.run(host, { resultTypes: ["violations"] });
    expect(violations.map((v) => v.id)).toEqual([]);
  });
});

describe("PageInput", () => {
  it("shows the current page and the count, and follows the viewer", async () => {
    const viewer = new FakeViewer({ currentPage: 2 });
    const host = renderIn(<TopBar reader={await openedController()} viewer={viewer} />);
    expect(input().value).toBe("3");
    expect(host.querySelector(".paper-page-input-total")?.textContent).toBe("/ 24");
    viewer.set({ currentPage: 6 });
    await frame();
    expect(input().value).toBe("7");
  });

  it("goes to a typed page on Enter", async () => {
    const viewer = new FakeViewer();
    renderIn(<TopBar reader={await openedController()} viewer={viewer} />);
    await userEvent.click(input());
    await userEvent.keyboard("12{Enter}");
    expect(viewer.calls).toEqual([{ method: "goToPage", value: 11 }]);
  });

  it("announces a page that doesn't exist, and goes back to the current one", async () => {
    const viewer = new FakeViewer();
    renderIn(<TopBar reader={await openedController()} viewer={viewer} />);
    await userEvent.click(input());
    await userEvent.keyboard("99{Enter}");
    expect(viewer.calls).toEqual([]);
    expect(input().value).toBe("3");
    expect(input().getAttribute("aria-invalid")).toBe("true");
    const status = document.getElementById(input().getAttribute("aria-describedby") ?? "");
    expect(status?.getAttribute("role")).toBe("status");
    expect(status?.textContent).toBe("There is no page 99. Enter a page from 1 to 24.");
  });

  it("puts the current page back on Escape, without closing anything around it", async () => {
    const onKeyDown = vi.fn();
    renderIn(
      // biome-ignore lint/a11y/noStaticElementInteractions: listens for an Escape that should never reach it
      <div onKeyDown={(e) => e.key === "Escape" && onKeyDown()}>
        <TopBar reader={await openedController()} viewer={new FakeViewer()} />
      </div>,
    );
    await userEvent.click(input());
    await userEvent.keyboard("5{Escape}");
    expect(input().value).toBe("3");
    expect(onKeyDown).not.toHaveBeenCalled();
  });

  it("shows a page label with its position, and goes to a typed label", async () => {
    const labels = [...Array.from({ length: 21 }, (_, n) => String(n + 1)), "A-1", "A-2", "A-3"];
    const viewer = new FakeViewer({ currentPage: 21 });
    const host = renderIn(<TopBar reader={await openedController()} viewer={viewer} labels={labels} />);
    expect(input().value).toBe("A-1");
    expect(host.querySelector(".paper-page-input-total")?.textContent).toBe("(22 / 24)");
    await userEvent.click(input());
    await userEvent.keyboard("a-3{Enter}");
    expect(viewer.calls).toEqual([{ method: "goToPage", value: 23 }]);
  });
});

describe("ZoomControl", () => {
  it("shows the zoom and steps it with its buttons", async () => {
    const viewer = new FakeViewer({ percent: 110 });
    renderIn(<TopBar reader={await openedController()} viewer={viewer} />);
    expect(button("Zoom, 110%")?.textContent).toBe("110%");
    await userEvent.click(button("Zoom in") as HTMLButtonElement);
    await userEvent.click(button("Zoom out") as HTMLButtonElement);
    expect(viewer.calls).toEqual([
      { method: "setZoom", value: 125 },
      { method: "setZoom", value: 100 },
    ]);
  });

  it("sets a fit mode or a preset from its menu, by keyboard", async () => {
    const viewer = new FakeViewer();
    renderIn(<TopBar reader={await openedController()} viewer={viewer} />);
    (button("Zoom, 110%") as HTMLButtonElement).focus();
    await userEvent.keyboard("{Enter}");
    await frame();
    expect(focused().textContent).toBe("Fit width");
    await userEvent.keyboard("{Enter}");
    (button("Zoom, 110%") as HTMLButtonElement).focus();
    await userEvent.keyboard("{Enter}");
    await frame();
    await userEvent.keyboard("{End}{Enter}");
    expect(viewer.calls).toEqual([
      { method: "setZoom", value: "fit-width" },
      { method: "setZoom", value: 400 },
    ]);
  });

  it("can't step past the viewer's range", async () => {
    const viewer = new FakeViewer({ percent: 800 });
    renderIn(<TopBar reader={await openedController()} viewer={viewer} />);
    expect((button("Zoom in") as HTMLButtonElement).disabled).toBe(true);
    viewer.set({ percent: 10 });
    await frame();
    expect((button("Zoom out") as HTMLButtonElement).disabled).toBe(true);
  });
});

/** A More menu whose Save item needs the save dialog. */
function GatedMore({ onItems }: { onItems: (items: MenuEntry<"open" | "save">[]) => void }) {
  const save = useGatedEntry("canSaveAsDialog", { id: "save", label: "Save a copy…" });
  const [items] = useState(() => [{ id: "open" as const, label: "Open another file…" }]);
  onItems(save === undefined ? items : [...items, save]);
  return null;
}

describe("useGatedEntry", () => {
  const itemsFor = (platform: ReturnType<typeof fakePlatform>) => {
    let seen: MenuEntry<"open" | "save">[] = [];
    renderIn(
      <PlatformProvider platform={platform}>
        <GatedMore onItems={(items) => (seen = items)} />
      </PlatformProvider>,
    );
    return seen;
  };

  it("keeps an item the platform can do, disables one it could, and leaves out one it never can", () => {
    expect(itemsFor(fakePlatform({ flags: { canSaveAsDialog: true } })).at(-1)).toEqual({
      id: "save",
      label: "Save a copy…",
    });
    const reason = "Use a browser that can save to your files.";
    expect(itemsFor(fakePlatform({ reasons: { canSaveAsDialog: reason } })).at(-1)).toMatchObject({
      isDisabled: true,
      disabledReason: reason,
    });
    expect(itemsFor(fakePlatform()).map((item) => item.id)).toEqual(["open"]);
  });
});

describe("ReaderShell", () => {
  it("lays out the slots with the page's landmarks, the canvas taking the space left", async () => {
    const host = renderIn(
      <div style={{ blockSize: "600px" }}>
        <ReaderShell
          topBar={<TopBar reader={await openedController()} />}
          rail={<nav aria-label="Pages">Pages</nav>}
          inspector={<aside aria-label="Repair details">Details</aside>}
          status={<span>0 network requests for this document</span>}
        >
          <div data-testid="viewer" />
        </ReaderShell>
      </div>,
    );
    expect(host.querySelector("header")?.className).toBe("paper-top-bar");
    expect(host.querySelector("main [data-testid=viewer]")).not.toBeNull();
    expect(host.querySelector("footer")?.textContent).toBe("0 network requests for this document");
    const canvas = (host.querySelector("main") as HTMLElement).getBoundingClientRect();
    const rail = (host.querySelector(".paper-reader-rail") as HTMLElement).getBoundingClientRect();
    const inspector = (host.querySelector(".paper-reader-inspector") as HTMLElement).getBoundingClientRect();
    expect(rail.width).toBeCloseTo(220, 0);
    expect(inspector.width).toBeCloseTo(288, 0);
    expect(canvas.left).toBeCloseTo(rail.right, 0);
    expect(canvas.right).toBeCloseTo(inspector.left, 0);
    expect(canvas.height).toBeGreaterThan(400);
  });
});

/** A reader whose document needs a password: `answers` are the opener's replies to each try. */
function lockedReader(...answers: ("wrong" | "open" | "wait")[]) {
  const tried: (string | undefined)[] = [];
  const opener = (_file: unknown, password: string | undefined) => {
    tried.push(password);
    if (password === undefined) return Promise.reject(Object.assign(new Error(), { code: "password-required" }));
    const answer = answers.shift();
    if (answer === "wait") return new Promise<{ pageCount: number }>(() => undefined);
    if (answer === "wrong") return Promise.reject(Object.assign(new Error(), { code: "wrong-password" }));
    return Promise.resolve({ pageCount: 24 });
  };
  return { reader: new ReaderController(fakePlatform(), opener), tried };
}

const passwordField = () => document.querySelector(".paper-password-input") as HTMLInputElement;

describe("PasswordDialog", () => {
  it("asks for the password when the document needs one, focused on the field", async () => {
    const { reader } = lockedReader();
    renderIn(<PasswordDialog reader={reader} />);
    expect(document.querySelector("[role=dialog]")).toBeNull();
    await reader.open(fakeFile("Q3-board-report.pdf"));
    await frame();
    const dialog = document.querySelector("[role=dialog]") as HTMLElement;
    expect(dialog.getAttribute("aria-labelledby")).not.toBeNull();
    expect(dialog.textContent).toContain("Q3-board-report.pdf is encrypted.");
    expect(focused()).toBe(passwordField());
    expect(passwordField().type).toBe("password");
    expect(passwordField().autocomplete).toBe("current-password");
  });

  it("sends the password on Enter, shows Opening while it is tried, and closes once open", async () => {
    const { reader, tried } = lockedReader("open");
    renderIn(<PasswordDialog reader={reader} />);
    await reader.open(fakeFile());
    await frame();
    await userEvent.keyboard("hunter2{Enter}");
    await frame();
    expect(tried).toEqual([undefined, "hunter2"]);
    await expect.poll(() => document.querySelector("[role=dialog]")).toBeNull();
    expect(reader.getState().phase).toBe("open");
  });

  it("keeps Open disabled as Opening while a try runs", async () => {
    const { reader } = lockedReader("wait");
    renderIn(<PasswordDialog reader={reader} />);
    await reader.open(fakeFile());
    await frame();
    await userEvent.keyboard("hunter2{Enter}");
    await frame();
    const open = button("Opening…") as HTMLButtonElement;
    expect(open.disabled).toBe(true);
  });

  it("says a password was wrong under the field, which keeps focus with its text selected", async () => {
    const { reader } = lockedReader("wrong");
    renderIn(<PasswordDialog reader={reader} />);
    await reader.open(fakeFile());
    await frame();
    await userEvent.keyboard("hunter3{Enter}");
    await expect.poll(() => document.querySelector("[role=alert]")?.textContent).toContain("didn't open the file");
    const field = passwordField();
    expect(field.getAttribute("aria-invalid")).toBe("true");
    expect(document.getElementById(field.getAttribute("aria-describedby") ?? "")?.getAttribute("role")).toBe("alert");
    expect(focused()).toBe(field);
    expect([field.selectionStart, field.selectionEnd]).toEqual([0, 7]);
  });

  it("shows the password while Show is pressed", async () => {
    const { reader } = lockedReader();
    renderIn(<PasswordDialog reader={reader} />);
    await reader.open(fakeFile());
    await frame();
    const show = button("Show password") as HTMLButtonElement;
    expect(show.textContent).toBe("Show");
    await userEvent.click(show);
    expect(show.getAttribute("aria-pressed")).toBe("true");
    expect(passwordField().type).toBe("text");
  });

  it.each(["Cancel", "Escape"])("goes back to the home screen on %s, forgetting what was typed", async (how) => {
    const { reader } = lockedReader();
    renderIn(<PasswordDialog reader={reader} />);
    await reader.open(fakeFile());
    await frame();
    await userEvent.keyboard("secret");
    if (how === "Cancel") await userEvent.click(button("Cancel") as HTMLButtonElement);
    else await userEvent.keyboard("{Escape}");
    await expect.poll(() => document.querySelector("[role=dialog]")).toBeNull();
    expect(reader.getState().phase).toBe("empty");
    await reader.open(fakeFile());
    await frame();
    expect(passwordField().value).toBe("");
  });

  it("passes axe, wrong password shown", async () => {
    const { reader } = lockedReader("wrong");
    renderIn(<PasswordDialog reader={reader} />);
    await reader.open(fakeFile());
    await frame();
    await userEvent.keyboard("x{Enter}");
    await expect.poll(() => document.querySelector("[role=alert]")).not.toBeNull();
    const { violations } = await axe.run(document.querySelector("[role=dialog]") as HTMLElement, {
      resultTypes: ["violations"],
    });
    expect(violations.map((v) => v.id)).toEqual([]);
  });
});
