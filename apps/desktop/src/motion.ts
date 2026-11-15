// Reduced motion in the desktop window (motion plan §5): Settings › Motion, kept by the main
// process with the other preferences, and the system's setting. The webview reads the system's
// `prefers-reduced-motion` itself, except where WebKitGTK misses GNOME's (src-tauri's motion.rs
// reads that one), so the root carries `data-motion="reduce"` when the choice is Reduce motion or
// the main process says the system asks for it. The tokens and @laminapdf/ui read that attribute
// as they read the media query.

import type { Motion } from "@laminapdf/ui";
import { useEffect, useState } from "react";
import type { DesktopShell } from "./engine/desktop-engine-client.ts";

/**
 * Whether the window reduces motion.
 * @param choice - Settings › Motion.
 * @param system - What the main process read of the system's setting: `true` when it asks for
 *   reduced motion, `null` where the webview reads the setting itself.
 */
export function reducesMotion(choice: Motion, system: boolean | null): boolean {
  return choice === "reduce" || system === true;
}

/**
 * Sets `data-motion` on the document's root from `choice` and the system's setting, which the main
 * process reads at start and whenever the window comes to the front (GNOME's setting may have
 * changed meanwhile).
 */
export function useMotion(shell: DesktopShell | undefined, choice: Motion): void {
  const [system, setSystem] = useState<boolean | null>(null);
  useEffect(() => {
    if (shell === undefined) return;
    const read = () => {
      shell.invoke<boolean | null>("system_motion").then(setSystem, (error: unknown) => {
        console.error("reading the system's motion setting failed", error);
      });
    };
    read();
    window.addEventListener("focus", read);
    return () => window.removeEventListener("focus", read);
  }, [shell]);
  useEffect(() => {
    const root = document.documentElement;
    if (reducesMotion(choice, system)) root.setAttribute("data-motion", "reduce");
    else root.removeAttribute("data-motion");
  }, [choice, system]);
}
