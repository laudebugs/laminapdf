// Bottom sheets (§7.16, §6.2): a panel that rises from the bottom edge to a detent, medium (the
// page stays visible above it) or large (just under the top bar's space). The grabber is a real
// button that switches between them, so nothing needs a drag (WCAG 2.5.7); dragging the grabber
// or header snaps to the nearest detent, or past the medium one closes the sheet. A modal sheet
// is a dialog over the scrim; a non-modal one leaves the page usable and keeps the focused
// element from hiding under it (WCAG 2.4.11).

import { COMPONENT, SPACE } from "@laminapdf/tokens";
import { type PointerEvent, type ReactNode, type RefObject, useEffect, useId, useRef, useState } from "react";
import { Button as AriaButton, Dialog as AriaDialog, Modal, ModalOverlay } from "react-aria-components";
import { createPortal } from "react-dom";
import { IconButton } from "../components/button.js";
import { markModal } from "./aria-modal.js";
import { LayerScope, useLayerOpenedHere } from "./layers.js";

/** Where an open sheet rests. Closing it is the third, collapsed position. */
export type SheetDetent = "medium" | "large";

/** Props every `BottomSheet` takes. */
interface SheetProps {
  /** Whether the sheet is open. */
  isOpen: boolean;
  /** Called with `false` when the sheet asks to close: the close button, Escape, a drag down, the scrim. */
  onOpenChange: (isOpen: boolean) => void;
  /** The title, which names the sheet. */
  title: string;
  /** A first paragraph under the title, read with the name (`aria-describedby`). */
  description?: string;
  /** A count beside the title: "24 pages". */
  count?: string;
  /** The close button's name: "Close pages". Leave it out only with `isRequired`. */
  closeLabel?: string;
  /** The grabber's name: "Resize sheet". Leave it out for a sheet that stays at one detent. */
  resizeLabel?: string;
  /** The detent, when the app controls it. */
  detent?: SheetDetent;
  /** The detent it opens at; medium when left out. */
  defaultDetent?: SheetDetent;
  /** Called when the grabber or a drag moves the sheet to another detent. */
  onDetentChange?: (detent: SheetDetent) => void;
  /** A required question: no close button, and Escape, the scrim and a drag down leave it open. */
  isRequired?: boolean;
  /** The sheet's content, which scrolls. */
  children?: ReactNode;
  /** Actions below the content, full width: a dialog's buttons on the phone. */
  footer?: ReactNode;
}

/** Props of `BottomSheet`. Only a modal sheet is a dialog, so only it takes a `role`. */
export type BottomSheetProps = SheetProps &
  (
    | {
        /** A modal sheet asks something: focus stays inside and the page waits. True when left out. */
        isModal?: true;
        /** `alertdialog` for a destructive confirm. */
        role?: "dialog" | "alertdialog";
      }
    | {
        /** A non-modal sheet: a labelled region beside a page that stays in use. */
        isModal: false;
        role?: never;
      }
  );

/** A drag moves this far, in CSS px per ms, before it counts as a flick towards the next detent. */
const FLICK = 0.5;
/** A flick counts only when the pointer is released within this many ms of its last move. */
const FLICK_HOLD = 100;
/** A press that moves less than this, in CSS px, is a press, not a drag. */
const SLOP = 4;

/** The sheet's top, in CSS px from the viewport's top, at `detent`. */
function topOf(detent: SheetDetent): number {
  return detent === "large" ? COMPONENT["sheet-detent-large"] : window.innerHeight * COMPONENT["sheet-detent-medium"];
}

/** A bottom sheet. */
export function BottomSheet(props: BottomSheetProps): ReactNode {
  const opening = props.defaultDetent ?? "medium";
  const [ownDetent, setOwnDetent] = useState<SheetDetent>(opening);
  // Each opening starts at the default detent, not wherever the sheet was when it last closed.
  useEffect(() => {
    if (!props.isOpen) setOwnDetent(opening);
  }, [props.isOpen, opening]);
  if (!props.isOpen) return null;
  const placed: Placed = {
    detent: props.detent ?? ownDetent,
    setDetent: (next) => {
      setOwnDetent(next);
      props.onDetentChange?.(next);
    },
    close: () => {
      if (props.isRequired !== true) props.onOpenChange(false);
    },
  };
  return props.isModal === false ? (
    createPortal(<FreeSheet {...props} {...placed} />, document.body)
  ) : (
    <ModalSheet {...props} {...placed} />
  );
}

