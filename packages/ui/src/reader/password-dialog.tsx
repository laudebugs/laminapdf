// The password prompt (R03): a dialog over the reader while an encrypted document waits for its
// password. The password goes to the controller once per try and lives only in this field; a
// wrong one is announced under the field, which keeps focus with its text selected. Open shows
// "Opening…" while the try runs; Cancel, Escape or Close go back to the home screen.

import { Icon } from "@laminapdf/icons/react";
import { type FormEvent, type ReactNode, useEffect, useId, useRef, useState } from "react";
import { ToggleButton as AriaToggleButton } from "react-aria-components";
import { Button } from "../components/button.js";
import { message } from "../messages.js";
import { Dialog } from "../overlays/dialog.js";
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
  const [password, setPassword] = useState("");
  const [shown, setShown] = useState(false);
  const [checking, setChecking] = useState(false);
  const field = useRef<HTMLInputElement>(null);
  const errorId = useId();
  const asking = state.phase === "needs-password";
  const wrong = asking && state.wrong;
  const isOpen = asking || (checking && state.phase === "opening");
  useEffect(() => {
    if (state.phase !== "opening") setChecking(false);
    if (state.phase === "needs-password" && state.wrong) field.current?.select();
  }, [state]);
  useEffect(() => {
    if (isOpen) return;
    setPassword("");
    setShown(false);
  }, [isOpen]);
  const submit = (event?: FormEvent) => {
    event?.preventDefault();
    if (password === "" || checking) return;
    setChecking(true);
    void reader.submitPassword(password);
  };
  return (
    <Dialog
      isOpen={isOpen}
      onOpenChange={(open) => open || reader.cancel()}
      title={message("reader.password.title")}
      description={message("reader.password.body", { name: state.phase === "empty" ? "" : state.file.name })}
      closeLabel={message("reader.password.close")}
      actions={
        <>
          <Button onPress={() => reader.cancel()}>{message("reader.password.cancel")}</Button>
          <Button variant="primary" isDisabled={checking} onPress={() => submit()}>
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
