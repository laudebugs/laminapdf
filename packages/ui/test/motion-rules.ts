// The motion and state rules of design language §7.0, checked in every stylesheet the apps and
// sites ship: durations, easings and the action cursors come from tokens, no transition names
// `all`, and `:hover` only inside `@media (hover: hover)`, since a tap leaves plain `:hover` on.
// What breaks a rule today is listed in motion-baseline.json, which only shrinks: a new
// offender fails, and so does a listed one that is gone (remove it from the list).

import { readdirSync, readFileSync } from "node:fs";

const ROOT = new URL("../../../", import.meta.url);
/** The folders whose CSS (and Astro `<style>` blocks) people see. */
const DIRS = [
  "packages/ui/src/",
  "packages/site-kit/src/",
  "apps/web/src/",
  "apps/desktop/src/",
  "apps/docs/src/",
  "apps/site/src/",
];

export type Kind = "duration" | "easing" | "cursor" | "transition-all" | "hover";

/** Each stylesheet's CSS, comments removed: `.css` files whole, `.astro` files' `<style>` blocks. */
export function sheets(): [string, string][] {
  return DIRS.flatMap((dir) =>
    readdirSync(new URL(dir, ROOT), { recursive: true, encoding: "utf8" })
      .map((f) => f.replaceAll("\\", "/"))
      .filter((f) => /\.(css|astro)$/.test(f) && !f.includes("generated"))
      .map((f): [string, string] => {
        const text = readFileSync(new URL(`${dir}${f}`, ROOT), "utf8");
        const css = f.endsWith(".astro")
          ? [...text.matchAll(/<style[^>]*>([\s\S]*?)<\/style>/g)].map((m) => m[1] ?? "").join("\n")
          : text;
        return [`${dir}${f}`, css.replace(/\/\*[\s\S]*?\*\//g, "")];
      }),
  );
}

/** Each declaration with the at-rules around it (`@media (hover: hover)` …) and its selector. */
function declarations(css: string): { decl: string; selector: string; media: string[] }[] {
  const out: { decl: string; selector: string; media: string[] }[] = [];
  const stack: string[] = [];
  let buffer = "";
  for (const ch of css) {
    if (ch === "{") {
      stack.push(buffer.trim());
      buffer = "";
    } else if (ch === "}") {
      if (buffer.trim() !== "") out.push(...toDeclaration(buffer, stack));
      buffer = "";
      stack.pop();
    } else if (ch === ";") {
      out.push(...toDeclaration(buffer, stack));
      buffer = "";
    } else {
      buffer += ch;
    }
  }
  return out;
}

/** `buffer` as a declaration, with its selector and the at-rules around it; none if it isn't one. */
function toDeclaration(buffer: string, stack: string[]) {
  const decl = buffer.trim();
  if (decl === "" || !decl.includes(":")) return [];
  const selector = [...stack].reverse().find((s) => !s.startsWith("@")) ?? "";
  return [{ decl, selector, media: stack.filter((s) => s.startsWith("@")) }];
}

type Found = [Kind, string][];

/** Literal durations in a transition or animation (0, 0s and 0ms are no time, so they pass). */
function durations(name: string, value: string): Found {
  if (!/^(transition|animation)(-duration|-delay)?$/.test(name)) return [];
  return [...value.matchAll(/(?<![\w-])(\d*\.?\d+)m?s\b/g)]
    .filter((m) => Number(m[1]) !== 0)
    .map((m): [Kind, string] => ["duration", `${name}: ${m[0]}`]);
}

/** Easings written in place: `ease`, its variants and `cubic-bezier()` (`linear` and `steps()` pass). */
function easings(name: string, value: string): Found {
  if (!/^(transition|animation)(-timing-function)?$/.test(name)) return [];
  return [...value.matchAll(/cubic-bezier\(|(?<![\w-])ease(?:-in-out|-in|-out)?(?![\w-])/g)].map(
    (m): [Kind, string] => ["easing", `${name}: ${m[0]}`],
  );
}

/** The action, disabled and pending cursors written in place, not read from their tokens. */
function cursors(name: string, value: string): Found {
  const keyword = value.replace(/\s*!important$/, "");
  return name === "cursor" && /^(pointer|default|progress)$/.test(keyword) ? [["cursor", keyword]] : [];
}

function transitionAll(name: string, value: string): Found {
  return /^transition(-property)?$/.test(name) && /(?:^|[\s,])all(?:[\s,]|$)/.test(value)
    ? [["transition-all", `${name}: ${value}`]]
    : [];
}

/** `@media (hover: hover) and (pointer: fine)`, the guard §7.0 asks of a raw `:hover`: both
    features, and neither negated. */
function hoverGuard(media: string): boolean {
  return /\(\s*hover:\s*hover\s*\)/.test(media) && /\(\s*pointer:\s*fine\s*\)/.test(media) && !/\bnot\b/.test(media);
}

/** Every break of a rule in `css`, as `kind: what`; a hover selector once, however many declarations. */
export function offences(css: string): Found {
  const found: Found = [];
  const hovers = new Set<string>();
  for (const { decl, selector, media } of declarations(css)) {
    const [prop = "", ...rest] = decl.split(":");
    const name = prop.trim();
    const value = rest.join(":").trim();
    found.push(
      ...durations(name, value),
      ...easings(name, value),
      ...cursors(name, value),
      ...transitionAll(name, value),
    );
    const guarded = media.some(hoverGuard);
    const shown = selector.replace(/\s+/g, " ");
    if (selector.includes(":hover") && !guarded && !hovers.has(shown)) {
      hovers.add(shown);
      found.push(["hover", shown]);
    }
  }
  return found;
}