interface Placed {
  detent: SheetDetent;
  setDetent: (detent: SheetDetent) => void;
  close: () => void;
}

/** A modal sheet: a dialog over the scrim, with the page behind it inert. */
function ModalSheet(props: BottomSheetProps & Placed): ReactNode {
  const ref = useRef<HTMLDivElement>(null);
  const titleId = useId();
  const required = props.isRequired === true;
  const describedBy = props.description === undefined ? {} : { "aria-describedby": `${titleId}-description` };
  useKeyboardInset(ref);
  return (
    <ModalOverlay
      isOpen
      onOpenChange={props.onOpenChange}
      isDismissable={!required}
      isKeyboardDismissDisabled={required}
      className="paper-sheet-overlay"
    >
      <Modal ref={ref} className="paper-sheet" data-detent={props.detent}>
        <AriaDialog
          className="paper-sheet-dialog"
          aria-labelledby={titleId}
          {...describedBy}
          {...(props.role === undefined ? {} : { role: props.role })}
          ref={markModal}
        >
          {() => (
            <LayerScope dismiss={props.close} dismissable={!required}>
              <SheetParts {...props} titleId={titleId} sheet={ref} />
            </LayerScope>
          )}
        </AriaDialog>
      </Modal>
    </ModalOverlay>
  );
}

/** A non-modal sheet: a labelled region over the page, which stays usable. */
function FreeSheet(props: BottomSheetProps & Placed): ReactNode {
  const ref = useRef<HTMLElement>(null);
  const titleId = useId();
  useKeyboardInset(ref);
  return (
    <LayerScope dismiss={props.close} dismissable={props.isRequired !== true}>
      <FocusClear sheet={ref} detent={props.detent} />
      <section
        ref={ref}
        className="paper-sheet"
        aria-labelledby={titleId}
        {...(props.description === undefined ? {} : { "aria-describedby": `${titleId}-description` })}
        data-detent={props.detent}
      >
        <SheetParts {...props} titleId={titleId} sheet={ref} />
      </section>
    </LayerScope>
  );
}

/** The grabber, header, content and footer every sheet has. */
function SheetParts(
  props: BottomSheetProps & Placed & { titleId: string; sheet: RefObject<HTMLElement | null> },
): ReactNode {
  const { title, count, closeLabel, resizeLabel, detent, setDetent, close, children, footer, titleId, sheet } = props;
  const drag = useSheetDrag(sheet, detent, setDetent, close, resizeLabel !== undefined);
  return (
    <>
      <div className="paper-sheet-handle" {...drag.handlers}>
        {resizeLabel === undefined ? null : (
          <AriaButton
            className="paper-sheet-grabber"
            aria-label={resizeLabel}
            onPress={(event) => {
              // Only a pointer's press can be the release of a drag; a key press always resizes.
              if (event.pointerType !== "keyboard" && event.pointerType !== "virtual" && drag.consumePress()) return;
              setDetent(detent === "large" ? "medium" : "large");
            }}
          >
            <span className="paper-sheet-grabber-bar" />
          </AriaButton>
        )}
        <div className="paper-sheet-header" data-grabber={resizeLabel === undefined ? undefined : true}>
          <h2 id={titleId} className="paper-sheet-title">
            {title}
          </h2>
          {count === undefined ? null : <span className="paper-sheet-count">{count}</span>}
          {closeLabel === undefined || props.isRequired === true ? null : (
            <span className="paper-sheet-close">
              <IconButton label={closeLabel} icon="close" size={32} onPress={close} />
            </span>
          )}
        </div>
      </div>
      <div className="paper-sheet-body">
        {props.description === undefined ? null : (
          <p id={`${titleId}-description`} className="paper-sheet-description">
            {props.description}
          </p>
        )}
        {children}
      </div>
      {footer === undefined ? null : <div className="paper-sheet-footer">{footer}</div>}
    </>
  );
}

