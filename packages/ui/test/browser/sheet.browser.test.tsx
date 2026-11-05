import axe from "axe-core";
import { useState } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import {
  BottomSheet,
  type BottomSheetProps,
  Button,
  Dialog,
  Menu,
  MenuTrigger,
  TextInput,
  ToastQueue,
  ToastRegion,
} from "../../src/index.ts";
import { cleanup, focused, renderIn } from "./render.tsx";

afterEach(() => {
  cleanup();
  document.documentElement.removeAttribute("data-theme");
});

const sheet = () => document.querySelector(".paper-sheet") as HTMLElement | null;
const grabber = () => document.querySelector(".paper-sheet-grabber") as HTMLElement;
const header = () => document.querySelector(".paper-sheet-title") as HTMLElement;
const frame = () => new Promise((resolve) => requestAnimationFrame(resolve));
const pause = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

/** A closed modal sheet and the button that opens it. */
function MovePages() {
  const [open, setOpen] = useState(false);
  return (
    <>
      <Button onPress={() => setOpen(true)}>Move pages</Button>
      <BottomSheet
        isOpen={open}
        onOpenChange={setOpen}
        title="Move pages"
        description="Pages 4–6 move after the page you choose."
        closeLabel="Close"
      >
        <TextInput label="After page" />
      </BottomSheet>
    </>
  );
}

/** A menu button for the sheet's content. */
function SortMenu() {
  return (
    <MenuTrigger>
      <Button>Sort</Button>
      <Menu
        aria-label="Sort"
        platform="other"
        onAction={() => {}}
        items={[
          { id: "number", label: "By number" },
          { id: "label", label: "By label" },
        ]}
      />
    </MenuTrigger>
  );
}

/** A non-modal sheet the app moves to the large detent on F2, while focus stays in the page. */
function Growing() {
  const [detent, setDetent] = useState<"medium" | "large">("medium");
  return (
    <div>
      <div style={{ blockSize: `${Math.round(window.innerHeight * 0.4)}px` }} />
      <input aria-label="Find" onKeyDown={(e) => e.key === "F2" && setDetent("large")} />
      <div style={{ blockSize: "2000px" }} />
      <Pages isModal={false} detent={detent} />
    </div>
  );
}

// Compile-time check: a non-modal sheet is a region, so it takes no dialog role.
export function nonModalRole() {
  // @ts-expect-error role needs a modal sheet
  return <BottomSheet isOpen onOpenChange={() => {}} title="Pages" isModal={false} role="alertdialog" />;
}

/** A dialog with a field, and the button that opens it. */
function MovePagesDialog() {
  const [open, setOpen] = useState(false);
  return (
    <>
      <Button onPress={() => setOpen(true)}>Move pages</Button>
      <Dialog isOpen={open} onOpenChange={setOpen} title="Move pages" closeLabel="Close">
        <TextInput label="After page" />
      </Dialog>
    </>
  );
}

type PagesProps = Partial<Omit<BottomSheetProps, "isModal" | "role">> & { isModal?: boolean; onClosed?: () => void };

/** The Pages sheet, open, with a button that opens it again. */
function Pages({ isModal, ...props }: PagesProps) {
  const [open, setOpen] = useState(true);
  const mode = isModal === false ? { isModal: false as const } : {};
  return (
    <>
      <Button onPress={() => setOpen(true)}>Pages</Button>
      <BottomSheet
        title="Pages"
        count="24 pages"
        closeLabel="Close pages"
        resizeLabel="Resize sheet"
        {...props}
        {...mode}
        isOpen={open}
        onOpenChange={(next) => {
          setOpen(next);
          if (!next) props.onClosed?.();
        }}
      >
        {props.children ?? <Button>Page 1</Button>}
      </BottomSheet>
    </>
  );
}

// A flick: two moves 5 ms apart. Firefox and WebKit coarsen event.timeStamp, so moves dispatched
// in the same millisecond carry no speed; 40 px per move stays past a flick's 0.5 px/ms even when a
// busy runner stretches the wait to 60 ms.
const FLICK = { steps: 2, gap: 5 };

/** Drags from the grabber (or `from`) by `dy` CSS px in `steps` moves `gap` ms apart, held `hold` ms before the release. */
async function drag(dy: number, { steps = 6, gap = 40, hold = 0, from = grabber() } = {}) {
  const target = from;
  const x = target.getBoundingClientRect().left + 10;
  const y = target.getBoundingClientRect().top + 10;
  const fire = (type: string, clientY: number) =>
    target.dispatchEvent(new PointerEvent(type, { bubbles: true, pointerId: 7, button: 0, clientX: x, clientY }));
  fire("pointerdown", y);
  for (let i = 1; i <= steps; i++) {
    if (gap > 0) await pause(gap);
    fire("pointermove", y + (dy * i) / steps);
  }
  if (hold > 0) await pause(hold);
  fire("pointerup", y + dy);
  await frame();
}

