import axe from "axe-core";
import { type ReactNode, useState } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { userEvent } from "vitest/browser";
import {
  Button,
  Dialog,
  type DialogProps,
  IconButton,
  Menu,
  MenuTrigger,
  Popover,
  PopoverTrigger,
  Radio,
  RadioGroup,
  TextInput,
  ToastQueue,
  ToastRegion,
  Tooltip,
} from "../../src/index.ts";
import { cleanup, focused, renderIn } from "./render.tsx";

afterEach(() => {
  cleanup();
  document.documentElement.removeAttribute("data-theme");
});

// Focus returns to a menu's or popover's button as it closes; WebKit does it a frame later, so
// those checks poll.
const role = (name: string) => document.querySelector(`[role=${name}]`) as HTMLElement | null;
const frame = () => new Promise((resolve) => requestAnimationFrame(resolve));

type Options = Partial<Pick<DialogProps, "role" | "returnFocusTo">> & {
  required?: boolean;
  children?: ReactNode;
  /** The opener unmounts once pressed, as a control can that goes away while its dialog is open. */
  vanish?: boolean;
};

function Opener({ required = false, children, vanish = false, ...rest }: Options) {
  const [open, setOpen] = useState(false);
  const [used, setUsed] = useState(false);
  const closing = required ? { isRequired: true as const } : { closeLabel: "Close" };
  return (
    <>
      {vanish && used ? null : (
        <Button
          onPress={() => {
            setUsed(true);
            setOpen(true);
          }}
        >
          Move pages
        </Button>
      )}
      <Button>Outside</Button>
      <Dialog
        {...rest}
        {...closing}
        isOpen={open}
        onOpenChange={setOpen}
        title="Move pages"
        description="Move pages 4–6 to a new position."
        actions={
          <>
            <Button onPress={() => setOpen(false)}>Cancel</Button>
            <Button variant="primary" onPress={() => setOpen(false)}>
              Move
            </Button>
          </>
        }
      >
        <TextInput label="Move after page" defaultValue="12" />
        {children}
      </Dialog>
    </>
  );
}

function Radios() {
  return (
    <Dialog isOpen onOpenChange={() => {}} title="Print" closeLabel="Close">
      <RadioGroup label="Pages" defaultValue="current">
        <Radio value="all">All pages</Radio>
        <Radio value="current">Current page</Radio>
      </RadioGroup>
    </Dialog>
  );
}

/** A dialog that closes by unmounting. */
function Conditional({ fallback }: { fallback: HTMLElement }) {
  const [open, setOpen] = useState(true);
  return open ? (
    <Dialog isOpen onOpenChange={setOpen} title="Move" closeLabel="Close" returnFocusTo={() => fallback} />
  ) : null;
}

async function openDialog(options: Options = {}) {
  renderIn(<Opener {...options} />);
  await userEvent.tab();
  await userEvent.keyboard("{Enter}");
  return role(options.role ?? "dialog") as HTMLElement;
}

