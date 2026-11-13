// How the reader writes shortcuts: ⌘ on Apple's systems, Ctrl elsewhere.

import type { KeyPlatform } from "@laminapdf/ui";

/** The platform shortcut chips and the bookmark editor's move keys are written for. */
export const KEYS: KeyPlatform = /Mac|iPhone|iPad/.test(navigator.platform) ? "mac" : "other";
