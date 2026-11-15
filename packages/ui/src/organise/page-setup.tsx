// The page-setup dialogs (Paper O09): Crop pages (margins off each edge, what they leave, and the
// notice, always shown, that the cut edges stay in the file), Resize pages (a paper, an
// orientation, and whether the content scales or keeps its size at an anchor), and Page boxes
// (each of the five boxes as four numbers in points). Each Apply is one op the app applies, so
// each is one undo step.

import { type ReactNode, useEffect, useState } from "react";
import { Button } from "../components/button.js";
import { Checkbox, Radio, RadioGroup } from "../components/choice.js";
import { Select, TextInput, type TextInputProps } from "../components/field.js";
import { Notice } from "../components/notice.js";
import { laterCountMessage, laterMessage } from "../later-messages.js";
import { Dialog } from "../overlays/dialog.js";
import {
  type LengthUnit,
  type Margins,
  nudged,
  oriented,
  PAPERS,
  type PageSize,
  type PaperName,
  paperOf,
  points,
  shown,
  sizesOf,
  step,
  visibleArea,
} from "./geometry.js";

/** A key event on a text field, as React Aria gives it. */
type FieldKeyEvent = Parameters<NonNullable<TextInputProps["onKeyDown"]>>[0];

/** "216 × 279 mm". */
const sizeText = (size: PageSize, unit: LengthUnit) =>
  laterMessage("setup.size", { width: shown(size.width, unit), height: shown(size.height, unit), unit });

/** A length field whose arrow keys change it by a step (WCAG 2.5.7's alternative to dragging). */
function LengthField(props: {
  label: string;
  value: string;
  unit: LengthUnit;
  onChange: (value: string) => void;
  errorMessage?: string;
  autoFocus?: boolean;
}): ReactNode {
  const onKeyDown = (e: FieldKeyEvent) => {
    // React Aria stops a handled key from bubbling: let every other key go on (Escape to the dialog).
    if (e.key !== "ArrowUp" && e.key !== "ArrowDown") {
      e.continuePropagation();
      return;
    }
    e.preventDefault();
    const delta = step(props.unit, e.shiftKey) * (e.key === "ArrowUp" ? 1 : -1);
    props.onChange(nudged(props.value, delta, props.unit));
  };
  return (
    <TextInput
      label={props.label}
      value={props.value}
      onChange={props.onChange}
      onKeyDown={onKeyDown}
      inputMode="decimal"
      {...(props.errorMessage === undefined ? {} : { errorMessage: props.errorMessage })}
      {...(props.autoFocus === true ? { autoFocus: true } : {})}
    />
  );
}

/** Which pages an O09 dialog changes: the selection, or every page. */
export type ApplyTo = "selected" | "all";

/** Props of `CropDialog`. */
export interface CropDialogProps {
  readonly isOpen: boolean;
  readonly onClose: () => void;
  readonly unit: LengthUnit;
  /** Every page's displayed size, in order. */
  readonly pages: readonly PageSize[];
  /** The selected pages' indices (none: every page). */
  readonly selected: readonly number[];
  /** Crops the pages `to` names by `margins` in points of the page as it shows. */
  readonly onCrop: (margins: Margins, to: ApplyTo) => void;
}

const SIDES = ["top", "bottom", "left", "right"] as const;
const NO_MARGINS = { top: "0", bottom: "0", left: "0", right: "0" };

