// The frame of a panel docked in the inspector (R04's repairs, R13's network activity): a header
// with the title, a line under it and a close button, the body below. Escape closes it.

import { type ReactNode, useId } from "react";
import { IconButton } from "../components/button.js";

/** Props of `DockedPanel`. */
export interface DockedPanelProps {
  title: string;
  /** The line under the title: the file's name, or since when the panel counts. */
  subtitle: ReactNode;
  /** The close button's accessible name. */
  closeLabel: string;
  /** Closes the panel: the close button and Escape. */
  onClose: () => void;
  children: ReactNode;
}

/** A panel for the inspector slot, named by its title. */
export function DockedPanel({ title, subtitle, closeLabel, onClose, children }: DockedPanelProps): ReactNode {
  const id = useId();
  return (
    <aside
      className="paper-repair"
      aria-labelledby={id}
      onKeyDown={(event) => {
        if (event.key !== "Escape") return;
        event.stopPropagation();
        onClose();
      }}
    >
      <header className="paper-repair-header">
        <div>
          <h2 id={id} className="paper-repair-title">
            {title}
          </h2>
          <p className="paper-repair-file">{subtitle}</p>
        </div>
        <IconButton icon="close" label={closeLabel} onPress={onClose} />
      </header>
      {children}
    </aside>
  );
}
