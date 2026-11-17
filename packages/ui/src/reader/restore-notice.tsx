// What a window says after it brings the last session back (desktop shell S02): a strip under the
// top bar that counts the tabs it restored and the files it couldn't find, with Show missing file
// and Dismiss. Its sentence is a polite status, so a screen reader hears it once; the app takes it
// away on Dismiss or the user's first navigation. Locating a missing file that looks different
// from the one in the session asks first (`LocateDifferentDialog`).

import { Icon } from "@laminapdf/icons/react";
import { type ReactNode, useEffect, useState } from "react";
import { Button, IconButton } from "../components/button.js";
import { message } from "../messages.js";
import { QuestionDialog } from "../overlays/question-dialog.js";

/** Props of `RestoreNotice`. */
export interface RestoreNoticeProps {
  /** How many tabs the window brought back. */
  tabs: number;
  /** How many of their files couldn't be found. */
  missing: number;
  /** Show missing file was pressed: the app shows the first tab whose file is missing. Left out
      once no tab's file is missing: the button goes. */
  onShowMissing?: () => void;
  /** Dismiss was pressed. */
  onDismiss: () => void;
}

/** Says the last session came back, and how many of its files are missing. */
export function RestoreNotice({ tabs, missing, onShowMissing, onDismiss }: RestoreNoticeProps): ReactNode {
  // The status region is in the page, empty, before its sentence: a screen reader then says the
  // sentence, where it may skip a region that arrives with its text already in it.
  const [said, setSaid] = useState(false);
  useEffect(() => setSaid(true), []);
  const sentence = [
    tabs === 1 ? message("restore.tabs.one") : message("restore.tabs.other", { count: tabs }),
    missing === 0
      ? undefined
      : missing === 1
        ? message("restore.missing.one")
        : message("restore.missing.other", { count: missing }),
  ]
    .filter((part) => part !== undefined)
    .join(" ");
  return (
    <div className="paper-restore-notice" data-missing={missing > 0 || undefined}>
      <Icon name={missing > 0 ? "warning" : "check"} />
      {/* The sentence alone is the status, so a screen reader hears it without the buttons. */}
      <p role="status">
        {said ? (
          <>
            <strong>{message("restore.title")}</strong> {sentence}
          </>
        ) : null}
      </p>
      {onShowMissing === undefined ? null : (
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

/** Asks before a located file whose size or date differs from the session's opens in its tab. */
export function LocateDifferentDialog({ name, onAnswer }: LocateDifferentDialogProps): ReactNode {
  return (
    <QuestionDialog
      open={name !== undefined}
      title={message("restore.different.title")}
      description={message("restore.different.body", { name: name ?? "" })}
      no={message("restore.different.cancel")}
      yes={message("restore.different.open")}
      yesVariant="primary"
      onAnswer={onAnswer}
    />
  );
}