describe("BottomSheet", () => {
  it("changes detent from its grabber on click, Enter and Space, without any drag (WCAG 2.5.7)", async () => {
    renderIn(<Pages />);
    const grabber = document.querySelector("[aria-label='Resize sheet']") as HTMLElement;
    const box = grabber.getBoundingClientRect();
    expect([box.width, box.height]).toEqual([64, 44]);
    // The whole box takes the press, though the header tucks up into its lower half.
    expect(grabber.contains(document.elementFromPoint(box.left + box.width / 2, box.bottom - 2))).toBe(true);
    expect(sheet()?.dataset.detent).toBe("medium");
    await userEvent.click(grabber);
    expect(sheet()?.dataset.detent).toBe("large");
    grabber.focus();
    await userEvent.keyboard("{Enter}");
    expect(sheet()?.dataset.detent).toBe("medium");
    await userEvent.keyboard(" ");
    expect(sheet()?.dataset.detent).toBe("large");
    expect(sheet()?.getBoundingClientRect().top).toBeCloseTo(56, 0);
  });

  it("opens at its default detent again after it closes", async () => {
    const host = renderIn(<Pages />);
    await userEvent.click(document.querySelector("[aria-label='Resize sheet']") as HTMLElement);
    expect(sheet()?.dataset.detent).toBe("large");
    await userEvent.click(document.querySelector("[aria-label='Close pages']") as HTMLElement);
    expect(sheet()).toBeNull();
    await userEvent.click(host.querySelector("button") as HTMLElement);
    expect(sheet()?.dataset.detent).toBe("medium");
  });

  it("snaps a slow drag of the grabber or header to the nearest detent, and a small one back", async () => {
    renderIn(<Pages />);
    await drag(-20);
    expect(sheet()?.dataset.detent).toBe("medium");
    await drag(-(window.innerHeight * 0.48 - 56) + 10);
    expect(sheet()?.dataset.detent).toBe("large");
    expect(sheet()?.style.getPropertyValue("--sheet-drag")).toBe("0px");
    await drag(window.innerHeight * 0.48 - 56 - 10, { from: header() });
    expect(sheet()?.dataset.detent).toBe("medium");
  });

  it("goes one detent with a flick: down from large to medium, down from medium closed", async () => {
    const onClosed = vi.fn();
    renderIn(<Pages defaultDetent="large" onClosed={onClosed} />);
    // A fast drag that stops before the release is no flick: it snaps back to the nearest detent.
    await drag(80, { ...FLICK, hold: 150 });
    expect(sheet()?.dataset.detent).toBe("large");
    await drag(80, FLICK);
    expect(sheet()?.dataset.detent).toBe("medium");
    await drag(80, FLICK);
    expect(sheet()).toBeNull();
    expect(onClosed).toHaveBeenCalledOnce();
  });

  it("keeps a drag going when a second finger touches and lifts", async () => {
    renderIn(<Pages />);
    const target = grabber();
    const { left, top } = target.getBoundingClientRect();
    const fire = (type: string, pointerId: number, clientY: number) =>
      target.dispatchEvent(
        new PointerEvent(type, { bubbles: true, pointerId, button: 0, clientX: left + 10, clientY }),
      );
    const y = top + 10;
    const far = -(window.innerHeight * 0.48 - 56) + 10;
    fire("pointerdown", 7, y);
    fire("pointermove", 7, y - 20);
    fire("pointerdown", 8, y + 100);
    fire("pointerup", 8, y + 100);
    await pause(40);
    fire("pointermove", 7, y + far);
    await pause(150);
    fire("pointerup", 7, y + far);
    await frame();
    expect(sheet()?.dataset.detent).toBe("large");
    expect(sheet()?.style.getPropertyValue("--sheet-drag")).toBe("0px");
    expect(sheet()?.hasAttribute("data-dragging")).toBe(false);
  });

  it("takes the grabber's key press after a drag of the header", async () => {
    renderIn(<Pages />);
    await drag(-(window.innerHeight * 0.48 - 56) + 10, { from: header() });
    expect(sheet()?.dataset.detent).toBe("large");
    await pause(0);
    grabber().focus();
    await userEvent.keyboard("{Enter}");
    // The press commits on a busy runner a little after the key event returns.
    await expect.poll(() => sheet()?.dataset.detent).toBe("medium");
  });

  it("is a modal dialog: labelled, described, the page inert, Escape closes and focus goes back", async () => {
    const host = renderIn(<MovePages />);
    await userEvent.tab();
    await userEvent.keyboard("{Enter}");
    const dialog = document.querySelector("[role=dialog]") as HTMLElement;
    expect(dialog.getAttribute("aria-modal")).toBe("true");
    expect(document.getElementById(dialog.getAttribute("aria-labelledby") ?? "")?.textContent).toBe("Move pages");
    expect(document.getElementById(dialog.getAttribute("aria-describedby") ?? "")?.textContent).toContain("4–6");
    expect(
      host.closest("[inert]") ?? host.querySelector("[inert]") ?? host.parentElement?.closest("[inert]"),
    ).not.toBeNull();
    await userEvent.keyboard("{Escape}");
    expect(sheet()).toBeNull();
    await expect.poll(focused).toBe(host.querySelector("button"));
  });

  it("leaves a required question open: no close button, and Escape and a flick down do nothing", async () => {
    renderIn(<Pages isRequired />);
    expect(document.querySelector("[aria-label='Close pages']")).toBeNull();
    await userEvent.keyboard("{Escape}");
    await drag(120, FLICK);
    expect(sheet()).not.toBeNull();
  });

  it("non-modal: the page stays usable, Escape still closes it, and it never covers the focused element", async () => {
    const onClosed = vi.fn();
    const host = renderIn(
      <div>
        <div style={{ blockSize: `${Math.round(window.innerHeight * 0.75)}px` }} />
        <input aria-label="Find" />
        <div style={{ blockSize: "2000px" }} />
        <Pages isModal={false} onClosed={onClosed} />
      </div>,
    );
    expect(document.querySelector("[aria-modal]")).toBeNull();
    expect(host.closest("[inert]")).toBeNull();
    expect(getComputedStyle(sheet() as HTMLElement).zIndex).toBe("30");
    const input = host.querySelector("input") as HTMLInputElement;
    input.focus();
    await frame();
    await frame();
    expect(input.getBoundingClientRect().bottom).toBeLessThanOrEqual(
      (sheet() as HTMLElement).getBoundingClientRect().top,
    );
    await userEvent.keyboard("{Escape}");
    expect(onClosed).toHaveBeenCalledOnce();
  });

  it("non-modal: clears the focused element when the sheet grows, keeping a tall field's top in view", async () => {
    const host = renderIn(<Growing />);
    const input = host.querySelector("input") as HTMLInputElement;
    input.focus();
    await frame();
    await frame();
    expect(input.getBoundingClientRect().bottom).toBeLessThanOrEqual(
      (sheet() as HTMLElement).getBoundingClientRect().top,
    );
    await userEvent.keyboard("{F2}");
    await frame();
    await frame();
    expect(sheet()?.dataset.detent).toBe("large");
    expect(input.getBoundingClientRect().top).toBeGreaterThanOrEqual(0);
    expect(input.getBoundingClientRect().bottom).toBeLessThanOrEqual(
      (sheet() as HTMLElement).getBoundingClientRect().top,
    );
    cleanup();
    window.scrollTo(0, 0);
    const tall = renderIn(
      <div>
        <div style={{ blockSize: `${Math.round(window.innerHeight * 0.5)}px` }} />
        <textarea aria-label="Note" style={{ blockSize: "200px" }} />
        <div style={{ blockSize: "2000px" }} />
        <Pages isModal={false} defaultDetent="large" />
      </div>,
    );
    (tall.querySelector("textarea") as HTMLTextAreaElement).focus();
    await frame();
    await frame();
    expect((tall.querySelector("textarea") as HTMLElement).getBoundingClientRect().top).toBeGreaterThanOrEqual(0);
  });

  it("non-modal: leaves the page still while focus moves through a menu opened from the sheet", async () => {
    renderIn(
      <div>
        <div style={{ blockSize: "3000px" }} />
        <Pages isModal={false}>
          <SortMenu />
        </Pages>
      </div>,
    );
    const sort = [...document.querySelectorAll("button")].find((b) => b.textContent === "Sort") as HTMLElement;
    sort.focus();
    const before = window.scrollY;
    await userEvent.keyboard("{Enter}");
    await userEvent.keyboard("{ArrowDown}");
    await frame();
    await frame();
    expect(document.querySelector("[role=menu]")).not.toBeNull();
    expect(window.scrollY).toBe(before);
  });

  it("rises with an on-screen keyboard, its top no higher than the large detent", async () => {
    renderIn(<Pages />);
    const panel = sheet() as HTMLElement;
    panel.style.setProperty("--sheet-keyboard", "200px");
    expect(panel.getBoundingClientRect().top).toBeCloseTo(window.innerHeight * 0.48 - 200, 0);
    expect(panel.getBoundingClientRect().bottom).toBeCloseTo(window.innerHeight - 200, 0);
    panel.style.setProperty("--sheet-keyboard", `${window.innerHeight * 0.48}px`);
    expect(panel.getBoundingClientRect().top).toBeCloseTo(56, 0);
  });

  it("stacks under a menu opened in it and a toast, and Escape closes the menu first", async () => {
    const queue = new ToastQueue();
    renderIn(
      <>
        <Pages>
          <SortMenu />
        </Pages>
        <ToastRegion queue={queue} aria-label="Notifications" />
      </>,
    );
    queue.add({ message: "Copied", timeout: 60_000 });
    const sort = [...document.querySelectorAll("button")].find((b) => b.textContent === "Sort") as HTMLElement;
    sort.focus();
    await userEvent.keyboard("{Enter}");
    const z = (el: Element | null) => Number(getComputedStyle(el as Element).zIndex);
    const menu = document.querySelector("[role=menu]")?.closest(".paper-popover") ?? null;
    // A modal sheet is a dialog, so it sits in the modal layer.
    expect(z(document.querySelector(".paper-sheet-overlay"))).toBe(40);
    expect(z(menu)).toBe(50);
    expect(z(document.querySelector(".paper-toast-region"))).toBe(60);
    await userEvent.keyboard("{Escape}");
    expect(document.querySelector("[role=menu]")).toBeNull();
    expect(sheet()).not.toBeNull();
    await expect.poll(focused).toBe(sort);
    await userEvent.keyboard("{Escape}");
    expect(sheet()).toBeNull();
  });
});

