// Renders an element as an app would: the stylesheet loaded, inside a theme, a direction and an
// optional density.

import "../../src/styles/index.css";
import type { ReactElement } from "react";
import { flushSync } from "react-dom";
import { createRoot, type Root } from "react-dom/client";
import { type Density, DensityProvider } from "../../src/index.ts";

/** Where to render. */
export interface Setting {
  theme?: "light" | "dark" | "high-contrast";
  dir?: "ltr" | "rtl";
  density?: Density;
}

/** Renders `element` into a fresh host and returns the host. */
export function renderIn(element: ReactElement, { theme = "light", dir = "ltr", density }: Setting = {}): HTMLElement {
  const host = document.createElement("div");
  host.dataset.theme = theme;
  host.dir = dir;
  host.style.padding = "16px";
  host.style.background = "var(--paper-surface)";
  host.style.color = "var(--paper-text)";
  host.style.font = "var(--paper-type-body)";
  document.body.append(host);
  const root = createRoot(host);
  roots.push(root);
  flushSync(() =>
    root.render(density === undefined ? element : <DensityProvider density={density}>{element}</DensityProvider>),
  );
  return host;
}

const roots: Root[] = [];

/** Unmounts every rendered element, so overlays undo what they did to the page, then removes the hosts. */
export function cleanup(): void {
  for (const root of roots.splice(0)) flushSync(() => root.unmount());
  document.body.replaceChildren();
}

/** The host's interactive elements: what a pointer or keyboard can reach. */
export function targets(host: HTMLElement): HTMLElement[] {
  return [
    ...host.querySelectorAll<HTMLElement>(
      "button, a[href], [role=link], [role=tab], select, input:not([type=checkbox]):not([type=radio]), textarea, label:has(input)",
    ),
  ];
}

/** The first element under `host` that matches `selector`. */
export const get = (host: HTMLElement, selector: string) => host.querySelector(selector) as HTMLElement;

/** Every element under `host` that matches `selector`. */
export const all = (host: HTMLElement, selector: string) => [...host.querySelectorAll<HTMLElement>(selector)];

/** The element with keyboard focus. */
export const focused = () => document.activeElement as HTMLElement;
