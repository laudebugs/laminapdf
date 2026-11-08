import axe from "axe-core";
import { useState } from "react";
import { afterEach, describe, expect, it, onTestFinished, vi } from "vitest";
import { userEvent } from "vitest/browser";
import {
  Button,
  CommandPalette,
  type CommandPaletteProps,
  Dialog,
  EmptyState,
  type PaletteSection,
  SelectionToolbar,
  SelectionToolbarDivider,
  TextInput,
} from "../../src/index.ts";
import { cleanup, focused, renderIn } from "./render.tsx";

afterEach(() => {
  cleanup();
  document.documentElement.removeAttribute("data-theme");
});

const frame = () => new Promise((resolve) => requestAnimationFrame(resolve));
const toolbar = () => document.querySelector("[role=toolbar]") as HTMLElement;
const rect = (top: number, left: number) => ({ top, left, width: 200, height: 20 });

function Selection({
  anchor,
  onDismiss = () => {},
}: {
  anchor: () => DOMRect | ReturnType<typeof rect>;
  onDismiss?: () => void;
}) {
  return (
    <>
      <textarea aria-label="Page text" defaultValue="Selected words" />
      <SelectionToolbar aria-label="Selection" anchor={anchor} onDismiss={onDismiss}>
        <Button icon="copy">Copy</Button>
        <Button icon="highlight">Highlight</Button>
        <SelectionToolbarDivider />
        <Button>Comment</Button>
      </SelectionToolbar>
    </>
  );
}

/** A selection the app extends, which moves its toolbar without any scroll. */
function Extending() {
  const [top, setTop] = useState(300);
  return (
    <>
      <Button onPress={() => setTop(500)}>Extend</Button>
      <Selection anchor={() => rect(top, 300)} />
    </>
  );
}

