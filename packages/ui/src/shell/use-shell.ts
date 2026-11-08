// The shell's state for one window: a `ShellLayout` over the platform's storage, read back once
// mounted, and the modes with Read registered. Leaving Read clears the selection it made.

import { useEffect, useState } from "react";
import { message } from "../messages.js";
import type { PlatformStorage } from "../platform/types.js";
import { ShellLayout } from "./layout.js";
import { ModeRegistry } from "./modes.js";

/** The layout and the modes, made once for the component's life. */
export function useShell(storage: PlatformStorage): { readonly layout: ShellLayout; readonly modes: ModeRegistry } {
  const [shell] = useState(() => {
    const layout = new ShellLayout(storage);
    const modes = new ModeRegistry();
    modes.register({ id: "read", title: message("command.mode.read"), icon: "reader", leave: () => layout.deselect() });
    return { layout, modes };
  });
  useEffect(() => void shell.layout.load(), [shell]);
  return shell;
}
