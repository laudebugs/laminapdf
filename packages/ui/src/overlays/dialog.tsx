// Dialogs (§7.18) on React Aria's modal: the scrim, focus trapped inside and returned to the
// opener, Escape and scrim clicks closing it unless it asks a required question, the page behind
// inert and its scrolling held still.

import { type ReactNode, useEffect, useId, useRef } from "react";
import { Dialog as AriaDialog, Heading, Modal, ModalOverlay } from "react-aria-components";
import { IconButton } from "../components/button.js";
import { LayerRegistration } from "./layers.js";

/** How wide a dialog may grow: 440 by default, 560 for a form, 820 for a preview or two panes. */
export type DialogSize = "default" | "form" | "wide";

interface DialogBase {
  /** Whether the dialog is open. */
  isOpen: boolean;
  /** Called with `false` when the dialog asks to close: Escape, the scrim, the close button. */
  onOpenChange: (isOpen: boolean) => void;
  /** The title, which names the dialog. */
  title: string;
  /** A first paragraph under the title, read with the name (`aria-describedby`). */
  description?: string;
  /** The body: fields, lists, notices. Focus starts on its first field, or give a control `autoFocus`. */
  children?: ReactNode;
  /** The buttons, right-aligned with the primary last. A destructive confirm puts `autoFocus` on Cancel. */
  actions?: ReactNode;
  /** `default` when left out. */
  size?: DialogSize;
  /** `alertdialog` for a destructive confirm. */
  role?: "dialog" | "alertdialog";
  /** Where focus goes when the dialog closes and the control that opened it is gone. */
  returnFocusTo?: () => HTMLElement | null;
}

/** Props of `Dialog`: a dialog that may be dismissed has a close button, which needs a name. */
export type DialogProps = DialogBase &
  (
    | {
        /** A required question: no close button, and Escape and the scrim leave it open. */ isRequired: true;
        closeLabel?: never;
      }
    | { isRequired?: false /** The close button's name: "Close". */; closeLabel: string }
  );

const FRAME: Record<DialogSize, string> = {
  default: "paper-dialog-frame",
  form: "paper-dialog-frame paper-dialog-frame-form",
  wide: "paper-dialog-frame paper-dialog-frame-wide",
};

/** A modal dialog. */
export function Dialog(props: DialogProps): ReactNode {
  const { isOpen, onOpenChange, title, description, children, actions, size = "default", role, returnFocusTo } = props;
  const required = props.isRequired === true;
  const descriptionId = useId();
  useFocusFallback(isOpen, returnFocusTo);
  return (
    <ModalOverlay
      isOpen={isOpen}
      onOpenChange={onOpenChange}
      isDismissable={!required}
      isKeyboardDismissDisabled={required}
      className="paper-modal-overlay"
    >
      <Modal className={FRAME[size]}>
        <AriaDialog
          className="paper-dialog"
          // React Aria makes the page behind inert but leaves aria-modal out, and drops it as a
          // prop; §7.18 asks for it, so it is set on the element.
          ref={markModal}
          {...(role === undefined ? {} : { role })}
          {...(description === undefined ? {} : { "aria-describedby": descriptionId })}
        >
          {({ close }) => (
            <>
              <LayerRegistration dismiss={close} dismissable={!required} />
              <Heading slot="title" className="paper-dialog-title">
                {title}
              </Heading>
              {description === undefined ? null : (
                <p id={descriptionId} className="paper-dialog-description">
                  {description}
                </p>
              )}
              {children}
              {actions === undefined ? null : <div className="paper-dialog-actions">{actions}</div>}
              <FirstFieldFocus />
              {/* Last in the DOM, so focus starts on the first field or action; dialog.css puts it top right. */}
              {props.isRequired === true ? null : (
                <span className="paper-dialog-close">
                  <IconButton label={props.closeLabel} icon="close" size={28} onPress={close} />
                </span>
              )}
            </>
          )}
        </AriaDialog>
      </Modal>
    </ModalOverlay>
  );
}

const FIELDS = "input:not([type=hidden]):not(:disabled), select:not(:disabled), textarea:not(:disabled)";

/**
 * Focuses the dialog's first field as it opens, unless something inside already has focus (a
 * control with `autoFocus`). React Aria would focus the dialog itself; this effect runs before its
 * own, because a child's effects run before its parent's.
 */
function FirstFieldFocus(): ReactNode {
  const marker = useRef<HTMLSpanElement>(null);
  useEffect(() => {
    const dialog = marker.current?.closest<HTMLElement>("[role=dialog], [role=alertdialog]");
    if (dialog === null || dialog === undefined || dialog.contains(document.activeElement)) return;
    dialog.querySelector<HTMLElement>(FIELDS)?.focus();
  }, []);
  return <span ref={marker} hidden />;
}

function markModal(element: HTMLElement | null): void {
  element?.setAttribute("aria-modal", "true");
}

/** After the dialog closes, focuses `fallback` when focus came back to nothing (its opener is gone). */
function useFocusFallback(isOpen: boolean, fallback: (() => HTMLElement | null) | undefined): void {
  const wasOpen = useRef(false);
  useEffect(() => {
    if (isOpen) {
      wasOpen.current = true;
      return;
    }
    if (!wasOpen.current || fallback === undefined) return;
    wasOpen.current = false;
    // React Aria restores focus as the dialog unmounts; check once that has happened.
    const frame = requestAnimationFrame(() => {
      const active = document.activeElement;
      if (active === null || active === document.body || !active.isConnected) fallback()?.focus();
    });
    return () => cancelAnimationFrame(frame);
  }, [isOpen, fallback]);
}