describe("Dialog", () => {
  it("is a labelled, described modal dialog that starts focus on its first field", async () => {
    const dialog = await openDialog();
    expect(dialog.getAttribute("aria-modal") ?? dialog.closest("[aria-modal]")?.getAttribute("aria-modal")).toBe(
      "true",
    );
    const title = document.getElementById(dialog.getAttribute("aria-labelledby") ?? "");
    expect(title?.textContent).toBe("Move pages");
    expect(document.getElementById(dialog.getAttribute("aria-describedby") ?? "")?.textContent).toContain("4–6");
    await expect.poll(() => focused().tagName).toBe("INPUT");
  });

  it("traps Tab inside and makes the page behind inert", async () => {
    const dialog = await openDialog();
    for (let i = 0; i < 6; i++) {
      await userEvent.tab();
      expect(dialog.contains(focused())).toBe(true);
    }
    const outside = [...document.querySelectorAll("button")].find((b) => b.textContent === "Outside");
    expect(outside?.closest("[inert]")).not.toBeNull();
  });

  it("closes on Escape and returns focus to its opener", async () => {
    await openDialog();
    await userEvent.keyboard("{Escape}");
    expect(role("dialog")).toBeNull();
    // WebKit moves focus back a little after the dialog unmounts.
    await expect.poll(() => focused().textContent).toBe("Move pages");
  });

  it("closes on a scrim click, from its close button, but not when it asks a required question", async () => {
    await openDialog();
    await userEvent.click(document.querySelector(".paper-modal-overlay") as HTMLElement, { position: { x: 4, y: 4 } });
    expect(role("dialog")).toBeNull();
    cleanup();

    await openDialog();
    await userEvent.click(document.querySelector("[aria-label=Close]") as HTMLElement);
    expect(role("dialog")).toBeNull();
    cleanup();

    const dialog = await openDialog({ required: true, role: "alertdialog" });
    expect(dialog.querySelector("[aria-label=Close]")).toBeNull();
    await userEvent.keyboard("{Escape}");
    await userEvent.click(document.querySelector(".paper-modal-overlay") as HTMLElement, { position: { x: 4, y: 4 } });
    expect(role("alertdialog")).not.toBeNull();
  });

  it("sends focus to the fallback when its opener is gone", async () => {
    const fallback = document.createElement("button");
    fallback.textContent = "Fallback";
    expect(await openDialog({ returnFocusTo: () => fallback, vanish: true })).not.toBeNull();
    document.body.append(fallback);
    await expect
      .poll(() => [...document.querySelectorAll("button")].some((b) => b.textContent === "Move pages"))
      .toBe(false);
    await userEvent.keyboard("{Escape}");
    await expect.poll(focused).toBe(fallback);
  });

  it("starts on a radio group's checked radio", () => {
    renderIn(<Radios />);
    expect((focused() as HTMLInputElement).value).toBe("current");
  });

  it("sends focus to the fallback when it closes by unmounting and its opener is gone", async () => {
    const fallback = document.createElement("button");
    document.body.append(fallback);
    renderIn(<Conditional fallback={fallback} />);
    await userEvent.keyboard("{Escape}");
    await expect.poll(focused).toBe(fallback);
  });

  it("closes a menu inside it first, then itself", async () => {
    const dialog = await openDialog({
      children: (
        <MenuTrigger>
          <Button>Position</Button>
          <Menu
            aria-label="Position"
            platform="other"
            onAction={() => {}}
            items={[
              { id: "before", label: "Before page" },
              { id: "after", label: "After page" },
            ]}
          />
        </MenuTrigger>
      ),
    });
    const position = [...dialog.querySelectorAll("button")].find((b) => b.textContent === "Position") as HTMLElement;
    position.focus();
    await userEvent.keyboard("{Enter}");
    expect(role("menu")).not.toBeNull();
    const zOf = (el: Element | null) => Number(getComputedStyle(el as Element).zIndex);
    expect(zOf(role("menu")?.closest(".paper-popover") ?? null)).toBeGreaterThan(
      zOf(document.querySelector(".paper-modal-overlay")),
    );
    await userEvent.keyboard("{Escape}");
    expect(role("menu")).toBeNull();
    expect(role("dialog")).not.toBeNull();
    await expect.poll(focused).toBe(position);
    await userEvent.keyboard("{Escape}");
    expect(role("dialog")).toBeNull();
  });
});

const ITEMS = [
  { id: "copy", label: "Copy", icon: "copy", shortcut: { mac: "Mod+C", other: "Mod+C" } },
  { id: "rename", label: "Rename", icon: "rename" },
  { id: "export", label: "Export", isDisabled: true, disabledReason: "Nothing to export yet" },
  { id: "delete", label: "Delete", icon: "delete", isDestructive: true },
] as const;

function MoreMenu({ onAction }: { onAction: (id: string) => void }) {
  return (
    <MenuTrigger>
      <IconButton label="More" icon="more" />
      <Menu aria-label="More" items={ITEMS} onAction={onAction} platform="other" />
    </MenuTrigger>
  );
}

describe("Menu", () => {
  it("follows the menu button pattern: Enter opens on the first item, arrows, Home, End and letters move", async () => {
    const onAction = vi.fn();
    const host = renderIn(<MoreMenu onAction={onAction} />);
    const button = host.querySelector("button") as HTMLElement;
    expect(button.getAttribute("aria-haspopup")).toBe("true");
    expect(button.getAttribute("aria-expanded")).toBe("false");
    await userEvent.tab();
    await userEvent.keyboard("{Enter}");
    expect(button.getAttribute("aria-expanded")).toBe("true");
    expect(focused().textContent).toContain("Copy");
    await userEvent.keyboard("{ArrowDown}");
    expect(focused().textContent).toContain("Rename");
    await userEvent.keyboard("{ArrowDown}");
    expect(focused().textContent).toContain("Delete");
    await userEvent.keyboard("{Home}");
    expect(focused().textContent).toContain("Copy");
    await userEvent.keyboard("{End}");
    expect(focused().textContent).toContain("Delete");
    await userEvent.keyboard("r");
    expect(focused().textContent).toContain("Rename");
    await userEvent.keyboard("{Enter}");
    expect(onAction).toHaveBeenCalledWith("rename");
    expect(role("menu")).toBeNull();
    await expect.poll(focused).toBe(button);
  });

  it("shows shortcut chips and the disabled item's reason, and Escape returns focus to the button", async () => {
    const host = renderIn(<MoreMenu onAction={() => {}} />);
    await userEvent.tab();
    await userEvent.keyboard("{ArrowDown}");
    const menu = role("menu") as HTMLElement;
    expect(menu.querySelector("kbd")?.textContent).toBe("Ctrl C");
    const exportItem = [...menu.querySelectorAll("[role=menuitem]")].find((i) => i.textContent?.includes("Export"));
    expect(exportItem?.getAttribute("aria-disabled")).toBe("true");
    expect(exportItem?.textContent).toContain("Nothing to export yet");
    await userEvent.keyboard("{Escape}");
    expect(role("menu")).toBeNull();
    await expect.poll(focused).toBe(host.querySelector("button"));
  });
});

