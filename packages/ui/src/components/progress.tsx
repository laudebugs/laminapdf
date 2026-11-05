// Progress (§7.22): a bar with its label above, determinate when it knows how far along it is and
// indeterminate otherwise (a sliding segment that stands still under reduced motion), and the
// skeleton blocks that stand in for content while it loads.

import type { CSSProperties, ReactNode } from "react";
import { ProgressBar as AriaProgressBar, Label } from "react-aria-components";

/** Props of `ProgressBar`. */
export interface ProgressBarProps {
  /** The visible label: "Saving…". The percentage follows it when the bar is determinate. */
  label: string;
  /** How far along, from 0 to 100; leave it out while that isn't known. */
  value?: number;
}

/** A progress bar (`role="progressbar"`): `aria-valuenow` is set when `value` is given. */
export function ProgressBar({ label, value }: ProgressBarProps): ReactNode {
  return (
    <AriaProgressBar className="paper-progress" value={value ?? 0} isIndeterminate={value === undefined}>
      {({ percentage, valueText, isIndeterminate }) => (
        <>
          <span className="paper-progress-label">
            <Label>{label}</Label>
            {isIndeterminate ? null : <span>{valueText}</span>}
          </span>
          <span className="paper-progress-track">
            <span className="paper-progress-fill" style={{ "--progress": `${percentage ?? 0}%` } as CSSProperties} />
          </span>
        </>
      )}
    </AriaProgressBar>
  );
}

/** Props of `Skeleton`. */
export interface SkeletonProps {
  /** The block's width as a CSS length or percentage; the whole line when left out. */
  inlineSize?: string;
  /** The block's height; one line of text when left out. */
  blockSize?: string;
}

/**
 * A placeholder block while content loads. It is hidden from assistive technology, so mark the
 * region that holds it `aria-busy="true"` until the content arrives.
 */
export function Skeleton({ inlineSize, blockSize }: SkeletonProps): ReactNode {
  return <span className="paper-skeleton" aria-hidden="true" style={{ inlineSize, blockSize }} />;
}