/** Crop pages: margins off each edge, the area they leave, and where the cut edges go (nowhere). */
export function CropDialog(props: CropDialogProps): ReactNode {
  const { unit, pages, selected, isOpen } = props;
  const [margins, setMargins] = useState<Record<(typeof SIDES)[number], string>>(NO_MARGINS);
  const [to, setTo] = useState<ApplyTo>("selected");
  useEffect(() => {
    if (isOpen) {
      setMargins(NO_MARGINS);
      setTo(selected.length === 0 ? "all" : "selected");
    }
  }, [isOpen, selected.length]);
  const targets =
    to === "all" || selected.length === 0 ? pages : selected.map((i) => pages[i]).filter((p) => p !== undefined);
  const read = Object.fromEntries(SIDES.map((s) => [s, points(margins[s], unit)])) as Record<
    (typeof SIDES)[number],
    number | undefined
  >;
  const valid = SIDES.every((s) => read[s] !== undefined);
  const value: Margins = { top: read.top ?? 0, right: read.right ?? 0, bottom: read.bottom ?? 0, left: read.left ?? 0 };
  const sizes = sizesOf(targets);
  const leavesNothing = valid && targets.some((page) => visibleArea(page, value) === undefined);
  const first = sizes[0];
  const left = first === undefined ? undefined : visibleArea(first, value);
  const count = targets.length;
  return (
    <Dialog
      isOpen={isOpen}
      onOpenChange={(open) => {
        if (!open) props.onClose();
      }}
      title={laterMessage("crop.title")}
      closeLabel={laterMessage("crop.close")}
      size="form"
      actions={
        <>
          <Button onPress={props.onClose}>{laterMessage("setup.cancel")}</Button>
          <Button variant="ghost" onPress={() => setMargins(NO_MARGINS)}>
            {laterMessage("crop.reset")}
          </Button>
          <Button
            variant="primary"
            isDisabled={!valid || leavesNothing || count === 0}
            onPress={() => {
              props.onCrop(value, to);
              props.onClose();
            }}
          >
            {laterCountMessage(count, { one: "crop.go.one", many: "crop.go" })}
          </Button>
        </>
      }
    >
      <fieldset className="paper-setup-margins">
        <legend className="paper-field-label">{laterMessage("crop.remove", { unit })}</legend>
        {SIDES.map((side, i) => (
          <LengthField
            key={side}
            label={laterMessage(`crop.${side}`)}
            unit={unit}
            value={margins[side]}
            onChange={(v) => setMargins((m) => ({ ...m, [side]: v }))}
            autoFocus={i === 0}
            {...(read[side] === undefined
              ? { errorMessage: laterMessage("setup.number") }
              : leavesNothing
                ? { errorMessage: laterMessage("crop.nothing") }
                : {})}
          />
        ))}
      </fieldset>
      <p className="paper-setup-line">
        {sizes.length > 1
          ? laterMessage("crop.visible.mixed")
          : first === undefined || left === undefined
            ? laterMessage("crop.nothing")
            : laterMessage("crop.visible", { area: sizeText(left, unit), page: sizeText(first, unit) })}
      </p>
      {selected.length === 0 ? null : (
        <RadioGroup label={laterMessage("setup.applyTo")} value={to} onChange={(v) => setTo(v as ApplyTo)}>
          <Radio value="selected">
            {laterCountMessage(selected.length, { one: "setup.selected.one", many: "setup.selected" })}
          </Radio>
          <Radio value="all">{laterCountMessage(pages.length, { one: "setup.all.one", many: "setup.all" })}</Radio>
        </RadioGroup>
      )}
      <Notice tone="warning" title={laterMessage("crop.notice.title")}>
        {laterMessage("crop.notice.body")}
      </Notice>
    </Dialog>
  );
}

/** How Resize treats the content: scaled to the new paper, or kept at its size at an anchor. */
export type ResizeContent = "scale" | "extend";

/** Where content kept at its size sits on the new paper. */
export const ANCHORS = [
  "topLeft",
  "top",
  "topRight",
  "left",
  "center",
  "right",
  "bottomLeft",
  "bottom",
  "bottomRight",
] as const;

/** One of them. */
export type ResizeAnchor = (typeof ANCHORS)[number];

/** Props of `ResizeDialog`. */
export interface ResizeDialogProps {
  readonly isOpen: boolean;
  readonly onClose: () => void;
  readonly unit: LengthUnit;
  /** The displayed sizes of the pages it resizes. */
  readonly pages: readonly PageSize[];
  /**
   * Resizes them to `size` as shown (the orientation chosen applied), scaling the content or
   * keeping its size at `anchor`.
   */
  readonly onResize: (size: PageSize, content: ResizeContent, anchor: ResizeAnchor) => void;
}

const PAPER_NAMES: readonly PaperName[] = ["a4", "letter", "legal", "a5", "custom"];

/** The preview's longest side, in CSS pixels. */
const PREVIEW = 116;

/** "24 pages · now US Letter, 216 × 279 mm", or "24 pages · 3 sizes". */
function resizeSubtitle(pages: readonly PageSize[], unit: LengthUnit): string {
  const now = pages[0] ?? PAPERS.letter;
  const count = pages.length;
  const sizes = sizesOf(pages).length;
  if (sizes > 1) return laterMessage("resize.subtitle.mixed", { count, sizes });
  const name = paperOf(now);
  return laterCountMessage(
    count,
    { one: "resize.subtitle.one", many: "resize.subtitle" },
    {
      paper: name === undefined ? laterMessage("resize.paper.custom") : laterMessage(`resize.paper.${name}`),
      size: sizeText(now, unit),
    },
  );
}