describe("Dialog on a phone", () => {
  it("is a bottom sheet at the large detent with its actions full width below the content", async () => {
    const [width, height] = [window.innerWidth, window.innerHeight];
    await page.viewport(390, 844);
    try {
      renderIn(
        <Dialog
          isOpen
          onOpenChange={() => {}}
          title="Move pages"
          closeLabel="Close"
          actions={
            <>
              <Button>Cancel</Button>
              <Button variant="primary">Move</Button>
            </>
          }
        >
          <TextInput label="After page" />
        </Dialog>,
      );
      await frame();
      expect(sheet()?.dataset.detent).toBe("large");
      expect(document.querySelector("[role=dialog]")?.getAttribute("aria-modal")).toBe("true");
      const footer = document.querySelector(".paper-sheet-footer") as HTMLElement;
      for (const button of footer.querySelectorAll("button")) {
        expect(button.getBoundingClientRect().width).toBeCloseTo(footer.getBoundingClientRect().width - 32, 0);
        expect(button.getBoundingClientRect().height).toBeGreaterThanOrEqual(44);
      }
      await expect.poll(() => focused().tagName).toBe("INPUT");
    } finally {
      // Back to the size the other tests run at, which is wider than a phone.
      await page.viewport(width, height);
    }
  });

  it("keeps the layout it opened with until it closes, so turning the phone loses nothing", async () => {
    const [width, height] = [window.innerWidth, window.innerHeight];
    await page.viewport(390, 844);
    try {
      const host = renderIn(<MovePagesDialog />);
      await userEvent.click(host.querySelector("button") as HTMLElement);
      await userEvent.keyboard("12");
      await page.viewport(width, height);
      await frame();
      expect(sheet()).not.toBeNull();
      expect((document.querySelector("[role=dialog] input") as HTMLInputElement).value).toBe("12");
      await userEvent.keyboard("{Escape}");
      await userEvent.click(host.querySelector("button") as HTMLElement);
      expect(sheet()).toBeNull();
      expect(document.querySelector(".paper-dialog")).not.toBeNull();
    } finally {
      await page.viewport(width, height);
    }
  });
});

describe.each(["light", "dark", "high-contrast"] as const)("axe with a sheet open, %s theme", (theme) => {
  const check = async () => {
    const result = await axe.run(document.body, {
      runOnly: { type: "tag", values: ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"] },
    });
    expect(result.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`)).toEqual([]);
  };

  it("modal", async () => {
    document.documentElement.dataset.theme = theme;
    renderIn(<Pages />, { theme });
    await check();
  });

  it("non-modal", async () => {
    document.documentElement.dataset.theme = theme;
    renderIn(<Pages isModal={false} />, { theme });
    await check();
  });
});
