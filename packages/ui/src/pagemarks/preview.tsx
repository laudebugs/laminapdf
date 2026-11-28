// The Page marks dialog's preview column (Paper A03): the page's rendered picture with the mark
// drawn over it from the form's settings, a stepper to look at other pages, and the note that the
// saved mark may differ a little. The picture is `role="img"`, described by the note; the stepper's
// "Page 3 of 24" is a polite live region. Nothing is applied until the dialog's primary button.

import { type ReactNode, useEffect, useId, useRef } from "react";
import { IconButton } from "../components/button.js";
import { laterMessage } from "../later-messages.js";
import type { PageSize } from "./model.js";

/** The preview's width, in CSS pixels. */
const WIDTH = 280;

/** What the preview shows. */
export type PreviewPicture = CanvasImageSource | "reading" | "failed";

/** Props of `MarkPreview`. */
export interface MarkPreviewProps {
  /** The page shown, in points. */
  readonly size: PageSize;
  readonly picture: PreviewPicture;
  /** Page `label` (its place `index + 1` of `count`). */
  readonly label: string;
  readonly index: number;
  readonly count: number;
  readonly onStep: (delta: -1 | 1) => void;
  /** Draws the mark; `ctx`'s unit is one point of the page, and `pixel` is one CSS pixel in points. */
  readonly draw: (ctx: CanvasRenderingContext2D, pixel: number) => void;
  /** What the mark is, for the picture's name: "Watermark DRAFT on page 3". */
  readonly name: string;
}

/** The page's picture with the mark over it, and the stepper under it. */
export function MarkPreview({ size, picture, label, index, count, onStep, draw, name }: MarkPreviewProps): ReactNode {
  const note = useId();
  const canvas = useRef<HTMLCanvasElement>(null);
  const height = Math.round((WIDTH * size.height) / Math.max(size.width, 1));
  // Drawn after every render: the form's settings change what goes over the picture.
  useEffect(() => {
    const element = canvas.current;
    const ctx = element?.getContext("2d");
    if (element === null || ctx === null || ctx === undefined) return;
    const ratio = globalThis.devicePixelRatio || 1;
    element.width = Math.round(WIDTH * ratio);
    element.height = Math.round(height * ratio);
    ctx.fillStyle = "#fff";
    ctx.fillRect(0, 0, element.width, element.height);
    if (picture !== "reading" && picture !== "failed") {
      ctx.drawImage(picture, 0, 0, element.width, element.height);
    }
    const scale = (WIDTH * ratio) / Math.max(size.width, 1);
    ctx.save();
    ctx.scale(scale, scale);
    draw(ctx, size.width / WIDTH);
    ctx.restore();
  });
  return (
    <div className="paper-marks-preview">
      <div
        className="paper-marks-page"
        role="img"
        aria-label={name}
        aria-describedby={note}
        style={{ inlineSize: WIDTH, blockSize: height }}
      >
        <canvas ref={canvas} style={{ inlineSize: WIDTH, blockSize: height }} />
        {picture === "failed" ? (
          <p className="paper-marks-page-note">{laterMessage("pagemarks.preview.failed")}</p>
        ) : null}
      </div>
      <div className="paper-marks-stepper">
        <IconButton
          icon="chevron-left"
          label={laterMessage("pagemarks.preview.previous")}
          size={28}
          isDisabled={index <= 0}
          onPress={() => onStep(-1)}
        />
        <span role="status" className="paper-marks-stepper-label">
          {laterMessage("pagemarks.preview.page", { label, position: index + 1, count })}
        </span>
        <IconButton
          icon="chevron-right"
          label={laterMessage("pagemarks.preview.next")}
          size={28}
          isDisabled={index >= count - 1}
          onPress={() => onStep(1)}
        />
      </div>
      <p id={note} className="paper-marks-preview-note">
        {laterMessage("pagemarks.preview.note")}
      </p>
    </div>
  );
}