/** A drag in progress: where it started, and its latest position and velocity. */
interface Drag {
  id: number;
  y: number;
  lastY: number;
  lastT: number;
  velocity: number;
  moved: boolean;
}

/**
 * Where a drag released with the sheet's top at `top`, moving at `velocity`, leaves a sheet that
 * was at `detent`. A flick goes one detent in its direction (down from medium closes); a slow
 * release goes to the nearest detent, or closes when the sheet ends nearest the bottom.
 */
function settle(detent: SheetDetent, top: number, velocity: number): SheetDetent | "closed" {
  if (velocity > FLICK) return detent === "large" ? "medium" : "closed";
  if (velocity < -FLICK) return "large";
  const stops: [SheetDetent | "closed", number][] = [
    ["large", topOf("large")],
    ["medium", topOf("medium")],
    ["closed", window.innerHeight],
  ];
  return stops.reduce((a, b) => (Math.abs(b[1] - top) < Math.abs(a[1] - top) ? b : a))[0];
}

/** Follows a pointer move; false while the pointer is still within the press's slop. */
function track(drag: Drag, e: PointerEvent<HTMLElement>, sheet: HTMLElement | null): boolean {
  if (!drag.moved && Math.abs(e.clientY - drag.y) < SLOP) return false;
  if (!drag.moved) {
    drag.moved = true;
    sheet?.setAttribute("data-dragging", "");
    try {
      e.currentTarget.setPointerCapture(e.pointerId);
    } catch {
      // A pointer the browser no longer tracks can't be captured; the drag still follows it.
    }
  }
  const dt = e.timeStamp - drag.lastT;
  if (dt > 0) drag.velocity = (e.clientY - drag.lastY) / dt;
  drag.lastY = e.clientY;
  drag.lastT = e.timeStamp;
  return true;
}

/** Dragging the handle: the sheet follows the pointer, then snaps where `settle` says. */
function useSheetDrag(
  sheet: RefObject<HTMLElement | null>,
  detent: SheetDetent,
  setDetent: (detent: SheetDetent) => void,
  close: () => void,
  enabled: boolean,
) {
  const start = useRef<Drag | null>(null);
  const dragged = useRef(false);
  const move = (dy: number) => sheet.current?.style.setProperty("--sheet-drag", `${dy}px`);
  const end = (e: PointerEvent<HTMLElement>) => {
    const drag = start.current;
    // Another pointer's release leaves this drag going.
    if (drag === null || drag.id !== e.pointerId) return;
    start.current = null;
    if (!drag.moved) return;
    // Where the sheet is drawn now, keyboard inset and drag included.
    const top = sheet.current?.getBoundingClientRect().top ?? topOf(detent) + (e.clientY - drag.y);
    move(0);
    sheet.current?.removeAttribute("data-dragging");
    // The grabber's press that may follow this release is not a tap; any later press is.
    dragged.current = true;
    setTimeout(() => {
      dragged.current = false;
    }, 0);
    const velocity = e.timeStamp - drag.lastT < FLICK_HOLD ? drag.velocity : 0;
    const next = settle(detent, top, velocity);
    if (next === "closed") close();
    else if (next !== detent) setDetent(next);
  };
  return {
    /** Whether a drag just ended, so the grabber's press that follows it is not a tap. */
    consumePress: () => {
      const was = dragged.current;
      dragged.current = false;
      return was;
    },
    handlers: enabled
      ? {
          // In the capture phase: the grabber's press handling stops `pointerdown` from bubbling.
          onPointerDownCapture: (e: PointerEvent<HTMLElement>) => {
            // A second finger doesn't take over a drag in progress.
            if (e.button !== 0 || start.current?.moved === true) return;
            start.current = {
              id: e.pointerId,
              y: e.clientY,
              lastY: e.clientY,
              lastT: e.timeStamp,
              velocity: 0,
              moved: false,
            };
            dragged.current = false;
          },
          onPointerMove: (e: PointerEvent<HTMLElement>) => {
            const drag = start.current;
            if (drag === null || drag.id !== e.pointerId || !track(drag, e, sheet.current)) return;
            // The large detent is as high as the sheet goes.
            move(Math.max(e.clientY - drag.y, topOf("large") - topOf(detent)));
          },
          onPointerUp: end,
          onPointerCancel: end,
        }
      : {},
  };
}

