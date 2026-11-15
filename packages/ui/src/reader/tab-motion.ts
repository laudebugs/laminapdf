// The document tabs' motion (motion plan §5): when the strip's tabs change, each tab that stays
// slides from where it was to where it is, so a reorder and the gap a closed tab leaves close with
// motion, and a new tab fades in growing from the motion scale. The slide is a FLIP: the tabs are
// laid out at their new places first, then each is drawn back at its old place and let go, through
// the Web Animations API, with the tokens' durations and easings read from the strip's styles.
// Under reduced motion `--paper-motion-travel` is 0 and `--paper-motion-scale` 1, so nothing
// slides or grows and the new tab only fades (design language §7.0).

import { type RefObject, useLayoutEffect, useRef } from "react";

/** The attribute each tab's box carries with its id, which the hook measures by. */
const TAB_BOX = "data-tab-box";

/** A token's value on `element`, in milliseconds (`120ms`, `0.2s`), or `fallback`. */
function duration(style: CSSStyleDeclaration, name: string, fallback: number): number {
  const value = style.getPropertyValue(name).trim();
  const number = Number.parseFloat(value);
  if (Number.isNaN(number)) return fallback;
  return value.endsWith("ms") ? number : number * 1000;
}

/** A unitless token's value on `element`, or `fallback`. */
function factor(style: CSSStyleDeclaration, name: string, fallback: number): number {
  const number = Number.parseFloat(style.getPropertyValue(name));
  return Number.isNaN(number) ? fallback : number;
}

/** What a tab moves by: the tokens' travel, scale, durations and easings, read once per change. */
interface TabMotion {
  readonly travel: number;
  readonly scale: number;
  readonly enter: KeyframeAnimationOptions;
  readonly slide: KeyframeAnimationOptions;
}

function motionOf(style: CSSStyleDeclaration): TabMotion {
  return {
    travel: factor(style, "--paper-motion-travel", 1),
    scale: factor(style, "--paper-motion-scale", 1),
    enter: {
      duration: duration(style, "--paper-duration-enter-small", 120),
      easing: style.getPropertyValue("--paper-ease-enter").trim() || "ease-out",
    },
    slide: {
      duration: duration(style, "--paper-duration-enter-panel", 200),
      easing: style.getPropertyValue("--paper-ease-standard").trim() || "ease",
    },
  };
}

/** Animates one tab's box: in, when it `was` nowhere; across, when it moved from `was` to `at`. */
function animateTab(box: HTMLElement, was: number | undefined, at: number, motion: TabMotion): void {
  if (was === undefined) {
    box.animate(
      [
        { opacity: 0, transform: `scale(${motion.scale})` },
        { opacity: 1, transform: "none" },
      ],
      motion.enter,
    );
  } else if (was !== at && motion.travel > 0) {
    box.animate([{ transform: `translateX(${(was - at) * motion.travel}px)` }, { transform: "none" }], motion.slide);
  }
}

/** Where each tab's box in `list` is, by its id, from the strip's start edge. */
function measure(list: HTMLElement): Map<string, number> {
  const boxes = list.querySelectorAll<HTMLElement>(`[${TAB_BOX}]`);
  return new Map([...boxes].map((box) => [box.getAttribute(TAB_BOX) ?? "", box.offsetLeft]));
}

/**
 * Animates the tabs inside `list` whenever `ids` (the tabs in order) change. The first layout
 * draws still: tabs that were there when the strip appeared don't move in. When the strip changes
 * size (the window resized, so the tabs shrank or grew), the places are measured again without
 * moving anything, so the next change slides tabs from where they really are.
 */
export function useTabMotion(list: RefObject<HTMLElement | null>, ids: readonly string[]): void {
  const places = useRef<Map<string, number> | undefined>(undefined);
  const key = ids.join("\n");
  // biome-ignore lint/correctness/useExhaustiveDependencies: `key` is what changes the layout to animate
  useLayoutEffect(() => {
    const element = list.current;
    if (element === null) return;
    const before = places.current;
    const now = measure(element);
    places.current = now;
    const boxes = [...element.querySelectorAll<HTMLElement>(`[${TAB_BOX}]`)];
    // The first layout, and a webview without the Web Animations API, draw still.
    if (before === undefined || boxes[0]?.animate === undefined) return;
    const motion = motionOf(getComputedStyle(element));
    for (const box of boxes) {
      const id = box.getAttribute(TAB_BOX) ?? "";
      animateTab(box, before.get(id), now.get(id) ?? 0, motion);
    }
  }, [key, list]);
  useLayoutEffect(() => {
    const element = list.current;
    if (element === null || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(() => {
      if (places.current !== undefined) places.current = measure(element);
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, [list]);
}
