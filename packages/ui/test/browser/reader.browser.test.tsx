import axe from "axe-core";
import { afterEach, describe, expect, it, vi } from "vitest";
import { userEvent } from "vitest/browser";
import {
  DebugMenu,
  type MenuEntry,
  OpenDropZone,
  OpenFailure,
  PasswordDialog,
  PlatformProvider,
  ReaderController,
  ReaderShell,
  ReaderTopBar,
  type ReaderTopBarProps,
  RepairDetails,
  useGatedEntries,
  WholeFileDownload,
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

  it("keeps a disabled page field, still announced, while the document is locked", async () => {
    const host = renderIn(<TopBar reader={await openedController("needs-password")} viewer={new FakeViewer()} />);
    expect(input().getAttribute("aria-disabled")).toBe("true");
    expect(host.querySelector(".paper-page-input-total")?.textContent).toBe("/ –");
    expect(host.querySelector(".paper-zoom-control")).toBeNull();
    expect(button("Sidebar")).toBeUndefined();
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

  it("puts the current page back, announcing nothing, when the field is emptied", async () => {
    const viewer = new FakeViewer();
    renderIn(<TopBar reader={await openedController()} viewer={viewer} />);
    await userEvent.click(input());
    await userEvent.keyboard("{Backspace}{Enter}");
    expect(input().value).toBe("3");
    expect(input().getAttribute("aria-invalid")).toBeNull();
    expect(viewer.calls).toEqual([]);
  });

  it("announces the same mistake twice, and drops it on leaving the field", async () => {
    renderIn(<TopBar reader={await openedController()} viewer={new FakeViewer()} />);
    await userEvent.click(input());
    await userEvent.keyboard("99{Enter}");
    const first = document.querySelector("[role=status] > span");
    await userEvent.keyboard("99{Enter}");
    const second = document.querySelector("[role=status] > span");
    expect(second?.textContent).toBe(first?.textContent);
    expect(second).not.toBe(first);
    await userEvent.tab();
    expect(input().getAttribute("aria-invalid")).toBeNull();
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
  onItems(
    useGatedEntries<"open" | "save">([
      { id: "open", label: "Open another file…" },
      { id: "save", label: "Save a copy…", needs: "canSaveAsDialog" },
    ]),
  );
  return null;
}

describe("useGatedEntries", () => {
  const itemsFor = (platform: ReturnType<typeof fakePlatform>) => {
    let seen: MenuEntry<"open" | "save">[] = [];
    renderIn(
      <PlatformProvider platform={platform}>
        <GatedMore
          onItems={(items) => {
            seen = items;
          }}
        />
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

  it("keeps Open focused and pending as Opening while a try runs", async () => {
    const { reader, tried } = lockedReader("wait");
    renderIn(<PasswordDialog reader={reader} />);
    await reader.open(fakeFile());
    await frame();
    await userEvent.keyboard("hunter2");
    await userEvent.click(button("Open") as HTMLButtonElement);
    await frame();
    const open = button("Opening…") as HTMLButtonElement;
    expect(focused()).toBe(open);
    expect(open.getAttribute("aria-disabled")).toBe("true");
    await userEvent.keyboard("{Enter}");
    expect(tried).toEqual([undefined, "hunter2"]);
  });

  it("puts focus back in the field, text selected, when a clicked Open finds the password wrong", async () => {
    const { reader } = lockedReader("wrong");
    renderIn(<PasswordDialog reader={reader} />);
    await reader.open(fakeFile());
    await frame();
    await userEvent.keyboard("hunter3");
    await userEvent.click(button("Open") as HTMLButtonElement);
    await expect.poll(() => focused()).toBe(passwordField());
    expect([passwordField().selectionStart, passwordField().selectionEnd]).toEqual([0, 7]);
  });

  it("stays open on a click outside it, since the question needs an answer", async () => {
    const { reader } = lockedReader();
    renderIn(<PasswordDialog reader={reader} />);
    await reader.open(fakeFile());
    await frame();
    await userEvent.click(document.body, { position: { x: 5, y: 5 } });
    await frame();
    expect(document.querySelector("[role=dialog]")).not.toBeNull();
    expect(reader.getState().phase).toBe("needs-password");
  });

  it("forgets what was typed when another document opens during a try", async () => {
    const { reader } = lockedReader("wait");
    renderIn(<PasswordDialog reader={reader} />);
    await reader.open(fakeFile("first.pdf"));
    await frame();
    await userEvent.keyboard("first-secret{Enter}");
    await frame();
    await reader.open(fakeFile("second.pdf"));
    await frame();
    expect(document.querySelector("[role=dialog]")?.textContent).toContain("second.pdf is encrypted.");
    expect(passwordField().value).toBe("");
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

/** Drags `files` over the window and drops them, as a file manager would. */
function dropFiles(...files: File[]) {
  const data = new DataTransfer();
  for (const file of files) data.items.add(file);
  const target = document.querySelector(".paper-open-zone") as HTMLElement;
  target.dispatchEvent(new DragEvent("dragenter", { bubbles: true, cancelable: true, dataTransfer: data }));
  target.dispatchEvent(new DragEvent("dragover", { bubbles: true, cancelable: true, dataTransfer: data }));
  return () => target.dispatchEvent(new DragEvent("drop", { bubbles: true, cancelable: true, dataTransfer: data }));
}

describe("OpenDropZone", () => {
  const pdf = new File(["%PDF-1.7\n%…"], "Q3-board-report.pdf", { type: "application/pdf" });

  function home(platform = fakePlatform()) {
    const opened: string[] = [];
    const reader = new ReaderController(platform, async (file) => {
      opened.push(file.name);
      return { pageCount: 1 };
    });
    renderIn(
      <PlatformProvider platform={platform}>
        <OpenDropZone reader={reader} keys="other">
          <p>Lamina PDF reads PDFs inside this browser tab.</p>
        </OpenDropZone>
      </PlatformProvider>,
    );
    return { reader, opened, platform };
  }

  it("is a labelled section whose only stop is Choose a file, which asks the platform", async () => {
    const { platform } = home();
    const section = document.querySelector("section.paper-open") as HTMLElement;
    expect(section.getAttribute("aria-labelledby")).not.toBeNull();
    await userEvent.tab();
    expect(focused().textContent).toBe("Choose a file");
    await userEvent.keyboard("{Enter}");
    expect(platform.calls.openFiles).toEqual([{ multiple: false }]);
  });

  it("covers the window while a file is dragged over it, and opens a dropped PDF", async () => {
    const { opened } = home();
    const drop = dropFiles(pdf);
    await frame();
    expect(document.querySelector(".paper-open-overlay")?.textContent).toBe("Drop to open the PDF");
    drop();
    await expect.poll(() => opened).toEqual(["Q3-board-report.pdf"]);
    expect(document.querySelector(".paper-open-overlay")).toBeNull();
  });

  it("refuses a file that isn't a PDF, with a message, and opens nothing", async () => {
    const { opened } = home();
    dropFiles(new File(["hello"], "notes.pdf.txt", { type: "text/plain" }))();
    await expect
      .poll(() => document.querySelector("[role=alert]")?.textContent)
      .toBe("That isn't a PDF. Lamina PDF opens PDF files only.");
    expect(opened).toEqual([]);
  });

  it("opens a PDF whose header follows a few leading bytes, as many files do", async () => {
    const { opened } = home();
    dropFiles(new File(["﻿junk%PDF-1.4\n"], "scan.pdf"))();
    await expect.poll(() => opened).toEqual(["scan.pdf"]);
  });

  it("passes axe, with the shell's content under it", async () => {
    home();
    const { violations } = await axe.run(document.querySelector("section.paper-open") as HTMLElement, {
      resultTypes: ["violations"],
    });
    expect(violations.map((v) => v.id)).toEqual([]);
  });
});

describe("RepairDetails", () => {
  const repairs = [
    { code: "xref-repaired" },
    { code: "stream-length-recovered", pages: [3, 4, 9] },
    { code: "outline-item-skipped" },
    { code: "content-syntax" },
    { code: "invalid-date" },
    { code: "invalid-box" },
    { code: "a-code-from-the-future" },
  ];

  it("names itself, explains each repair over its code, and shows five kinds until asked for all", async () => {
    const host = renderIn(
      <RepairDetails fileName="Q3-board-report.pdf" repairs={repairs} onClose={() => {}} onCopy={() => {}} />,
    );
    const aside = host.querySelector("aside") as HTMLElement;
    expect(document.getElementById(aside.getAttribute("aria-labelledby") ?? "")?.textContent).toBe("Repaired on open");
    const rows = () => [...host.querySelectorAll(".paper-repair-row")].map((row) => row.textContent);
    expect(rows()).toHaveLength(5);
    expect(rows()[1]).toBe("Measured page data whose stated length was wrongstream-length-recovered · pages 3, 4, 9");
    await userEvent.click(button("Show all 7") as HTMLButtonElement);
    expect(rows()).toHaveLength(7);
    expect(rows()[6]).toBe("Repaired a problem in the filea-code-from-the-future");
  });

  it("closes on Escape or its close button, and copies its details", async () => {
    const onClose = vi.fn();
    const onCopy = vi.fn();
    renderIn(
      <RepairDetails
        fileName="Q3-board-report.pdf"
        repairs={repairs}
        notRun={["This file contains JavaScript. Lamina PDF never runs it."]}
        onClose={onClose}
        onCopy={onCopy}
      />,
    );
    await userEvent.click(button("Copy details") as HTMLButtonElement);
    expect(onCopy).toHaveBeenCalledOnce();
    await userEvent.keyboard("{Escape}");
    await userEvent.click(button("Close repair details") as HTMLButtonElement);
    expect(onClose).toHaveBeenCalledTimes(2);
  });

  it("passes axe", async () => {
    const host = renderIn(
      <RepairDetails
        fileName="Q3-board-report.pdf"
        repairs={repairs}
        notRun={["This file contains JavaScript. Lamina PDF never runs it."]}
        onClose={() => {}}
        onCopy={() => {}}
      />,
    );
    const { violations } = await axe.run(host, { resultTypes: ["violations"] });
    expect(violations.map((v) => v.id)).toEqual([]);
  });
});

describe("OpenFailure", () => {
  it("is announced, says why in words, and focuses Try another file", async () => {
    const onTryAnother = vi.fn();
    const host = renderIn(
      <OpenFailure
        error={{ code: "missing-root", message: "no /Root" }}
        detail="1.8 MB read"
        onTryAnother={onTryAnother}
        onCopy={() => {}}
      />,
    );
    await frame();
    const page = host.querySelector("[role=alert]") as HTMLElement;
    expect(page.querySelector("h2")?.textContent).toBe("This file can't be opened");
    expect(page.textContent).toContain("couldn't find the document's list of pages");
    expect(page.querySelector(".paper-failure-code")?.textContent).toBe("missing-root · 1.8 MB read");
    expect(focused().textContent).toBe("Try another file");
    await userEvent.keyboard("{Enter}");
    expect(onTryAnother).toHaveBeenCalledOnce();
  });

  it("passes axe", async () => {
    const host = renderIn(
      <OpenFailure error={{ code: "not-pdf", message: "" }} onTryAnother={() => {}} onCopy={() => {}} />,
    );
    const { violations } = await axe.run(host, { resultTypes: ["violations"] });
    expect(violations.map((v) => v.id)).toEqual([]);
  });
});

describe("WholeFileDownload", () => {
  it("shows megabytes and the percentage, and cancels", async () => {
    const onCancel = vi.fn();
    const host = renderIn(
      <div style={{ position: "relative", blockSize: "400px" }}>
        <WholeFileDownload loaded={29.8 * 1024 * 1024} total={48 * 1024 * 1024} onCancel={onCancel} />
      </div>,
    );
    const bar = host.querySelector("[role=progressbar]") as HTMLElement;
    expect(bar.getAttribute("aria-valuenow")).toBe("62");
    expect(host.textContent).toContain("29.8 of 48.0 MB");
    await userEvent.click(button("Cancel") as HTMLButtonElement);
    expect(onCancel).toHaveBeenCalledOnce();
  });

  it("is indeterminate when the host didn't say the size", () => {
    const host = renderIn(<WholeFileDownload loaded={3 * 1024 * 1024} onCancel={() => {}} />);
    expect(host.querySelector("[role=progressbar]")?.hasAttribute("aria-valuenow")).toBe(false);
    expect(host.textContent).toContain("3.0 MB");
  });
});

describe("DebugMenu", () => {
  it("lists the backends, checks the current one, and switches", async () => {
    const onBackendChange = vi.fn();
    renderIn(<DebugMenu backend="pdfium" onBackendChange={onBackendChange} keys="other" />);
    await userEvent.click(button("Debug") as HTMLButtonElement);
    await frame();
    const items = [...document.querySelectorAll("[role=menuitem]")];
    expect(items.map((item) => item.textContent)).toEqual(["Render with PDFium", "Render with hayro"]);
    expect(items[0]?.querySelector("svg")).not.toBeNull();
    await userEvent.click(items[1] as HTMLElement);
    expect(onBackendChange).toHaveBeenCalledWith("hayro");
  });
});
