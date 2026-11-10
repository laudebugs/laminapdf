// The link preview (§7.27, R07): where an internal link goes, before following it. The surface
// follows the pointer that asked (ADR 0024 §4). With a mouse, a pen or the keyboard it is a tooltip
// under the link (above when there is no room), which the link names with `aria-describedby`: it
// can be hovered (WCAG 1.4.13), Escape closes it through the layer stack with focus left on the
// link, and Enter follows the link. With touch it is a bottom sheet at the medium detent with
// "Go to page 11" and Close, never a floating card. What it shows (the picture, the title, the
// words) is the app's.

import { SPACE } from "@laminapdf/tokens";
import { type ReactNode, useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { Button } from "../components/button.js";
import { LayerScope } from "./layers.js";
import { type Placement, placeBelow } from "./place.js";
import { BottomSheet } from "./sheet.js";

/** A picture of the destination: a bitmap and its size on screen in CSS px. */
export interface LinkPreviewPicture {
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
  picture: LinkPreviewPicture | null | undefined;
  /** The picture's name: the text it shows, or "Top of page 11". */
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
const MARGIN = SPACE["8"];

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
  const place = usePlacement(ref, anchor);
  useDescribedBy(anchor, id);
  return createPortal(
    <LayerScope dismiss={onDismiss}>
      <div
        ref={ref}
        id={id}
        role="tooltip"
        className="paper-link-preview"
        data-side={place?.side}
        style={place === null ? { visibility: "hidden" } : { top: place.top, left: place.left }}
        onPointerEnter={() => onHold(true)}
        onPointerLeave={() => onHold(false)}
      >
        <Picture picture={picture} label={pictureLabel} />
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
        <Picture picture={picture} label={pictureLabel} />
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
  picture: LinkPreviewPicture | null | undefined;
  label?: string | undefined;
}): ReactNode {
  const ref = useRef<HTMLCanvasElement>(null);
  useLayoutEffect(() => {
    const canvas = ref.current;
    if (canvas === null || picture === null || picture === undefined) return;
    canvas.width = picture.bitmap.width;
    canvas.height = picture.bitmap.height;
    canvas.getContext("2d")?.drawImage(picture.bitmap, 0, 0);
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

/** Keeps the popover under its link (above when it doesn't fit), on every scroll, resize and render. */
function usePlacement(layer: React.RefObject<HTMLElement | null>, anchor: HTMLElement): Placement | null {
  // The compiler can't follow the latest-value ref kept here for effects.
  "use no memo";
  const [place, setPlace] = useState<Placement | null>(null);
  const measure = () => {
    const element = layer.current;
    if (element === null || !anchor.isConnected) return setPlace(null);
    const r = anchor.getBoundingClientRect();
    const viewport = { top: 0, left: 0, width: window.innerWidth, height: window.innerHeight };
    const next = placeBelow(
      { top: r.top, left: r.left, width: r.width, height: r.height },
      element.offsetWidth,
      element.offsetHeight,
      viewport,
      GAP,
      MARGIN,
    );
    setPlace((last) =>
      last !== null && next !== null && last.top === next.top && last.left === next.left && last.side === next.side
        ? last
        : next,
    );
  };
  const latest = useRef(measure);
  latest.current = measure;
  useLayoutEffect(() => {
    latest.current();
  });
  useLayoutEffect(() => {
    let frame = 0;
    const later = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => latest.current());
    };
    const size = new ResizeObserver(later);
    if (layer.current !== null) size.observe(layer.current);
    document.addEventListener("scroll", later, true);
    window.addEventListener("resize", later);
    return () => {
      cancelAnimationFrame(frame);
      size.disconnect();
      document.removeEventListener("scroll", later, true);
      window.removeEventListener("resize", later);
    };
  }, [layer]);
  return place;
}
