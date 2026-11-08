import "@laminapdf/tokens/tokens.css";
import axe from "axe-core";
import { createElement, type ReactElement } from "react";
import { flushSync } from "react-dom";
import { createRoot } from "react-dom/client";
import { describe, expect, it } from "vitest";
import { GALLERY_CSS, galleryHtml } from "../../gallery/gallery.ts";
import { ICONS } from "../../src/index.ts";
import { Icon } from "../../src/react/index.ts";

/** Renders `element` into a fresh host and returns its `<svg>`. */
function render(element: ReactElement, color = ""): SVGSVGElement {
  const host = document.createElement("div");
  host.style.color = color;
  document.body.append(host);
  flushSync(() => createRoot(host).render(element));
  const svg = host.querySelector("svg");
  if (svg === null) throw new Error("no <svg> rendered");
  return svg;
}

describe("<Icon>", () => {
  it("is decorative by default: hidden from assistive technology, not focusable", () => {
    const svg = render(createElement(Icon, { name: "find" }));
    expect(svg.getAttribute("aria-hidden")).toBe("true");
    expect(svg.getAttribute("focusable")).toBe("false");
    expect(svg.hasAttribute("role")).toBe(false);
    expect(svg.innerHTML).toBe(ICONS.find);
  });

  it("is an image with an accessible name when given a label", () => {
    const svg = render(createElement(Icon, { name: "warning", label: "Repaired file" }));
    expect(svg.getAttribute("role")).toBe("img");
    expect(svg.getAttribute("aria-label")).toBe("Repaired file");
    expect(svg.hasAttribute("aria-hidden")).toBe(false);
  });

  it("sets only its size, and takes its colour from the text around it", () => {
    const svg = render(createElement(Icon, { name: "close", size: 22 }), "rgb(31, 92, 99)");
    expect([svg.getAttribute("width"), svg.getAttribute("height")]).toEqual(["22", "22"]);
    expect(svg.getAttribute("viewBox")).toBe("0 0 24 24");
    expect(getComputedStyle(svg).stroke).toBe("rgb(31, 92, 99)");
  });
});

describe("the gallery", () => {
  const host = document.createElement("div");
  const style = document.createElement("style");
  style.textContent = GALLERY_CSS;
  document.head.append(style);
  host.innerHTML = galleryHtml();
  document.body.append(host);

  it("shows every icon", () => {
    const shown = new Set([...host.querySelectorAll("[data-icon]")].map((row) => row.getAttribute("data-icon")));
    expect([...shown].sort()).toEqual(Object.keys(ICONS).sort());
  });

  it("has no accessibility violation (axe, WCAG 2.2 AA)", async () => {
    const result = await axe.run(host, {
      runOnly: { type: "tag", values: ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"] },
    });
    // Each violation names its elements and axe's reason, so a failure on another system says
    // what to fix without a rerun.
    const found = result.violations.flatMap((v) =>
      v.nodes.map((node) => `${v.id}: ${node.target.join(" ")}: ${node.failureSummary ?? ""}`),
    );
    expect(found).toEqual([]);
  });
});
