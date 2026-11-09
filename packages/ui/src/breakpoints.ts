// The window widths where the chrome changes layout, from the tokens (design language §6.2), as
// media queries: one home, so the shell and every dialog switch at the same width.

import { BREAKPOINTS } from "@laminapdf/tokens";

/** The phone: dialogs and panels become bottom sheets, and the bottom bar replaces the status bar. */
export const PHONE_QUERY = `(max-width: ${BREAKPOINTS["phone-max"]}px)`;
