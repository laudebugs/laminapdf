// A label that keeps its width when it turns bold (motion plan §3, 7): a hidden bold copy shares
// its grid cell (styles/states.css), so selecting a tab, mode or toggle moves nothing. The copy
// is aria-hidden and invisible, so it adds nothing to the control's name or to find in page.

import type { ReactNode } from "react";

/** `children` in a box as wide as their bold form. */
export function StableLabel({ children }: { children: ReactNode }): ReactNode {
  return (
    <span className="paper-stable-label">
      <span>{children}</span>
      <span className="paper-stable-label-bold" aria-hidden="true">
        {children}
      </span>
    </span>
  );
}
