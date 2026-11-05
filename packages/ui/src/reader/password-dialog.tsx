// The password prompt (R03): a required question over the reader while an encrypted document waits
// for its password. The password goes to the controller once per try and lives only in this field,
// which empties when the document changes; a wrong one is announced under the field, which keeps
// focus with its text selected. Open reads "Opening…" while the try runs. Cancel and Escape go
// back to the home screen; a click outside or a swipe doesn't, since the question needs an answer.

import { Icon } from "@laminapdf/icons/react";
import { type FormEvent, type ReactNode, useEffect, useId, useRef, useState } from "react";
import { ToggleButton as AriaToggleButton } from "react-aria-components";
import { Button } from "../components/button.js";
import { message } from "../messages.js";
import { Dialog } from "../overlays/dialog.js";
import type { PlatformFile } from "../platform/types.js";
import { useStoreState } from "../state/hooks.js";
import type { ReaderController } from "../state/reader-controller.js";

/** Props of `PasswordDialog`. */
export interface PasswordDialogProps {
  /** The reader whose document needs a password: the dialog opens while it asks for one. */
  reader: ReaderController;
}

/** Asks for an encrypted document's password whenever the reader needs one. */
export function PasswordDialog({ reader }: PasswordDialogProps): ReactNode {
  const state = useStoreState(reader, (s) => s);
  const file = state.phase === "empty" ? undefined : state.file;
  const [password, setPassword] = useState("");
  const [shown, setShown] = useState(false);
  // The document whose password is being tried: another one opening ends the try.
  const [trying, setTrying] = useState<PlatformFile | undefined>(undefined);
  const field = useRef<HTMLInputElement>(null);
  const errorId = useId();
  const checking = trying !== undefined && state.phase === "opening" && trying === file;
  const wrong = state.phase === "needs-password" && state.wrong;
  const isOpen = state.phase === "needs-password" || checking;
  useEffect(() => {
    if (state.phase !== "opening") setTrying(undefined);
    if (state.phase === "needs-password" && state.wrong) {
      field.current?.focus();
      field.current?.select();
    }
  }, [state]);
  // A different document, or none, never sees what was typed for this one.
  // biome-ignore lint/correctness/useExhaustiveDependencies: runs when the document changes
  useEffect(() => {
    setPassword("");
    setShown(false);
  }, [file]);
  const submit = (event?: FormEvent) => {
    event?.preventDefault();
    if (password === "" || checking) return;
    setTrying(file);
    void reader.submitPassword(password);
  };
  return (
    <Dialog
      isOpen={isOpen}
      onOpenChange={(open) => open || reader.cancel()}
      isRequired
      onEscape={() => reader.cancel()}
      title={message("reader.password.title")}
      description={message("reader.password.body", { name: file?.name ?? "" })}
      actions={
        <>
          <Button onPress={() => reader.cancel()}>{message("reader.password.cancel")}</Button>
          {/* Pending rather than disabled, so a press keeps focus on it while the try runs. */}
          <Button variant="primary" isPending={checking} onPress={() => submit()}>
            {message(checking ? "reader.password.opening" : "reader.password.open")}
          </Button>
        </>
      }
    >
      <form className="paper-password" onSubmit={submit}>
        <div className="paper-password-field" data-invalid={wrong || undefined}>
          <input
            ref={field}
            className="paper-password-input"
            type={shown ? "text" : "password"}
            autoComplete="current-password"
            aria-label={message("reader.password.field")}
            aria-invalid={wrong || undefined}
            aria-describedby={wrong ? errorId : undefined}
            value={password}
            readOnly={checking}
            onChange={(event) => setPassword(event.target.value)}
          />
          <AriaToggleButton
            className="paper-password-show"
            aria-label={message("reader.password.showName")}
            isSelected={shown}
            onChange={setShown}
          >
            {message("reader.password.show")}
          </AriaToggleButton>
        </div>
        {wrong ? (
          <p id={errorId} role="alert" className="paper-password-error">
            <Icon name="warning" size={16} />
            {message("reader.password.wrong")}
          </p>
        ) : null}
        <p className="paper-password-help">{message("reader.password.help")}</p>
      </form>
    </Dialog>
  );
}
