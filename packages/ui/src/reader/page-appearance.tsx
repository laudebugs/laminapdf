// The View settings popover's Page appearance section (R10): the four appearance chips, then
// whether the choice is for this document or all of them, and a line saying photos and
// annotation colours stay as they are. Without a document open the choice is for all documents.

import type { PageAppearance } from "@laminapdf/tokens";
import type { ReactNode } from "react";
import { PageAppearanceChips } from "../components/appearance-chips.js";
import { Radio, RadioGroup } from "../components/choice.js";
import { message } from "../messages.js";

/** Whom a page appearance is chosen for. */
export type PageAppearanceScope = "document" | "all";

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
