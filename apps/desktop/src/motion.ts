// Reduced motion in the desktop window (motion plan §5): Settings › Appearance and accessibility ›
// Motion, kept by the main process with the other preferences, and the system's setting. The webview
// reads the system's `prefers-reduced-motion` itself, except where WebKitGTK misses GNOME's
// (src-tauri's motion.rs reads that one), so `useMotion` returns `"reduce"` when the choice is
// Reduce motion or the main process says the system asks for it. `applyAppearance` sets
// `data-motion="reduce"` from that effective value; the tokens and @laminapdf/ui read that
// attribute as they read the media query.

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
 * Resolves the effective motion setting (`"reduce"` or `"system"`) from `choice` and the system's
 * setting, which the main process reads at start and whenever the window comes to the front
 * (GNOME's setting may have changed meanwhile). Where it answers `null` (the webview reads the
 * setting itself) it is never asked again; an answer to an earlier read that arrives after a later
 * one is dropped.
 */
export function useMotion(shell: DesktopShell | undefined, choice: Motion): Motion {
  const [system, setSystem] = useState<boolean | null>(null);
  useEffect(() => {
    if (shell === undefined) return;
    let asked = 0;
    let answered = 0;
    const read = () => {
      const ask = ++asked;
      shell.invoke<boolean | null>("system_motion").then(
        (reduces) => {
          if (ask < answered) return;
          answered = ask;
          if (reduces === null) window.removeEventListener("focus", read);
          setSystem(reduces);
        },
        (error: unknown) => console.error("reading the system's motion setting failed", error),
      );
    };
    read();
    window.addEventListener("focus", read);
    return () => window.removeEventListener("focus", read);
  }, [shell]);
  return reducesMotion(choice, system) ? "reduce" : "system";
}
