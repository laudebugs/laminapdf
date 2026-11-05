// `<Icon>`: the icons as a React 19 component, kept off the root entry so it stays framework-free.

import { createElement, type ReactElement } from "react";
import { ICONS, type IconName } from "../generated/icons.js";
import { type IconSize, iconButtonProps } from "../index.js";

/** Props of `<Icon>`. */
export interface IconProps {
  /** The icon. */
  name: IconName;
  /** Width and height in CSS px; 16 when left out. */
  size?: IconSize;
  /**
   * An accessible name, for an icon that stands alone (`role="img"`). Leave it out inside a
   * labelled button: the icon is then decorative and hidden from assistive technology.
   */
  label?: string;
  /** A class for the `<svg>`; the colour comes from the parent's `color` (`currentColor`). */
  className?: string;
}

/** A Paper icon: decorative by default, or an image with an accessible name when given a `label`. */
export function Icon({ name, size = 16, label, className }: IconProps): ReactElement {
  const a11y =
    label === undefined
      ? { "aria-hidden": true, focusable: "false" }
      : { role: "img", "aria-label": iconButtonProps(label)["aria-label"] };
  return createElement("svg", {
    xmlns: "http://www.w3.org/2000/svg",
    width: size,
    height: size,
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: 1.5,
    ...(className === undefined ? {} : { className }),
    ...a11y,
    // The markup comes from the package's own lint-checked files, never from input.
    dangerouslySetInnerHTML: { __html: ICONS[name] },
  });
}