/** A custom size typed in `unit`, in points, or undefined when either side isn't a length above 0. */
function customSize(custom: { width: string; height: string }, unit: LengthUnit): PageSize | undefined {
  const width = points(custom.width, unit);
  const height = points(custom.height, unit);
  return width !== undefined && height !== undefined && width > 0 && height > 0 ? { width, height } : undefined;
}

/** Custom's width and height fields. */
function CustomSize(props: {
  unit: LengthUnit;
  custom: { width: string; height: string };
  onChange: (custom: { width: string; height: string }) => void;
}): ReactNode {
  const { unit, custom } = props;
  return (
    <div className="paper-setup-pair">
      <LengthField
        label={laterMessage("resize.width", { unit })}
        unit={unit}
        value={custom.width}
        onChange={(width) => props.onChange({ ...custom, width })}
        {...(customSize(custom, unit) === undefined ? { errorMessage: laterMessage("setup.number") } : {})}
      />
      <LengthField
        label={laterMessage("resize.height", { unit })}
        unit={unit}
        value={custom.height}
        onChange={(height) => props.onChange({ ...custom, height })}
      />
    </div>
  );
}

/** The page now (dashed) and the new size (solid) at one scale, and the anchor grid beside them. */
function ResizePreview(props: {
  now: PageSize;
  size: PageSize | undefined;
  unit: LengthUnit;
  anchor: ResizeAnchor;
  onAnchor: (anchor: ResizeAnchor) => void;
  scaled: boolean;
}): ReactNode {
  const { now, size, unit } = props;
  const scale = PREVIEW / Math.max(now.width, now.height, size?.width ?? 0, size?.height ?? 0);
  return (
    <div className="paper-setup-resize">
      <svg
        className="paper-setup-preview"
        width={PREVIEW + 4}
        height={PREVIEW + 4}
        role="img"
        aria-label={laterMessage("resize.preview", {
          from: sizeText(now, unit),
          to: size === undefined ? "?" : sizeText(size, unit),
        })}
      >
        <rect className="paper-setup-preview-now" x={2} y={2} width={now.width * scale} height={now.height * scale} />
        {size === undefined ? null : (
          <rect
            className="paper-setup-preview-new"
            x={2}
            y={2}
            width={size.width * scale}
            height={size.height * scale}
          />
        )}
      </svg>
      <RadioGroup
        label={laterMessage("resize.anchor")}
        value={props.anchor}
        onChange={(v) => props.onAnchor(v as ResizeAnchor)}
        isDisabled={props.scaled}
      >
        <span className="paper-setup-anchors">
          {ANCHORS.map((a) => (
            <Radio key={a} value={a}>
              <span className="paper-visually-hidden">{laterMessage(`resize.anchor.${a}`)}</span>
            </Radio>
          ))}
        </span>
      </RadioGroup>
    </div>
  );
}

/** Resize pages: a paper, an orientation, and how the content goes onto it. */
export function ResizeDialog(props: ResizeDialogProps): ReactNode {
  const { unit, pages, isOpen } = props;
  const now = pages[0] ?? PAPERS.letter;
  const [paper, setPaper] = useState<PaperName>("a4");
  const [orientation, setOrientation] = useState<"portrait" | "landscape">("portrait");
  const [content, setContent] = useState<ResizeContent>("scale");
  const [anchor, setAnchor] = useState<ResizeAnchor>("center");
  const [custom, setCustom] = useState({ width: "", height: "" });
  useEffect(() => {
    if (!isOpen) return;
    setPaper(paperOf(now) === "a4" ? "letter" : "a4");
    setOrientation(now.width > now.height ? "landscape" : "portrait");
    setContent("scale");
    setAnchor("center");
    setCustom({ width: shown(now.width, unit), height: shown(now.height, unit) });
  }, [isOpen, now, unit]);
  const size = paper === "custom" ? customSize(custom, unit) : oriented(PAPERS[paper], orientation);
  const count = pages.length;
  return (
    <Dialog
      isOpen={isOpen}
      onOpenChange={(open) => {
        if (!open) props.onClose();
      }}
      title={laterMessage("resize.title")}
      description={resizeSubtitle(pages, unit)}
      closeLabel={laterMessage("resize.close")}
      size="form"
      actions={
        <>
          <Button onPress={props.onClose}>{laterMessage("setup.cancel")}</Button>
          <Button
            variant="primary"
            isDisabled={size === undefined || count === 0}
            onPress={() => {
              if (size === undefined) return;
              props.onResize(size, content, anchor);
              props.onClose();
            }}
          >
            {laterCountMessage(count, { one: "resize.go.one", many: "resize.go" })}
          </Button>
        </>
      }
    >
      <Select
        label={laterMessage("resize.size")}
        value={paper}
        onChange={setPaper}
        options={PAPER_NAMES.map((p) => ({ value: p, label: laterMessage(`resize.paper.${p}`) }))}
      />
      {paper === "custom" ? (
        <CustomSize unit={unit} custom={custom} onChange={setCustom} />
      ) : (
        <RadioGroup
          label={laterMessage("resize.orientation")}
          value={orientation}
          onChange={(v) => setOrientation(v as "portrait" | "landscape")}
        >
          <Radio value="portrait">{laterMessage("resize.portrait")}</Radio>
          <Radio value="landscape">{laterMessage("resize.landscape")}</Radio>
        </RadioGroup>
      )}
      <RadioGroup
        label={laterMessage("resize.content")}
        value={content}
        onChange={(v) => setContent(v as ResizeContent)}
      >
        <Radio value="scale">{laterMessage("resize.scale")}</Radio>
        <Radio value="extend">{laterMessage("resize.extend")}</Radio>
      </RadioGroup>
      <ResizePreview
        now={now}
        size={size}
        unit={unit}
        anchor={anchor}
        onAnchor={setAnchor}
        scaled={content === "scale"}
      />
      <p className="paper-setup-line">{laterMessage("resize.note")}</p>
    </Dialog>
  );
}