describe("Popover", () => {
  it("opens a named dialog from its button and closes on Escape", async () => {
    const host = renderIn(
      <PopoverTrigger>
        <Button>View</Button>
        <Popover aria-label="View settings">
          <Button>Fit width</Button>
        </Popover>
      </PopoverTrigger>,
    );
    await userEvent.tab();
    await userEvent.keyboard("{Enter}");
    expect(role("dialog")?.getAttribute("aria-label")).toBe("View settings");
    await userEvent.keyboard("{Escape}");
    expect(role("dialog")).toBeNull();
    await expect.poll(focused).toBe(host.querySelector("button"));
  });
});

describe("the layer stack", () => {
  it("closes the innermost layer on Escape even when focus is outside it", async () => {
    renderIn(
      <PopoverTrigger>
        <Button>View</Button>
        <Popover aria-label="View settings">
          <Button>Fit width</Button>
        </Popover>
      </PopoverTrigger>,
    );
    await userEvent.tab();
    await userEvent.keyboard("{Enter}");
    expect(role("dialog")).not.toBeNull();
    (document.activeElement as HTMLElement).blur();
    expect(document.activeElement).toBe(document.body);
    await userEvent.keyboard("{Escape}");
    expect(role("dialog")).toBeNull();
  });
});

describe("Tooltip", () => {
  it("shows at once on keyboard focus, with the shortcut chip, and Escape hides it", async () => {
    renderIn(
      <Tooltip text="Find in document" shortcut={{ mac: "Mod+F", other: "Mod+F" }} platform="mac">
        <IconButton label="Find in document" icon="find" />
      </Tooltip>,
    );
    await userEvent.tab();
    const tip = role("tooltip");
    expect(Number(getComputedStyle(tip as HTMLElement).zIndex)).toBe(50);
    expect(tip?.textContent).toBe("Find in document⌘F");
    expect(focused().getAttribute("aria-describedby")).toBe(tip?.id);
    await userEvent.keyboard("{Escape}");
    expect(role("tooltip")).toBeNull();
  });
});

