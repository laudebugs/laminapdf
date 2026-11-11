// Dialogs (§7.18) on React Aria's modal: the scrim, focus trapped inside and returned to the
// opener, Escape and scrim clicks closing it unless it asks a required question, the page behind
// inert and its scrolling held still.

import { type ReactNode, type RefObject, useEffect, useId, useRef, useState, useSyncExternalStore } from "react";
import { Dialog as AriaDialog, Heading, Modal, ModalOverlay } from "react-aria-components";
import { PHONE_QUERY } from "../breakpoints.js";
import { IconButton } from "../components/button.js";
import { markModal } from "./aria-modal.js";
import { LayerScope } from "./layers.js";
import { BottomSheet } from "./sheet.js";

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
  /**
   * A required question that Escape answers as its Cancel does (R03's password prompt): called on
   * Escape, while the scrim and swipes still leave the dialog open.
   */
  onEscape?: () => void;
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

/** The phone's width, where a dialog becomes a bottom sheet (§7.18); one list for every dialog. */
let phoneQuery: MediaQueryList | undefined;
function phone(): MediaQueryList {
  phoneQuery ??= window.matchMedia(PHONE_QUERY);
  return phoneQuery;
}

function subscribePhone(onChange: () => void): () => void {
  phone().addEventListener("change", onChange);
  return () => phone().removeEventListener("change", onChange);
}

/**
 * Whether the dialog lays out as a phone's sheet: the viewport's width when it opens, kept until
 * it closes, so turning the phone doesn't rebuild an open dialog and lose what was typed in it.
 */
export function usePhoneLayout(isOpen: boolean): boolean {
  const live = useSyncExternalStore(
    subscribePhone,
    () => phone().matches,
    () => false,
  );
  const [opened, setOpened] = useState(live);
  useEffect(() => {
    if (!isOpen) setOpened(live);
  }, [isOpen, live]);
  return isOpen ? opened : live;
}

/** A modal dialog; on a phone-wide viewport, a bottom sheet at the large detent with full-width actions. */
export function Dialog(props: DialogProps): ReactNode {
  const { isOpen, onOpenChange, title, description, children, actions, size = "default", role, returnFocusTo } = props;
  const required = props.isRequired === true;
  const descriptionId = useId();
  const asSheet = usePhoneLayout(isOpen);
  useFocusFallback(isOpen, returnFocusTo);
  if (asSheet) {
    return (
      <BottomSheet
        isOpen={isOpen}
        onOpenChange={onOpenChange}
        title={title}
        defaultDetent="large"
        {...(description === undefined ? {} : { description })}
        {...(role === undefined ? {} : { role })}
        {...(props.isRequired === true ? { isRequired: true } : { closeLabel: props.closeLabel })}
        {...(actions === undefined ? {} : { footer: actions })}
      >
        {children}
        <FirstFieldFocus />
      </BottomSheet>
    );
  }
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
          ref={markModal}
          {...(role === undefined ? {} : { role })}
          {...(description === undefined ? {} : { "aria-describedby": descriptionId })}
        >
          {({ close }) => (
            <LayerScope dismiss={props.onEscape ?? close} dismissable={!required || props.onEscape !== undefined}>
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
            </LayerScope>
          )}
        </AriaDialog>
      </Modal>
    </ModalOverlay>
  );
}

const FIELDS = "input:not([type=hidden]):not(:disabled), select:not(:disabled), textarea:not(:disabled)";

/** The first field to focus: a radio group counts by its checked radio when it has one. */
function firstField(dialog: HTMLElement): HTMLElement | undefined {
  for (const field of dialog.querySelectorAll<HTMLElement>(FIELDS)) {
    if (!(field instanceof HTMLInputElement) || field.type !== "radio" || field.checked) return field;
    const group = field.name === "" ? [] : [...dialog.querySelectorAll<HTMLInputElement>("input[type=radio]")];
    if (!group.some((r) => r.name === field.name && r.checked)) return field;
  }
  return undefined;
}

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
    firstField(dialog)?.focus();
  }, []);
  return <span ref={marker} hidden />;
}

/**
 * After the dialog closes, focuses `fallback` when focus came back to nothing (its opener is
 * gone), whether the dialog closed through `isOpen` or by unmounting.
 */
function useFocusFallback(isOpen: boolean, fallback: (() => HTMLElement | null) | undefined): void {
  // The compiler can't follow the latest-value ref kept here for effects (useEffectEvent could replace it).
  "use no memo";
  const latest = useRef(fallback);
  latest.current = fallback;
  const wasOpen = useRef(false);
  useEffect(() => {
    if (isOpen) wasOpen.current = true;
    else if (wasOpen.current) {
      wasOpen.current = false;
      focusFallbackLater(latest);
    }
  }, [isOpen]);
  useEffect(
    () => () => {
      if (wasOpen.current) focusFallbackLater(latest);
    },
    [],
  );
}

// React Aria restores focus as the dialog unmounts; check once a frame has passed.
function focusFallbackLater(fallback: RefObject<(() => HTMLElement | null) | undefined>): void {
  requestAnimationFrame(() => {
    const active = document.activeElement;
    if (active === null || active === document.body || !active.isConnected) fallback.current?.()?.focus();
  });
}
