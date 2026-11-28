// The Watermark tab (Paper A03): text or a picture, its font and size, colour, opacity and angle,
// the layer (in front of the content, or behind it, which makes it a background), and whether a
// reader can hide it.

import type { TokenName } from "@laminapdf/tokens";
import type { ReactNode } from "react";
import { Button } from "../components/button.js";
import { Checkbox, Radio, RadioGroup } from "../components/choice.js";
import { TextInput } from "../components/field.js";
import { SwatchGroup } from "../components/swatches.js";
import { CONTENT_COLOURS } from "../content/commands.js";
import { laterMessage } from "../later-messages.js";
import { LookRow, type MarkFont, NumberField } from "./fields.js";
import { WATERMARK_COLOURS, type WatermarkValues } from "./model.js";
import type { MarkImage } from "./preview-draw.js";

/** Props of `WatermarkTab`. */
export interface WatermarkTabProps {
  readonly values: WatermarkValues;
  readonly onChange: (change: Partial<WatermarkValues>) => void;
  readonly fonts: readonly MarkFont[];
  readonly missing: readonly string[];
  /** False on a PDF/A-1 file, which can't hold the layer a hideable mark needs. */
  readonly canHide: boolean;
  readonly image: MarkImage | undefined;
  readonly onPickImage: () => void;
  /** Whether the mark is being edited, so its layer is fixed (a watermark stays one). */
  readonly editing: boolean;
  /** An edited picture the document no longer has the file of: it has to be chosen again. */
  readonly imageLost: boolean;
}

/** The colours as swatches, in the design's order. */
const SWATCHES = WATERMARK_COLOURS.map((id) => ({
  id,
  label: laterMessage(`content.colour.${id}`),
  color: (CONTENT_COLOURS.find((c) => c.id === id) as (typeof CONTENT_COLOURS)[number]).token as TokenName,
}));

/** The Watermark tab's form. */
export function WatermarkTab(props: WatermarkTabProps): ReactNode {
  const { values, onChange } = props;
  return (
    <div className="paper-marks-fields">
      <RadioGroup
        label={laterMessage("pagemarks.use")}
        orientation="horizontal"
        value={values.source}
        onChange={(source) => onChange({ source: source as WatermarkValues["source"] })}
      >
        <Radio value="text">{laterMessage("pagemarks.use.text")}</Radio>
        <Radio value="image">{laterMessage("pagemarks.use.image")}</Radio>
      </RadioGroup>
      {values.source === "text" ? <TextFields {...props} /> : <ImageFields {...props} />}
      <div className="paper-marks-row">
        <NumberField
          label={laterMessage("pagemarks.opacity")}
          value={values.opacity}
          min={0}
          max={100}
          onChange={(opacity) => onChange({ opacity })}
        />
        <NumberField
          label={laterMessage("pagemarks.angle")}
          value={values.angle}
          min={-360}
          max={360}
          onChange={(angle) => onChange({ angle })}
        />
      </div>
      <RadioGroup
        label={laterMessage("pagemarks.layer")}
        value={values.layer}
        isDisabled={props.editing}
        onChange={(layer) => onChange({ layer: layer as WatermarkValues["layer"] })}
      >
        <Radio value="front">{laterMessage("pagemarks.layer.front")}</Radio>
        <Radio value="behind">{laterMessage("pagemarks.layer.behind")}</Radio>
      </RadioGroup>
      {props.canHide ? (
        <Checkbox isSelected={values.hideable} onChange={(hideable) => onChange({ hideable })}>
          {laterMessage("pagemarks.hideable")}
        </Checkbox>
      ) : null}
    </div>
  );
}

/** The text, its font and size, and its colour. */
function TextFields({ values, onChange, fonts, missing }: WatermarkTabProps): ReactNode {
  return (
    <>
      <TextInput
        label={laterMessage("pagemarks.text")}
        value={values.text}
        onChange={(text) => onChange({ text })}
        autoFocus
        {...(values.text.trim() === "" ? { description: laterMessage("pagemarks.text.empty") } : {})}
      />
      <LookRow fonts={fonts} missing={missing} values={values} onChange={onChange} />
      <SwatchGroup
        label={laterMessage("pagemarks.colour")}
        swatches={SWATCHES}
        value={values.colour}
        size={28}
        onChange={(colour) => onChange({ colour })}
      />
    </>
  );
}

/** The picture: its name, and the button that chooses one. */
function ImageFields({ image, imageLost, onPickImage }: WatermarkTabProps): ReactNode {
  return (
    <div className="paper-marks-picture">
      <Button onPress={onPickImage}>
        {laterMessage(image === undefined ? "pagemarks.image.choose" : "pagemarks.image.change")}
      </Button>
      <p className="paper-marks-picture-name">
        {image === undefined
          ? laterMessage(imageLost ? "pagemarks.image.lost" : "pagemarks.image.none")
          : laterMessage("pagemarks.image.name", { file: image.name })}
      </p>
    </div>
  );
}