describe("ToastRegion", () => {
  it("is a polite live region before any toast, and shows the newest three at the bottom", () => {
    const queue = new ToastQueue();
    const host = renderIn(<ToastRegion queue={queue} aria-label="Notifications" />);
    const live = host.querySelector("[role=status]") as HTMLElement;
    expect(live.childElementCount).toBe(0);
    for (const message of ["One", "Two", "Three", "Four"]) queue.add({ message, timeout: 60_000 });
    return new Promise<void>((resolve) => {
      requestAnimationFrame(() => {
        expect([...live.children].map((c) => c.textContent)).toEqual(["Two", "Three", "Four"]);
        resolve();
      });
    });
  });

  it("removes a toast after its timeout, unless the region is hovered", async () => {
    const queue = new ToastQueue();
    const host = renderIn(<ToastRegion queue={queue} aria-label="Notifications" />);
    queue.add({ message: "Copied", timeout: 150 });
    await frame();
    const toast = host.querySelector(".paper-toast") as HTMLElement;
    await userEvent.hover(toast);
    await new Promise((r) => setTimeout(r, 300));
    expect(host.querySelector(".paper-toast")).not.toBeNull();
    await userEvent.unhover(toast);
    await new Promise((r) => setTimeout(r, 300));
    expect(host.querySelector(".paper-toast")).toBeNull();
  });

  it("runs the action from the keyboard and dismisses the toast; a handle dismisses too", async () => {
    const queue = new ToastQueue();
    const onUndo = vi.fn();
    const host = renderIn(<ToastRegion queue={queue} aria-label="Notifications" />);
    queue.add({ message: "Highlight deleted", action: { label: "Undo", onPress: onUndo }, timeout: 60_000 });
    const handle = queue.add({ message: "Copied", timeout: 60_000 });
    await frame();
    await userEvent.tab();
    expect(focused().textContent).toBe("Undo");
    await userEvent.keyboard("{Enter}");
    expect(onUndo).toHaveBeenCalledOnce();
    handle.dismiss();
    await frame();
    expect(host.querySelector(".paper-toast")).toBeNull();
  });

  it("keeps an endless toast, and resumes the timers once a pressed action takes focus away", async () => {
    const queue = new ToastQueue();
    const host = renderIn(
      <>
        <Button>Before</Button>
        <ToastRegion queue={queue} aria-label="Notifications" />
      </>,
    );
    queue.add({ message: "Update ready", timeout: Number.POSITIVE_INFINITY });
    queue.add({ message: "Highlight deleted", action: { label: "Undo", onPress: () => {} }, timeout: 60_000 });
    await frame();
    await userEvent.tab();
    await userEvent.tab();
    expect(focused().textContent).toBe("Undo");
    await userEvent.keyboard("{Enter}");
    await expect.poll(() => focused().textContent).toBe("Before");
    queue.add({ message: "Copied", timeout: 100 });
    await new Promise((r) => setTimeout(r, 300));
    expect([...host.querySelectorAll(".paper-toast")].map((t) => t.textContent)).toEqual(["Update ready"]);
  });

  it("sits above dialogs and menus, and stays reachable while a dialog is open", async () => {
    const queue = new ToastQueue();
    renderIn(
      <>
        <Opener />
        <ToastRegion queue={queue} aria-label="Notifications" />
      </>,
    );
    await userEvent.tab();
    await userEvent.keyboard("{Enter}");
    queue.add({ message: "Saved", timeout: 60_000 });
    await frame();
    const region = document.querySelector(".paper-toast-region") as HTMLElement;
    expect(region.closest("[inert]")).toBeNull();
    const z = (el: Element | null) => Number(getComputedStyle(el as Element).zIndex);
    expect(z(region)).toBeGreaterThan(z(document.querySelector(".paper-modal-overlay")));
    // A menu in the dialog sits between the two.
    cleanup();
    renderIn(
      <>
        <MoreMenu onAction={() => {}} />
        <ToastRegion queue={queue} aria-label="Notifications" />
      </>,
    );
    await userEvent.tab();
    await userEvent.keyboard("{Enter}");
    const menu = role("menu")?.closest(".paper-popover") ?? null;
    expect(z(menu)).toBe(50);
    expect(z(document.querySelector(".paper-toast-region"))).toBeGreaterThan(z(menu));
  });
});

describe.each(["light", "dark", "high-contrast"] as const)("axe with each overlay open, %s theme", (theme) => {
  const check = async (rules: axe.RuleObject = {}) => {
    const result = await axe.run(document.body, {
      runOnly: { type: "tag", values: ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"] },
      rules,
    });
    expect(result.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`)).toEqual([]);
  };

  it("dialog", async () => {
    document.documentElement.dataset.theme = theme;
    await openDialog();
    await check();
  });

  it("menu", async () => {
    document.documentElement.dataset.theme = theme;
    renderIn(<MoreMenu onAction={() => {}} />, { theme });
    await userEvent.tab();
    await userEvent.keyboard("{Enter}");
    // axe's experimental label-content-name-mismatch wants an item's name to hold every visible
    // character; a menu item's shortcut chip and disabled reason are its description instead,
    // which this checks directly.
    for (const item of document.querySelectorAll("[role=menuitem]")) {
      const described = (item.getAttribute("aria-describedby") ?? "")
        .split(" ")
        .map((id) => document.getElementById(id));
      for (const extra of item.querySelectorAll("kbd, .paper-menu-reason")) expect(described).toContain(extra);
    }
    await check({ "label-content-name-mismatch": { enabled: false } });
  });

  it("tooltip and toasts", async () => {
    document.documentElement.dataset.theme = theme;
    const queue = new ToastQueue();
    renderIn(
      <>
        <Tooltip text="Find in document">
          <IconButton label="Find in document" icon="find" />
        </Tooltip>
        <ToastRegion queue={queue} aria-label="Notifications" />
      </>,
      { theme },
    );
    queue.add({ message: "Copied", action: { label: "Undo", onPress: () => {} }, timeout: 60_000 });
    await userEvent.tab();
    await check();
  });
});
