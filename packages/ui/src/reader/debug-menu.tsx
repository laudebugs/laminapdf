// The debug menu: switches the render backend between PDFium and hayro to compare them (ADR 0004).
// The shell shows it only when the address's fragment asks for it (`#debug`, ADR 0011 §3.6), in
// the top bar's debug slot; nothing here reads the address.

import type { ReactNode } from "react";
import { IconButton } from "../components/button.js";
import type { KeyPlatform } from "../components/shortcut-chip.js";
import { message } from "../messages.js";
import { Menu, type MenuEntry, MenuTrigger } from "../overlays/popover.js";

/** A render backend the viewer can switch to. */
export type DebugBackend = "pdfium" | "hayro";

/** Props of `DebugMenu`. */
export interface DebugMenuProps {
  /** The backend rendering now; its entry shows a check. */
  backend: DebugBackend;
  /** Called with the backend chosen from the menu. */
  onBackendChange: (backend: DebugBackend) => void;
  keys: KeyPlatform;
}

const NAMES: Record<DebugBackend, string> = { pdfium: "PDFium", hayro: "hayro" };

/** The debug menu's button and its backend choices. */
export function DebugMenu({ backend, onBackendChange, keys }: DebugMenuProps): ReactNode {
  const entries: MenuEntry<DebugBackend>[] = (Object.keys(NAMES) as DebugBackend[]).map((id) => ({
    id,
    // The current one says so in words, which a screen reader reads, as well as with the check.
    label: message(id === backend ? "debug.backendCurrent" : "debug.backend", { backend: NAMES[id] }),
    ...(id === backend ? { icon: "check" as const } : {}),
  }));
  return (
    <MenuTrigger>
      <IconButton icon="tools" label={message("debug.menu")} />
      <Menu
        aria-label={message("debug.menu")}
        items={entries}
        onAction={onBackendChange}
        platform={keys}
        placement="bottom end"
      />
    </MenuTrigger>
  );
}
