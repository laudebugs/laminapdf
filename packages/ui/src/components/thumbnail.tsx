// A page thumbnail (§7.6): the page image in a white box with the thumbnail shadow, its number
// below, and a 2 px accent outline on the current page. The viewer draws the image and passes it
// in as children, so this component never touches the viewer.

import type { ReactNode } from "react";
import { Button as AriaButton, type ButtonProps as AriaButtonProps } from "react-aria-components";

/** Where the thumbnail sits: 116 × 150 in the rail, 122 × 158 in the phone's Pages sheet. */
export type ThumbnailSize = "rail" | "phone";

/** Props of `Thumbnail`. */
export interface ThumbnailProps extends Omit<AriaButtonProps, "children" | "className" | "style" | "aria-label"> {
  /** The accessible name, by page label and position: "Page A-1, 22 of 24". Include the number shown. */
  label: string;
  /** The number shown below: the page label ("A-1") or its position ("22"). */
  number: string;
  /** The page in view: outlined, its number bold, and `aria-current="page"`. */
  isCurrent?: boolean;
  /** `rail` when left out. */
  size?: ThumbnailSize;
  /** The page image: an `<img>` or a `<canvas>` the viewer drew. It is fitted inside the box. */
  children?: ReactNode;
}

/** A thumbnail button that goes to its page. */
export function Thumbnail({
  label,
  number,
  isCurrent = false,
  size = "rail",
  children,
  ...rest
}: ThumbnailProps): ReactNode {
  return (
    <AriaButton
      {...rest}
      aria-label={label}
      {...(isCurrent ? { "aria-current": "page" } : {})}
      className="paper-thumbnail"
      data-size={size}
      data-current={isCurrent || undefined}
    >
      <span className="paper-thumbnail-page">{children}</span>
      <span className="paper-thumbnail-number" aria-hidden="true">
        {number}
      </span>
    </AriaButton>
  );
}
