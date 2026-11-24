// The View settings popover's Page appearance section (R10): the four appearance chips, then
// whether the choice is for this document or all of them, and a line saying photos and
// annotation colours stay as they are. Without a document open the choice is for all documents.
// Also holds the DOM/viewer application helpers so the web and desktop readers apply page
// appearances identically without copying DOM probes or motion suppression.

import type { PageAppearance } from "@laminapdf/tokens";
import { type ReactNode, useEffect } from "react";
import { PageAppearanceChips } from "../components/appearance-chips.js";
import { Radio, RadioGroup } from "../components/choice.js";
import { message } from "../messages.js";
import { suppressMotion } from "../motion.js";

/** Whom a page appearance is chosen for. */
export type PageAppearanceScope = "document" | "all";

/** An sRGB colour with alpha in 0..255, as `@laminapdf/viewer` takes for High contrast. */
export interface PageRgba {
  readonly r: number;
  readonly g: number;
  readonly b: number;
  readonly a: number;
}

/** The viewer-side page appearance payload: a named mode, or High contrast in system colours. */
export type ViewerPageAppearance =
  | Exclude<PageAppearance, "high-contrast">
  | { readonly kind: "high-contrast"; readonly ink: PageRgba; readonly paper: PageRgba };

/** Structural slice of `@laminapdf/viewer`'s `Viewer` needed to apply a page appearance. */
export interface PageAppearanceTarget {
  /** Draws pages in `appearance`, redrawing visible pages in place. */
  setAppearance(appearance: ViewerPageAppearance): void;
}

/** Each appearance's visible name. */
const LABELS: Readonly<Record<PageAppearance, string>> = {
  original: message("pageAppearance.original"),
  sepia: message("pageAppearance.sepia"),
  dark: message("pageAppearance.dark"),
  "high-contrast": message("pageAppearance.high-contrast"),
};

/** An appearance's visible name ("Sepia"). */
export function pageAppearanceName(appearance: PageAppearance): string {
  return LABELS[appearance];
}

/** A system colour (`CanvasText`, `Canvas`) as the browser resolves it now. */
function systemColour(keyword: string): PageRgba {
  const probe = document.createElement("span");
  probe.style.color = keyword;
  document.body.append(probe);
  const [r = 0, g = 0, b = 0] = (getComputedStyle(probe).color.match(/\d+(\.\d+)?/g) ?? []).map(Number);
  probe.remove();
  return { r, g, b, a: 255 };
}

/** What the viewer draws for `name`: High contrast in the system's colours. */
function pageAppearanceForViewer(name: PageAppearance): ViewerPageAppearance {
  if (name !== "high-contrast") return name;
  return { kind: "high-contrast", ink: systemColour("CanvasText"), paper: systemColour("Canvas") };
}

/**
 * Applies `name` to the document root (`data-page-appearance`, suppressing transitions across the
 * switch) and to `viewer` when one is present.
 */
export function applyPageAppearance(
  root: HTMLElement,
  viewer: PageAppearanceTarget | undefined,
  name: PageAppearance,
): void {
  // A switch of look: nothing transitions from the old page colours (design language §7.0).
  if (root.dataset.pageAppearance !== name) suppressMotion(root);
  root.dataset.pageAppearance = name;
  viewer?.setAppearance(pageAppearanceForViewer(name));
}

/**
 * Keeps `document.documentElement` and `viewer` in step with `name`, re-reading system colours
 * when High contrast is active and the OS scheme or window focus changes.
 */
export function usePageAppearanceValue(viewer: PageAppearanceTarget | undefined, name: PageAppearance): void {
  useEffect(() => {
    const sync = () => applyPageAppearance(document.documentElement, viewer, name);
    sync();
    if (name !== "high-contrast") return;
    // High contrast follows the system's colours: read again when they may have changed.
    const scheme = window.matchMedia("(prefers-color-scheme: dark)");
    scheme.addEventListener("change", sync);
    window.addEventListener("focus", sync);
    return () => {
      scheme.removeEventListener("change", sync);
      window.removeEventListener("focus", sync);
    };
  }, [viewer, name]);
}

/** Props of `PageAppearanceFields`. */
export interface PageAppearanceFieldsProps {
  /** The appearance the pages are drawn in. */
  readonly value: PageAppearance;
  /** Called when the user chooses an appearance. */
  readonly onChange: (appearance: PageAppearance) => void;
  /** Whom the next choice is for; omitted while no document is open (then it is for all). */
  readonly scope?: PageAppearanceScope | undefined;
  /** Called when the user switches between this document and all documents. */
  readonly onScopeChange?: ((scope: PageAppearanceScope) => void) | undefined;
}

/** The chips, the scope and the help line: every choice applies at once. */
export function PageAppearanceFields({ value, onChange, scope, onScopeChange }: PageAppearanceFieldsProps): ReactNode {
  return (
    <div className="paper-page-appearance-fields">
      <span className="paper-field-label" aria-hidden="true">
        {message("pageAppearance.title")}
      </span>
      <PageAppearanceChips
        aria-label={message("pageAppearance.title")}
        labels={LABELS}
        value={value}
        onChange={onChange}
      />
      {scope === undefined ? null : (
        <RadioGroup
          label={message("pageAppearance.scope")}
          value={scope}
          onChange={(next) => onScopeChange?.(next as PageAppearanceScope)}
        >
          <Radio value="document">{message("pageAppearance.scope.document")}</Radio>
          <Radio value="all">{message("pageAppearance.scope.all")}</Radio>
        </RadioGroup>
      )}
      <p className="paper-field-help">{message("pageAppearance.help")}</p>
    </div>
  );
}