describe("SelectionToolbar", () => {
  it("sits centred above the selection, in the floating-toolbar layer", async () => {
    renderIn(<Selection anchor={() => rect(300, 300)} />);
    await frame();
    const box = toolbar().getBoundingClientRect();
    expect(box.bottom).toBeCloseTo(292, 0);
    expect(box.left + box.width / 2).toBeCloseTo(400, 0);
    expect(toolbar().dataset.side).toBe("above");
    expect(getComputedStyle(toolbar()).zIndex).toBe("20");
    expect(toolbar().getAttribute("aria-label")).toBe("Selection");
  });

  it("flips below when there is no room above, and stays inside the viewport", async () => {
    renderIn(<Selection anchor={() => rect(10, window.innerWidth - 60)} />);
    await frame();
    const box = toolbar().getBoundingClientRect();
    expect(toolbar().dataset.side).toBe("below");
    expect(box.top).toBeCloseTo(38, 0);
    expect(box.right).toBeLessThanOrEqual(window.innerWidth - 8 + 0.5);
  });

  it("follows the selection as its scroller moves", async () => {
    const host = renderIn(
      <div style={{ blockSize: "200px", overflow: "auto" }} data-testid="scroller">
        <div style={{ blockSize: "2000px", paddingBlockStart: "400px" }}>
          <span>Selected words</span>
        </div>
      </div>,
    );
    const words = host.querySelector("span") as HTMLElement;
    const scroller = host.querySelector("[data-testid=scroller]") as HTMLElement;
    renderIn(<Selection anchor={() => words.getBoundingClientRect()} />);
    await frame();
    const before = toolbar().getBoundingClientRect().top;
    scroller.scrollTop = 100;
    scroller.dispatchEvent(new Event("scroll"));
    // Polled: a busy runner can take more than a couple of frames to move the toolbar.
    await expect.poll(() => toolbar().getBoundingClientRect().top, { timeout: 4000 }).toBeCloseTo(before - 100, 0);
  });

  it("takes focus on Shift+F10, moves with the arrows, and Escape hands focus back", async () => {
    const onDismiss = vi.fn();
    const host = renderIn(<Selection anchor={() => rect(300, 300)} onDismiss={onDismiss} />);
    const text = host.querySelector("textarea") as HTMLTextAreaElement;
    text.focus();
    await userEvent.keyboard("{Shift>}{F10}{/Shift}");
    expect(focused().textContent).toBe("Copy");
    await userEvent.keyboard("{ArrowRight}");
    expect(focused().textContent).toBe("Highlight");
    await userEvent.keyboard("{ArrowRight}");
    expect(focused().textContent).toBe("Comment");
    await userEvent.keyboard("{Escape}");
    expect(onDismiss).toHaveBeenCalledOnce();
    expect(focused()).toBe(text);
  });

  it("leaves focus where it is on an Escape after focus left the toolbar", async () => {
    const onDismiss = vi.fn();
    const host = renderIn(
      <>
        <input aria-label="Note" />
        <Selection anchor={() => rect(300, 300)} onDismiss={onDismiss} />
      </>,
    );
    (host.querySelector("textarea") as HTMLTextAreaElement).focus();
    await userEvent.keyboard("{Shift>}{F10}{/Shift}");
    expect(focused().textContent).toBe("Copy");
    const note = host.querySelector("input") as HTMLInputElement;
    note.focus();
    await userEvent.keyboard("{Escape}");
    expect(onDismiss).toHaveBeenCalledOnce();
    expect(focused()).toBe(note);
  });

  it("hides while the selection is out of view, and Shift+F10 then leaves the key alone", async () => {
    const host = renderIn(<Selection anchor={() => rect(-1700, 300)} />);
    await frame();
    expect(toolbar().style.visibility).toBe("hidden");
    const text = host.querySelector("textarea") as HTMLTextAreaElement;
    text.focus();
    let prevented: boolean | undefined;
    const seen = (e: KeyboardEvent) => {
      if (e.key === "F10") prevented = e.defaultPrevented;
    };
    window.addEventListener("keydown", seen);
    onTestFinished(() => window.removeEventListener("keydown", seen));
    await userEvent.keyboard("{Shift>}{F10}{/Shift}");
    expect(prevented).toBe(false);
    expect(focused()).toBe(text);
  });

  it("gives focus back when the selection leaves view while focus is in the toolbar", async () => {
    let top = 300;
    const host = renderIn(<Selection anchor={() => rect(top, 300)} />);
    const text = host.querySelector("textarea") as HTMLTextAreaElement;
    text.focus();
    await userEvent.keyboard("{Shift>}{F10}{/Shift}");
    expect(focused().textContent).toBe("Copy");
    top = -1700;
    window.dispatchEvent(new Event("resize"));
    await frame();
    await frame();
    expect(toolbar().style.visibility).toBe("hidden");
    expect(focused()).toBe(text);
  });

  it("returns focus to where it came from after a Tab into it", async () => {
    const onDismiss = vi.fn();
    const host = renderIn(<Selection anchor={() => rect(300, 300)} onDismiss={onDismiss} />);
    const text = host.querySelector("textarea") as HTMLTextAreaElement;
    text.focus();
    await userEvent.tab();
    expect(focused().textContent).toBe("Copy");
    await userEvent.keyboard("{Escape}");
    expect(onDismiss).toHaveBeenCalledOnce();
    expect(focused()).toBe(text);
  });

  it("leaves Shift+F10 to a modal dialog that is open over it", async () => {
    renderIn(
      <>
        <Selection anchor={() => rect(300, 300)} />
        <Dialog isOpen onOpenChange={() => {}} title="Rename" closeLabel="Close">
          <TextInput label="Name" />
        </Dialog>
      </>,
    );
    await expect.poll(() => focused().tagName).toBe("INPUT");
    const field = focused();
    let prevented: boolean | undefined;
    const seen = (e: KeyboardEvent) => {
      if (e.key === "F10") prevented = e.defaultPrevented;
    };
    window.addEventListener("keydown", seen);
    onTestFinished(() => window.removeEventListener("keydown", seen));
    await userEvent.keyboard("{Shift>}{F10}{/Shift}");
    // The toolbar behind the modal is inert, so it leaves the key alone.
    expect(prevented).toBe(false);
    expect(focused()).toBe(field);
  });

  it("keeps its buttons flat when hovered", async () => {
    renderIn(<Selection anchor={() => rect(300, 300)} />);
    await frame();
    const copy = toolbar().querySelector("button") as HTMLElement;
    await userEvent.hover(copy);
    expect(copy.hasAttribute("data-hovered")).toBe(true);
    // A secondary button's hover lays a tint over the field fill; in the toolbar it is the tint alone.
    expect(getComputedStyle(copy).backgroundImage).toBe("none");
  });

  it("moves when the app passes a new selection, without a scroll", async () => {
    const host = renderIn(<Extending />);
    await frame();
    expect(toolbar().getBoundingClientRect().bottom).toBeCloseTo(292, 0);
    await userEvent.click(host.querySelector("button") as HTMLElement);
    expect(toolbar().getBoundingClientRect().bottom).toBeCloseTo(492, 0);
  });
});

