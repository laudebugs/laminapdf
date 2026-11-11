// The link preview (§7.27, R07): where an internal link goes, before following it. The surface
// follows the pointer that asked (ADR 0024 §4). With a mouse, a pen or the keyboard it is a tooltip
// under the link (above when there is no room), which the link names with `aria-describedby`: it
// can be hovered (WCAG 1.4.13), Escape closes it through the layer stack with focus left on the
// link, and Enter follows the link. With touch it is a bottom sheet at the medium detent with
// "Go to page 11" and Close, never a floating card. What it shows (the picture, the title, the
// words) is the app's.

import { SPACE } from "@laminapdf/tokens";
import { type ReactNode, type RefObject, useEffect, useId, useLayoutEffect, useRef } from "react";
import { createPortal } from "react-dom";
import { Button } from "../components/button.js";
import { useAnchoredPlacement } from "./anchored.js";
import { LayerScope } from "./layers.js";
import { placeBelow } from "./place.js";
import { BottomSheet } from "./sheet.js";

/** A picture of the destination: a bitmap and its size on screen in CSS px. */
export interface LinkPreviewImage {
  readonly bitmap: ImageBitmap;
  readonly width: number;
  readonly height: number;
}

/** Props of `LinkPreview`. */
export interface LinkPreviewProps {
  /** The link: the preview describes it, and is placed against it. */
  anchor: HTMLElement;
  /** How the preview was asked for: a popover for `fine` and `keyboard`, a sheet for `touch`. */
  pointer: "fine" | "keyboard" | "touch";
  /** Where the link goes: the destination's section, else its page label. */
  title: string;
  /** The line under the title: "Link to page 11 · Enter to go"; left out, there is none. */
  caption?: string;
  /** The destination's picture; `undefined` while it draws, `null` for a link that has none. */
  picture: LinkPreviewImage | null | undefined;
  /** The picture's name: the text it shows, or "Top of page 11"; the title when left out. */
  pictureLabel?: string;
  /** The sheet's text under the picture (the destination's text). */
  text?: string;
  /** The sheet's button that follows the link: "Go to page 11". Left out, the sheet has none. */
  goLabel?: string;
  /** The sheet's close button: "Close". */
  closeLabel: string;
  /** Follows the link. */
  onFollow: () => void;
  /** Escape, Close or a dismissed sheet: the app hides the preview. */
  onDismiss: () => void;
  /** The pointer came onto the popover (`true`) or left it: a hovered preview stays. */
  onHold: (held: boolean) => void;
}

/** The gap between the link and the popover, and between the popover and the viewport's edge. */
const GAP = SPACE["6"];

/** `anchor`'s box in the viewport, or null once it has left the page. */
function boxOf(anchor: HTMLElement): { top: number; left: number; width: number; height: number } | null {
  if (!anchor.isConnected) return null;
  const r = anchor.getBoundingClientRect();
  return { top: r.top, left: r.left, width: r.width, height: r.height };
}

/** A preview of a link's destination. Render it while the preview shows. */
export function LinkPreview(props: LinkPreviewProps): ReactNode {
  return props.pointer === "touch" ? <PreviewSheet {...props} /> : <PreviewPopover {...props} />;
}

function PreviewPopover({
  anchor,
  title,
  caption,
  picture,
  pictureLabel,
  onDismiss,
  onHold,
}: LinkPreviewProps): ReactNode {
  const id = useId();
  const ref = useRef<HTMLDivElement>(null);
  const place = useAnchoredPlacement(ref, { anchor: () => boxOf(anchor), place: placeBelow, gap: GAP });
  useDescribedBy(anchor, id);
  useHover(ref, onHold);
  return createPortal(
    <LayerScope dismiss={onDismiss}>
      <div
        ref={ref}
        id={id}
        role="tooltip"
        className="paper-link-preview"
        data-side={place?.side}
        style={place === null ? { visibility: "hidden" } : { top: place.top, left: place.left }}
      >
        <Picture picture={picture} label={pictureLabel ?? title} />
        <div className="paper-link-preview-title">{title}</div>
        {caption === undefined ? null : <div className="paper-link-preview-caption">{caption}</div>}
      </div>
    </LayerScope>,
    document.body,
  );
}

function PreviewSheet({
  title,
  caption,
  picture,
  pictureLabel,
  text,
  goLabel,
  closeLabel,
  onFollow,
  onDismiss,
}: LinkPreviewProps): ReactNode {
  return (
    <BottomSheet
      isOpen
      onOpenChange={(open) => {
        if (!open) onDismiss();
      }}
      title={title}
      {...(caption === undefined ? {} : { description: caption })}
      closeLabel={closeLabel}
      defaultDetent="medium"
      footer={
        goLabel === undefined ? undefined : (
          <Button variant="primary" onPress={onFollow}>
            {goLabel}
          </Button>
        )
      }
    >
      <div className="paper-link-preview-sheet">
        <Picture picture={picture} label={pictureLabel ?? title} />
        {text === undefined || text === "" ? null : <p className="paper-link-preview-text">{text}</p>}
      </div>
    </BottomSheet>
  );
}

/** The destination's picture on a canvas, drawn at the device's pixels; a placeholder while it draws. */
function Picture({
  picture,
  label,
}: {
  picture: LinkPreviewImage | null | undefined;
  label?: string | undefined;
}): ReactNode {
  const ref = useRef<HTMLCanvasElement>(null);
  useLayoutEffect(() => {
    const canvas = ref.current;
    if (canvas === null || picture === null || picture === undefined) return;
    canvas.width = picture.bitmap.width;
    canvas.height = picture.bitmap.height;
    const context = canvas.getContext("2d");
    if (context === null) return;
    try {
      context.drawImage(picture.bitmap, 0, 0);
    } catch {
      // The viewer closed the bitmap (a zoom change) before this drew it: the next picture replaces it.
    }
  }, [picture]);
  if (picture === null) return null;
  return picture === undefined ? (
    <div className="paper-link-preview-picture" data-loading aria-hidden="true" />
  ) : (
    <canvas
      ref={ref}
      role="img"
      aria-label={label}
      className="paper-link-preview-picture"
      style={{ width: picture.width, height: picture.height }}
    />
  );
}

/**
 * Calls `onChange` with `true` when the pointer comes onto `ref`'s element and `false` when it
 * leaves, from the browser's own pointerenter and pointerleave. React's `onPointerEnter` makes them
 * up from pointerout and pointerover instead, and reports an enter twice when the pointer comes from
 * a node outside React's roots, such as the page's body: once for that node's pointerout, once for
 * the element's pointerover (docs/bugs/link-preview-held-twice).
 */
function useHover(ref: RefObject<HTMLElement | null>, onChange: (hovered: boolean) => void): void {
  useEffect(() => {
    const element = ref.current;
    if (element === null) return;
    const enter = () => onChange(true);
    const leave = () => onChange(false);
    element.addEventListener("pointerenter", enter);
    element.addEventListener("pointerleave", leave);
    return () => {
      element.removeEventListener("pointerenter", enter);
      element.removeEventListener("pointerleave", leave);
    };
  }, [ref, onChange]);
}

/** Names the popover from the link while it shows (`aria-describedby`), leaving any other description. */
function useDescribedBy(anchor: HTMLElement, id: string): void {
  useEffect(() => {
    const before = anchor.getAttribute("aria-describedby");
    anchor.setAttribute("aria-describedby", before === null || before === "" ? id : `${before} ${id}`);
    return () => {
      if (before === null) anchor.removeAttribute("aria-describedby");
      else anchor.setAttribute("aria-describedby", before);
    };
  }, [anchor, id]);
}
