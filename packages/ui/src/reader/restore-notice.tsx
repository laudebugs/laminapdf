// What a window says after it brings the last session back (desktop shell S02): a strip under the
// top bar that counts the tabs it restored and the files it couldn't find, with Show missing file
// and Dismiss. Its sentence is a polite status, so a screen reader hears it once; the app takes it away on
// Dismiss or the reader's first navigation. Locating a missing file that looks different from the
// one in the session asks first (`LocateDifferentDialog`).

import { Icon } from "@laminapdf/icons/react";
import type { ReactNode } from "react";
import { Button, IconButton } from "../components/button.js";
import { message } from "../messages.js";
import { Dialog } from "../overlays/dialog.js";

/** Props of `RestoreNotice`. */
export interface RestoreNoticeProps {
  /** How many tabs the window brought back. */
  tabs: number;
  /** How many of their files couldn't be found. */
  missing: number;
  /** Show missing file was pressed: the app shows the first tab whose file is missing. */
  onShowMissing: () => void;
  /** Dismiss was pressed. */
  onDismiss: () => void;
}

/** Says the last session came back, and how many of its files are missing. */
export function RestoreNotice({ tabs, missing, onShowMissing, onDismiss }: RestoreNoticeProps): ReactNode {
  return (
    <div className="paper-restore-notice" data-missing={missing > 0 || undefined}>
      <Icon name={missing > 0 ? "warning" : "check"} />
      {/* The sentence alone is the status, so a screen reader hears it without the buttons. */}
      <p role="status">
        <strong>{message("restore.title")}</strong>{" "}
        {tabs === 1 ? message("restore.tabs.one") : message("restore.tabs.other", { count: tabs })}
        {missing === 0
          ? null
          : ` ${missing === 1 ? message("restore.missing.one") : message("restore.missing.other", { count: missing })}`}
      </p>
      {missing === 0 ? null : (
        <Button variant="ghost" onPress={onShowMissing}>
          {message(missing === 1 ? "restore.showMissing.one" : "restore.showMissing.other")}
        </Button>
      )}
      <IconButton icon="close" size={26} label={message("restore.dismiss")} onPress={onDismiss} />
    </div>
  );
}

/** Props of `LocateDifferentDialog`. */
export interface LocateDifferentDialogProps {
  /** The chosen file's name while the question is asked; undefined otherwise. */
  name: string | undefined;
  /** The answer: `true` opens the chosen file anyway. */
  onAnswer: (open: boolean) => void;
}

/** Asks before a located file whose size and date differ from the session's opens in its tab. */
export function LocateDifferentDialog({ name, onAnswer }: LocateDifferentDialogProps): ReactNode {
  return (
    <Dialog
      isOpen={name !== undefined}
      isRequired
      role="alertdialog"
      onOpenChange={(open) => {
        if (!open) onAnswer(false);
      }}
      onEscape={() => onAnswer(false)}
      title={message("restore.different.title")}
      description={message("restore.different.body", { name: name ?? "" })}
      actions={
        <>
          <Button autoFocus onPress={() => onAnswer(false)}>
            {message("edit.close.cancel")}
          </Button>
          <Button variant="primary" onPress={() => onAnswer(true)}>
            {message("restore.different.open")}
          </Button>
        </>
      }
    />
  );
}
