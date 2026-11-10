// LinkPreview (§7.27, R07, todo T5a): a tooltip under its link for the mouse and keyboard, which the
// link names with aria-describedby, holds while hovered and goes on Escape with focus left on the
// link; a bottom sheet with Go and Close for touch. axe passes with it open, in every theme and
// direction.
import axe from "axe-core";
import { type ReactNode, useEffect, useState } from "react";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { userEvent } from "vitest/browser";
import { LinkPreview, type LinkPreviewImage, type LinkPreviewProps } from "../../src/index.ts";
import { cleanup, focused, renderIn, type Setting } from "./render.tsx";

afterEach(cleanup);

let picture: LinkPreviewImage;
beforeAll(async () => {
  const pixels = new ImageData(80, 40);
  pixels.data.fill(200);
  picture = { bitmap: await createImageBitmap(pixels), width: 80, height: 40 };
});

const frame = () => new Promise((resolve) => requestAnimationFrame(resolve));

type Handlers = Pick<LinkPreviewProps, "onFollow" | "onDismiss" | "onHold">;

/** A link, and its preview once the link is in the page. */
function Harness({ pointer, handlers }: { pointer: LinkPreviewProps["pointer"]; handlers: Handlers }): ReactNode {
  const [anchor, setAnchor] = useState<HTMLElement | null>(null);
  const [open, setOpen] = useState(true);
  useEffect(() => {
    if (!open) anchor?.focus();
  }, [open, anchor]);
  return (
    <div style={{ padding: 40 }}>
      <p>
        As shown in{" "}
        <a ref={setAnchor} href="#cite.12" className="cite" style={{ color: "var(--paper-text)" }}>
          [12]
        </a>
        , previews help.
      </p>
      {anchor === null || !open ? null : (
        <LinkPreview
          anchor={anchor}
          pointer={pointer}
          title="References"
          caption="Link to page 7 · Enter to go"
          picture={picture}
          pictureLabel="[12] D. E. Knuth. The TeXbook. Addison-Wesley, 1984."
          text="[12] D. E. Knuth. The TeXbook. Addison-Wesley, 1984."
          goLabel="Go to page 7"
          closeLabel="Close"
          {...handlers}
          onDismiss={() => {
            handlers.onDismiss();
            setOpen(false);
          }}
        />
      )}
    </div>
  );
}

function handlers() {
  return { onFollow: vi.fn<() => void>(), onDismiss: vi.fn<() => void>(), onHold: vi.fn<(held: boolean) => void>() };
}

const tooltip = () => document.querySelector<HTMLElement>("[role=tooltip]");
const link = () => document.querySelector<HTMLElement>("a.cite");

describe("the popover (mouse and keyboard)", () => {
  it("is a tooltip the link names with aria-describedby", async () => {
    renderIn(<Harness pointer="keyboard" handlers={handlers()} />);
    await frame();
    const tip = tooltip();
    expect([tip !== null, link()?.getAttribute("aria-describedby") === tip?.id]).toEqual([true, true]);
  });

  it("names its picture with the destination's text", async () => {
    renderIn(<Harness pointer="fine" handlers={handlers()} />);
    await frame();
    expect(tooltip()?.querySelector("[role=img]")?.getAttribute("aria-label")).toBe(
      "[12] D. E. Knuth. The TeXbook. Addison-Wesley, 1984.",
    );
  });

  it("goes under its link", async () => {
    renderIn(<Harness pointer="fine" handlers={handlers()} />);
    await frame();
    await frame();
    const below = (tooltip()?.getBoundingClientRect().top ?? 0) >= (link()?.getBoundingClientRect().bottom ?? 1);
    expect(below).toBe(true);
  });

  it("goes on Escape with focus left on the link, and the link no longer names it", async () => {
    const h = handlers();
    renderIn(<Harness pointer="keyboard" handlers={h} />);
    await frame();
    link()?.focus();
    await userEvent.keyboard("{Escape}");
    await frame();
    expect([
      h.onDismiss.mock.calls.length,
      tooltip(),
      focused() === link(),
      link()?.hasAttribute("aria-describedby"),
    ]).toEqual([1, null, true, false]);
  });

  it("holds while the pointer is on it", async () => {
    const h = handlers();
    renderIn(<Harness pointer="fine" handlers={h} />);
    await frame();
    await frame();
    const tip = tooltip() as HTMLElement;
    await userEvent.hover(tip);
    await userEvent.unhover(tip);
    expect(h.onHold.mock.calls.map(([held]) => held)).toEqual([true, false]);
  });
});

describe("the sheet (touch)", () => {
  it("is a sheet with the title, the text and Go to page 7", async () => {
    const h = handlers();
    renderIn(<Harness pointer="touch" handlers={h} />);
    await frame();
    const sheet = document.querySelector<HTMLElement>("[role=dialog]");
    const go = [...(sheet?.querySelectorAll("button") ?? [])].find((b) => b.textContent === "Go to page 7");
    go?.click();
    expect([
      tooltip(),
      sheet?.textContent?.includes("References"),
      sheet?.textContent?.includes("TeXbook"),
      h.onFollow.mock.calls.length,
    ]).toEqual([null, true, true, 1]);
  });
});

const SETTINGS: Setting[] = (["light", "dark", "high-contrast"] as const).flatMap((theme) =>
  (["ltr", "rtl"] as const).map((dir) => ({ theme, dir })),
);

describe.each(SETTINGS)("axe with the popover open, $theme theme, $dir", (setting) => {
  it("finds no violation", async () => {
    renderIn(<Harness pointer="keyboard" handlers={handlers()} />, setting);
    await frame();
    const result = await axe.run(document.body, {
      runOnly: { type: "tag", values: ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"] },
    });
    expect(result.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`)).toEqual([]);
  });
});
