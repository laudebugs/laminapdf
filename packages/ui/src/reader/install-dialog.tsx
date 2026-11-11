// Installing the reader (R15): a dialog the user opens from More › Install Lamina PDF…, never
// shown unprompted. It says what installing gives (Open with, no connection needed, files stay
// on the device) and that the browser asks to confirm; Install, focused, hands over to the
// browser's own prompt. Cancelling that prompt changes nothing.

import { Icon } from "@laminapdf/icons/react";
import type { ReactNode } from "react";
import { Button } from "../components/button.js";
import { message } from "../messages.js";
import { Dialog } from "../overlays/dialog.js";

/** Props of `InstallDialog`. */
export interface InstallDialogProps {
  isOpen: boolean;
  /** Install was pressed: show the browser's prompt. */
  onInstall: () => void;
  /** Not now, Close, Escape or the scrim. */
  onClose: () => void;
}

/** Offers to install the reader as an app. */
export function InstallDialog({ isOpen, onInstall, onClose }: InstallDialogProps): ReactNode {
  return (
    <Dialog
      isOpen={isOpen}
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
      title={message("install.title")}
      closeLabel={message("install.close")}
      actions={
        <>
          <Button onPress={onClose}>{message("install.notNow")}</Button>
          <Button variant="primary" autoFocus onPress={onInstall}>
            {message("install.install")}
          </Button>
        </>
      }
    >
      <ul className="paper-install-facts">
        <li>
          <Icon name="folder" />
          {message("install.openWith")}
        </li>
        <li>
          <Icon name="offline" />
          {message("install.offline")}
        </li>
        <li>
          <Icon name="lock" />
          {message("install.private")}
        </li>
      </ul>
      <p className="paper-install-note">{message("install.note")}</p>
    </Dialog>
  );
}