/** A box as `[x0, y0, x1, y1]` in user space. */
export type Box = readonly [number, number, number, number];

/** The five boxes. */
export const BOX_NAMES = ["media", "crop", "bleed", "trim", "art"] as const;

/** One of them. */
export type BoxName = (typeof BOX_NAMES)[number];

/** A page's boxes as the boxes dialog reads them: each box, and which of the last three the page sets. */
export interface PageBoxValues {
  readonly media: Box;
  readonly crop: Box;
  readonly bleed: Box;
  readonly trim: Box;
  readonly art: Box;
  readonly own: { readonly bleed: boolean; readonly trim: boolean; readonly art: boolean };
}

/** The boxes to write: each a box, or `null` to remove it (the page then takes the crop box). */
export type BoxChanges = { readonly media: Box; readonly crop: Box } & {
  readonly [K in "bleed" | "trim" | "art"]: Box | null;
};

/** Props of `PageBoxesDialog`. */
export interface PageBoxesDialogProps {
  readonly isOpen: boolean;
  readonly onClose: () => void;
  /** The page whose boxes it shows ("3"), and how many pages it sets them on. */
  readonly page: string;
  readonly count: number;
  /** That page's boxes, or undefined while they are read. */
  readonly boxes: PageBoxValues | undefined;
  readonly onApply: (boxes: BoxChanges) => void;
}

const CORNERS = ["left", "bottom", "right", "top"] as const;
const FOLLOWS = ["bleed", "trim", "art"] as const;

type BoxText = readonly [string, string, string, string];

const boxText = (box: Box): BoxText => box.map((n) => String(Math.round(n * 100) / 100)) as unknown as BoxText;

/** The box typed, or undefined when it isn't four numbers with right above left and top above bottom. */
function readBox(text: BoxText): Box | undefined {
  const n = text.map((t) => (/^-?\d+(\.\d+)?$/.test(t.trim()) ? Number(t.trim()) : Number.NaN));
  const [x0, y0, x1, y1] = n as [number, number, number, number];
  return n.every(Number.isFinite) && x1 > x0 && y1 > y0 ? [x0, y0, x1, y1] : undefined;
}

const inside = (box: Box, outer: Box) =>
  box[0] >= outer[0] - 1e-6 && box[1] >= outer[1] - 1e-6 && box[2] <= outer[2] + 1e-6 && box[3] <= outer[3] + 1e-6;

/** Whether box `name` takes the crop box (the "Same as crop box" checkbox is on). */
const follows = (name: BoxName, same: Readonly<Record<(typeof FOLLOWS)[number], boolean>>) =>
  name === "bleed" || name === "trim" || name === "art" ? same[name] : false;

/** What is wrong with each box typed: not four ordered numbers, or outside the media box. */
function boxErrors(read: Record<BoxName, Box | undefined>): Partial<Record<BoxName, string>> {
  const out: Partial<Record<BoxName, string>> = {};
  for (const name of BOX_NAMES) {
    const box = read[name];
    if (box === undefined) out[name] = laterMessage("boxes.invalid");
    else if (name !== "media" && read.media !== undefined && !inside(box, read.media))
      out[name] = laterMessage("boxes.outside");
  }
  return out;
}

