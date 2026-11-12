// The capability page's place in the browser history, shared by the route at start (capabilities.tsx)
// and the page that loads later (capability-page.tsx), which would otherwise import each other.

/** The page's address. */
export const CAPABILITIES_PATH = "/capabilities";

/** Marks the history entry this reader pushed for the page, so the page's Back can go back to it. */
export const CAPABILITY_HISTORY_KEY = "laminaCapabilities";
