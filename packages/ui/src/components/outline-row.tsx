// An outline row (§7.7): a link to a section with its page number at the far end, at one of two
// levels, tinted when it holds the current position. A row with children has a chevron button
// before the link that shows or hides them, rather than a chevron inside the link, so the link
// and the button each do one thing.

import { Icon } from "@laminapdf/icons/react";
import type { ReactNode } from "react";
import { Button as AriaButton, Link } from "react-aria-components";

/** The chevron that shows or hides a row's children. */
export interface OutlineToggle {
  /** Whether the children show. */
  isExpanded: boolean;
  /** Called with the new state when the chevron is pressed. */
  onExpandedChange: (isExpanded: boolean) => void;
  /** The chevron's name, the same open or closed: "Sections in 2. Repairs backlog". */
  label: string;
}

/** Props of `OutlineRow`. */
export interface OutlineRowProps {
  /** The section title. */
  title: string;
  /** Its page label, shown at the far end. */
  page: string;
  /** 1 for a top-level section, 2 for one inside it. */
  level?: 1 | 2;
  /** The section that holds the current position: tinted, and `aria-current="location"`. */
  isCurrent?: boolean;
  /** Where the link goes, for a real URL. */
  href?: string;
  /** Called when the link is followed, for in-app navigation. */
  onPress?: () => void;
  /** The chevron for a row with children; `"space"` keeps its room in a tree where other rows have one. */
  toggle?: OutlineToggle | "space";
}

/** One row of the document outline, inside the app's `<nav aria-label="Document outline">`. */
export function OutlineRow({
  title,
  page,
  level = 1,
  isCurrent = false,
  href,
  onPress,
  toggle,
}: OutlineRowProps): ReactNode {
  return (
    <div className="paper-outline-row" data-level={level} data-current={isCurrent || undefined}>
      {toggle === undefined ? null : toggle === "space" ? (
        <span className="paper-outline-toggle" aria-hidden="true" />
      ) : (
        <AriaButton
          className="paper-outline-toggle"
          aria-label={toggle.label}
          aria-expanded={toggle.isExpanded}
          onPress={() => toggle.onExpandedChange(!toggle.isExpanded)}
        >
          <Icon name={toggle.isExpanded ? "chevron-down" : "chevron-right"} />
        </AriaButton>
      )}
      <Link
        className="paper-outline-link"
        {...(href === undefined ? {} : { href })}
        {...(onPress === undefined ? {} : { onPress })}
        {...(isCurrent ? { "aria-current": "location" } : {})}
      >
        <span className="paper-outline-title">{title}</span>
        <span className="paper-outline-page">{page}</span>
      </Link>
    </div>
  );
}