const SECTIONS: PaletteSection[] = [
  {
    id: "recent",
    title: "Recent",
    items: [{ id: "view.fit-width", title: "Fit width", group: "View", matches: [[0, 3]] }],
  },
  {
    id: "commands",
    title: "Commands",
    items: [
      { id: "edit.copy", title: "Copy", group: "Edit", shortcut: { mac: "Mod+C", other: "Mod+C" } },
      { id: "file.export", title: "Export", isDisabled: true, disabledReason: "Nothing to export yet" },
      { id: "view.find", title: "Find", group: "View", shortcut: { mac: "Mod+F", other: "Mod+F" } },
    ],
  },
];

function Palette(props: Partial<CommandPaletteProps>) {
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState(true);
  return (
    <CommandPalette
      isOpen={open}
      onOpenChange={setOpen}
      aria-label="Commands"
      placeholder="Type a command"
      query={query}
      onQueryChange={setQuery}
      sections={SECTIONS}
      onAction={() => {}}
      platform="other"
      footer={<span>↑↓ to move · Enter to run</span>}
      {...props}
    />
  );
}

const input = () => document.querySelector("[role=combobox]") as HTMLInputElement;
const active = () => document.getElementById(input().getAttribute("aria-activedescendant") ?? "");

describe("CommandPalette", () => {
  it("keeps a long list inside the window and scrolls it", async () => {
    const items = Array.from({ length: 60 }, (_, n) => ({ id: `command.${n}`, title: `Command ${n}` }));
    renderIn(<Palette sections={[{ id: "all", title: "All", items }]} />);
    const list = document.querySelector(".paper-palette-list") as HTMLElement;
    const palette = document.querySelector(".paper-palette") as HTMLElement;
    expect(palette.getBoundingClientRect().bottom).toBeLessThanOrEqual(window.innerHeight);
    expect(list.scrollHeight).toBeGreaterThan(list.clientHeight);
    await userEvent.keyboard("{End}");
    // React Aria scrolls the active option into view after the key is handled, a frame later
    // in WebKit and Firefox, so wait for the row to come inside the list.
    await expect
      .poll(
        () =>
          (active()?.getBoundingClientRect().bottom ?? Number.POSITIVE_INFINITY) - list.getBoundingClientRect().bottom,
      )
      .toBeLessThanOrEqual(1);
  });

  it("keeps focus in its combobox input while the arrows move the active row", async () => {
    renderIn(<Palette />);
    await frame();
    expect(focused()).toBe(input());
    expect(input().getAttribute("aria-expanded")).toBe("true");
    const list = document.getElementById(input().getAttribute("aria-controls") ?? "");
    expect(list?.getAttribute("role")).toBe("listbox");
    await userEvent.keyboard("{ArrowDown}");
    expect(focused()).toBe(input());
    expect(active()?.textContent).toContain("Fit width");
    await userEvent.keyboard("{ArrowDown}");
    expect(active()?.textContent).toContain("Copy");
    await userEvent.keyboard("{ArrowDown}");
    expect(active()?.textContent).toContain("Find");
    expect(focused()).toBe(input());
  });

  it("answers each APG combobox key from the input, which keeps focus", async () => {
    renderIn(<Palette />);
    await frame();
    const keys: [string, string][] = [
      ["{ArrowDown}", "Fit width"],
      ["{ArrowDown}", "Copy"],
      // The disabled Export row is skipped, and the arrows stop at either end.
      ["{ArrowDown}", "Find"],
      ["{ArrowDown}", "Find"],
      ["{ArrowUp}", "Copy"],
      ["{Home}", "Fit width"],
      ["{ArrowUp}", "Fit width"],
      ["{End}", "Find"],
      ["{PageUp}", "Fit width"],
      ["{PageDown}", "Find"],
    ];
    for (const [key, row] of keys) {
      await userEvent.keyboard(key);
      expect(focused(), key).toBe(input());
      expect(active()?.textContent, key).toContain(row);
    }
  });

  it("announces a disabled row with its reason, skips it, and labels each section", async () => {
    renderIn(<Palette />);
    await frame();
    const exportRow = [...document.querySelectorAll("[role=option]")].find((o) => o.textContent?.includes("Export"));
    expect(exportRow?.getAttribute("aria-disabled")).toBe("true");
    expect(exportRow?.textContent).toContain("Nothing to export yet");
    const groups = [...document.querySelectorAll("[role=listbox] [role=group]")];
    expect(groups.map((g) => document.getElementById(g.getAttribute("aria-labelledby") ?? "")?.textContent)).toEqual([
      "Recent",
      "Commands",
    ]);
    expect(document.querySelector(".paper-palette-match")?.textContent).toBe("Fit");
  });

  it("lists one command in two sections, and runs it from either", async () => {
    const onAction = vi.fn();
    const copy = { id: "edit.copy", title: "Copy", group: "Edit" };
    const sections = [
      { id: "recent", title: "Recent", items: [copy] },
      { id: "commands", title: "Commands", items: [copy] },
    ];
    renderIn(<Palette onAction={onAction} sections={sections} />);
    await frame();
    const rows = [...document.querySelectorAll("[role=option]")];
    expect(new Set(rows.map((row) => row.id)).size).toBe(2);
    await userEvent.keyboard("{ArrowDown}{ArrowDown}");
    expect(active()).toBe(rows[1]);
    await userEvent.keyboard("{Enter}");
    expect(onAction).toHaveBeenCalledWith("edit.copy");
  });

  it("runs the active row on Enter, and Escape closes it", async () => {
    const onAction = vi.fn();
    renderIn(<Palette onAction={onAction} />);
    await frame();
    await userEvent.keyboard("{ArrowDown}{ArrowDown}{Enter}");
    expect(onAction).toHaveBeenCalledWith("edit.copy");
    await userEvent.keyboard("{Escape}");
    expect(document.querySelector("[role=dialog]")).toBeNull();
  });

  it("passes the query out, shows the empty state, and a page in place of the list", async () => {
    const onQueryChange = vi.fn();
    renderIn(
      <Palette query="" onQueryChange={onQueryChange} sections={[]} empty={<EmptyState title="No commands match" />} />,
    );
    await frame();
    await userEvent.keyboard("zz");
    expect(onQueryChange).toHaveBeenLastCalledWith("z");
    expect(document.querySelector("[role=listbox]")?.textContent).toContain("No commands match");
    cleanup();
    renderIn(<Palette page={<p>Go to page</p>} />);
    await frame();
    expect(document.querySelector("[role=listbox]")).toBeNull();
    expect(document.querySelector("[role=dialog]")?.textContent).toContain("Go to page");
  });

  it("closes on a scrim click below a short list, inside its column", async () => {
    renderIn(<Palette sections={SECTIONS.slice(0, 1)} />);
    await frame();
    const box = (document.querySelector(".paper-palette") as HTMLElement).getBoundingClientRect();
    const overlay = document.querySelector(".paper-palette-overlay") as HTMLElement;
    await userEvent.click(overlay, { position: { x: box.left + box.width / 2, y: box.bottom + 100 } });
    expect(document.querySelector("[role=dialog]")).toBeNull();
  });

  it("shows the empty state, and no overlines, when every section is empty", async () => {
    renderIn(
      <Palette
        sections={[
          { id: "recent", title: "Recent", items: [] },
          { id: "commands", title: "Commands", items: [] },
        ]}
        empty={<EmptyState title="No commands match" />}
      />,
    );
    await frame();
    expect(document.querySelector("[role=listbox]")?.textContent).toContain("No commands match");
    expect(document.querySelectorAll("[role=listbox] [role=group]")).toHaveLength(0);
  });

  it("keeps a disabled row's group without a reason, and merges overlapping matches", async () => {
    renderIn(
      <Palette
        sections={[
          {
            id: "commands",
            title: "Commands",
            items: [
              {
                id: "file.export",
                title: "Export PDF",
                group: "File",
                isDisabled: true,
                matches: [
                  [0, 3],
                  [2, 6],
                ],
              },
            ],
          },
        ]}
      />,
    );
    await frame();
    const row = document.querySelector("[role=option]") as HTMLElement;
    expect(row.querySelector(".paper-palette-group")?.textContent).toBe("File");
    expect([...row.querySelectorAll(".paper-palette-match")].map((m) => m.textContent)).toEqual(["Export"]);
  });

  it("marks the active row with the tint and an inset ring, the focus ring staying on the input", async () => {
    renderIn(<Palette />);
    await frame();
    await userEvent.keyboard("{ArrowDown}");
    const row = active() as HTMLElement;
    expect(getComputedStyle(row).outlineStyle).toBe("none");
    expect(getComputedStyle(row).boxShadow).toContain("inset");
    expect(getComputedStyle(input().closest(".paper-palette-field") ?? input()).outlineStyle).not.toBe("none");
  });

  it("sits 96 px from the top in the modal layer", async () => {
    renderIn(<Palette />);
    await frame();
    expect(document.querySelector(".paper-palette")?.getBoundingClientRect().top).toBeCloseTo(96, 0);
    expect(getComputedStyle(document.querySelector(".paper-palette-overlay") as Element).zIndex).toBe("40");
  });
});

describe.each(["light", "dark", "high-contrast"] as const)("axe, %s theme", (theme) => {
  const check = async (rules: axe.RuleObject = {}) => {
    const result = await axe.run(document.body, {
      runOnly: { type: "tag", values: ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"] },
      rules,
    });
    expect(result.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`)).toEqual([]);
  };

  it("selection toolbar", async () => {
    document.documentElement.dataset.theme = theme;
    renderIn(<Selection anchor={() => rect(300, 300)} />, { theme });
    await frame();
    await check();
  });

  it("command palette", async () => {
    document.documentElement.dataset.theme = theme;
    renderIn(<Palette />, { theme });
    await frame();
    await userEvent.keyboard("{ArrowDown}");
    // As in a menu, a row's group and shortcut chip describe it rather than name it, which axe's
    // experimental label-content-name-mismatch doesn't allow for; the description is checked here.
    for (const row of document.querySelectorAll("[role=option]")) {
      const described = (row.getAttribute("aria-describedby") ?? "")
        .split(" ")
        .map((id) => document.getElementById(id));
      for (const meta of row.querySelectorAll(".paper-palette-meta")) expect(described).toContain(meta);
    }
    await check({ "label-content-name-mismatch": { enabled: false } });
  });
});