/**
 * Lifts the sheet above an on-screen keyboard, which shrinks the visual viewport: sheet.css raises
 * its bottom to the keyboard's top and its top by as much, up to the large detent.
 */
function useKeyboardInset(sheet: RefObject<HTMLElement | null>): void {
  useEffect(() => {
    const viewport = window.visualViewport;
    const update = () => {
      const hidden = viewport === null ? 0 : Math.max(0, window.innerHeight - viewport.height - viewport.offsetTop);
      sheet.current?.style.setProperty("--sheet-keyboard", `${hidden}px`);
    };
    update();
    // The visual viewport also scrolls on its own while the keyboard is up, moving its offsetTop.
    viewport?.addEventListener("resize", update);
    viewport?.addEventListener("scroll", update);
    return () => {
      viewport?.removeEventListener("resize", update);
      viewport?.removeEventListener("scroll", update);
    };
  }, [sheet]);
}

/**
 * In a non-modal sheet: scrolls the page's focused element out from under the sheet, when it takes
 * focus and when the sheet moves to another detent, so the sheet never covers what has focus
 * (WCAG 2.4.11). Focus in a layer opened from the sheet, such as its menu, is left alone.
 */
function FocusClear({ sheet, detent }: { sheet: RefObject<HTMLElement | null>; detent: SheetDetent }): ReactNode {
  const menuOpen = useLayerOpenedHere();
  const latest = useRef(menuOpen);
  latest.current = menuOpen;
  useEffect(() => {
    const onFocus = (event: FocusEvent) => {
      if (event.target instanceof HTMLElement) clearLater(event.target, sheet, latest);
    };
    document.addEventListener("focusin", onFocus);
    return () => document.removeEventListener("focusin", onFocus);
  }, [sheet]);
  // biome-ignore lint/correctness/useExhaustiveDependencies: a new detent can cover the focused element, so it checks again.
  useEffect(() => {
    if (document.activeElement instanceof HTMLElement) clearLater(document.activeElement, sheet, latest);
  }, [sheet, detent]);
  return null;
}

/**
 * After the next frame (once layers have registered and the sheet has moved), scrolls `target`
 * clear of the sheet, by no more than keeps its top inside the viewport.
 */
function clearLater(
  target: HTMLElement,
  sheet: RefObject<HTMLElement | null>,
  menuOpen: RefObject<() => boolean>,
): void {
  requestAnimationFrame(() => {
    const panel = sheet.current;
    if (panel === null || panel.contains(target) || !target.isConnected || menuOpen.current()) return;
    const box = target.getBoundingClientRect();
    const covered = box.bottom - panel.getBoundingClientRect().top;
    if (covered <= 0) return;
    // A field taller than the space above the sheet keeps its top in view rather than its bottom.
    const by = Math.min(covered + SPACE["16"], box.top - SPACE["16"]);
    if (by > 0) scrollerOf(target).scrollBy({ top: by });
  });
}

/** The nearest ancestor of `element` that scrolls vertically, else the page. */
function scrollerOf(element: HTMLElement): Element {
  for (let node = element.parentElement; node !== null; node = node.parentElement) {
    const { overflowY } = getComputedStyle(node);
    if ((overflowY === "auto" || overflowY === "scroll") && node.scrollHeight > node.clientHeight) return node;
  }
  return document.scrollingElement ?? document.documentElement;
}