/** One box's four numbers, and for the bleed, trim and art boxes, "Same as crop box". */
function BoxFieldset(props: {
  name: BoxName;
  text: BoxText;
  same: boolean | undefined;
  error: string | undefined;
  autoFocus: boolean;
  onText: (corner: number, value: string) => void;
  onSame: (same: boolean) => void;
}): ReactNode {
  const { name, error } = props;
  const box = laterMessage(`boxes.${name}`);
  return (
    <fieldset className="paper-setup-box" data-invalid={error === undefined ? undefined : true}>
      <legend className="paper-field-label">{box}</legend>
      {props.same === undefined ? null : (
        <Checkbox isSelected={props.same} onChange={props.onSame}>
          {laterMessage("boxes.same")}
        </Checkbox>
      )}
      {props.same === true ? null : (
        <div className="paper-setup-corners">
          {CORNERS.map((corner, c) => (
            <TextInput
              key={corner}
              label={laterMessage(`boxes.${corner}`)}
              aria-label={laterMessage("boxes.field", { box, side: laterMessage(`boxes.${corner}`).toLowerCase() })}
              value={props.text[c] ?? ""}
              inputMode="decimal"
              {...(props.autoFocus && c === 0 ? { autoFocus: true } : {})}
              onChange={(v) => props.onText(c, v)}
            />
          ))}
        </div>
      )}
      {error === undefined || props.same === true ? null : <p className="paper-field-error">{error}</p>}
    </fieldset>
  );
}

/** Page boxes: the media, crop, bleed, trim and art boxes as numbers, each of the last three possibly the crop box. */
export function PageBoxesDialog(props: PageBoxesDialogProps): ReactNode {
  const { boxes, isOpen } = props;
  const [text, setText] = useState<Record<BoxName, BoxText> | undefined>(undefined);
  const [same, setSame] = useState({ bleed: true, trim: true, art: true });
  useEffect(() => {
    if (!isOpen || boxes === undefined) return;
    setText(Object.fromEntries(BOX_NAMES.map((b) => [b, boxText(boxes[b])])) as Record<BoxName, BoxText>);
    setSame({ bleed: !boxes.own.bleed, trim: !boxes.own.trim, art: !boxes.own.art });
  }, [isOpen, boxes]);
  const read =
    text === undefined
      ? undefined
      : (Object.fromEntries(BOX_NAMES.map((b) => [b, readBox(text[b])])) as Record<BoxName, Box | undefined>);
  const errors = read === undefined ? {} : boxErrors(read);
  const valid = read !== undefined && BOX_NAMES.every((b) => follows(b, same) || errors[b] === undefined);
  const apply = () => {
    if (read?.media === undefined || read.crop === undefined) return;
    const pick = (b: (typeof FOLLOWS)[number]) => (same[b] ? null : (read[b] ?? null));
    props.onApply({ media: read.media, crop: read.crop, bleed: pick("bleed"), trim: pick("trim"), art: pick("art") });
    props.onClose();
  };
  const onText = (name: BoxName) => (c: number, v: string) =>
    setText((t) => {
      if (t === undefined) return t;
      const next = [...t[name]] as [string, string, string, string];
      next[c] = v;
      return { ...t, [name]: next };
    });
  return (
    <Dialog
      isOpen={isOpen}
      onOpenChange={(open) => {
        if (!open) props.onClose();
      }}
      title={laterMessage("boxes.title")}
      description={laterCountMessage(
        props.count,
        { one: "boxes.subtitle.one", many: "boxes.subtitle" },
        { page: props.page },
      )}
      closeLabel={laterMessage("boxes.close")}
      size="wide"
      actions={
        <>
          <Button onPress={props.onClose}>{laterMessage("setup.cancel")}</Button>
          <Button variant="primary" isDisabled={!valid} onPress={apply}>
            {laterMessage("boxes.apply")}
          </Button>
        </>
      }
    >
      <p className="paper-setup-line">{laterMessage("boxes.intro")}</p>
      {text === undefined
        ? null
        : BOX_NAMES.map((name, b) => (
            <BoxFieldset
              key={name}
              name={name}
              text={text[name]}
              same={name === "bleed" || name === "trim" || name === "art" ? same[name] : undefined}
              error={errors[name]}
              autoFocus={b === 0}
              onText={onText(name)}
              onSame={(on) => setSame((s) => ({ ...s, [name]: on }))}
            />
          ))}
    </Dialog>
  );
}
