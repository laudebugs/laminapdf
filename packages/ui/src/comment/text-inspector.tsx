// The text inspector (Paper C03): a text box's, callout's or typewriter text's own fields above
// the annotation inspector's shared ones. Font, size, bold and italic, alignment, the border's
// width and a callout's line end; the note that bold and italic are kept as rich text, a notice
// when characters use the fallback font or when the box held styles Lamina PDF doesn't keep, and
// Done editing while the box is typed into. Each field changes the box being typed into, or the
// selected box as one edit.

import type { ReactNode } from "react";
import { Button } from "../components/button.js";
import { Select } from "../components/field.js";
import { Notice } from "../components/notice.js";
import { chooseOnPress, PressedGroup } from "../components/pressed-group.js";
import { ToggleButton } from "../components/toggle.js";
import { type LaterMessageId, laterMessage } from "../later-messages.js";
import {
  LINE_ENDINGS,
  LINE_WIDTHS,
  type LineEndingName,
  TEXT_ALIGNMENTS,
  TEXT_FONTS,
  TEXT_SIZES,
  type TextAlignment,
} from "./commands.js";
import type { InspectorChoice } from "./inspector.js";

/** Props of `TextFields`. */
export interface TextFieldsProps {
  /** `helvetica`, `times`, `courier`, or another font the file names (shown as "As in the file"). */
  readonly font: InspectorChoice<string>;
  /** Points. */
  readonly size: InspectorChoice<number>;
  readonly bold: InspectorChoice<boolean>;
  readonly italic: InspectorChoice<boolean>;
  readonly align: InspectorChoice<TextAlignment>;
  /** The border's width in points, 0 for none; absent for typewriter text, which has none. */
  readonly border?: InspectorChoice<number>;
  /** A callout's line end. */
  readonly end?: InspectorChoice<LineEndingName>;
  /** Some characters are drawn with the fallback font. */
  readonly fallback: boolean;
  /** The box held styles outside the subset, which a change drops. */
  readonly simplified: boolean;
  /** Ends typing into the box; absent while it isn't typed into. */
  readonly onDone?: () => void;
  readonly disabled: boolean;
}

/** The alignment toggles' icons. */
const ALIGN_ICONS = { left: "align-left", center: "align-centre", right: "align-right" } as const;

/** The font select: the standard fonts, and "As in the file" for any other. */
function FontField({ font, disabled }: { font: InspectorChoice<string>; disabled: boolean }): ReactNode {
  const standard = (TEXT_FONTS as readonly string[]).includes(font.value ?? "");
  const options = [
    ...(standard ? [] : [{ value: "other", label: laterMessage("inspector.font.other") }]),
    ...TEXT_FONTS.map((f) => ({ value: f, label: laterMessage(`inspector.font.${f}`) })),
  ];
  return (
    <Select
      label={laterMessage("inspector.font")}
      options={options}
      value={standard ? (font.value as string) : "other"}
      isDisabled={disabled}
      onChange={(value) => {
        if (value !== "other") font.onChange(value);
      }}
    />
  );
}

/** The size select: the usual sizes, and the box's own when it is another. */
function SizeField({ size, disabled }: { size: InspectorChoice<number>; disabled: boolean }): ReactNode {
  const now = size.value ?? 12;
  const sizes = [...new Set<number>([...TEXT_SIZES, now])].sort((a, b) => a - b);
  return (
    <Select
      label={laterMessage("inspector.size")}
      options={sizes.map((s) => ({ value: String(s), label: laterMessage("comment.width.value", { width: s }) }))}
      value={String(now)}
      isDisabled={disabled}
      onChange={(value) => size.onChange(Number(value))}
    />
  );
}

/** A labelled row of toggle buttons. */
function ToggleRow({ label, children }: { label: string; children: ReactNode }): ReactNode {
  return (
    <div className="paper-field">
      <span className="paper-field-label" aria-hidden="true">
        {label}
      </span>
      <PressedGroup label={label} className="paper-comment-text-toggles">
        {children}
      </PressedGroup>
    </div>
  );
}

/** A select over `values`, each named by the later catalogue's `prefix` + value. */
function ChoiceField<Value extends string>(props: {
  label: string;
  values: readonly Value[];
  prefix: string;
  choice: InspectorChoice<Value>;
  disabled: boolean;
}): ReactNode {
  const { label, values, prefix, choice, disabled } = props;
  return (
    <Select
      label={label}
      options={values.map((v) => ({ value: v, label: laterMessage(`${prefix}${v}` as LaterMessageId) }))}
      value={choice.value ?? (values[0] as Value)}
      isDisabled={disabled}
      onChange={choice.onChange}
    />
  );
}

/** The border width select: None, then the line widths. */
function BorderWidthField({ border, disabled }: { border: InspectorChoice<number>; disabled: boolean }): ReactNode {
  const widths = [0, ...LINE_WIDTHS];
  return (
    <Select
      label={laterMessage("inspector.borderWidth")}
      options={widths.map((w) => ({
        value: String(w),
        label: w === 0 ? laterMessage("inspector.borderWidth.none") : laterMessage("comment.width.value", { width: w }),
      }))}
      value={String(border.value ?? 1)}
      isDisabled={disabled}
      onChange={(value) => border.onChange(Number(value))}
    />
  );
}

/** A text box's own fields. */
export function TextFields(props: TextFieldsProps): ReactNode {
  const { font, size, bold, italic, align, border, end, disabled } = props;
  return (
    <div className="paper-comment-text">
      <FontField font={font} disabled={disabled} />
      <SizeField size={size} disabled={disabled} />
      <ToggleRow label={laterMessage("inspector.style")}>
        <ToggleButton isSelected={bold.value === true} isDisabled={disabled} onChange={bold.onChange}>
          {laterMessage("inspector.bold")}
        </ToggleButton>
        <ToggleButton isSelected={italic.value === true} isDisabled={disabled} onChange={italic.onChange}>
          {laterMessage("inspector.italic")}
        </ToggleButton>
      </ToggleRow>
      <ToggleRow label={laterMessage("inspector.alignment")}>
        {TEXT_ALIGNMENTS.map((a) => (
          <ToggleButton
            key={a}
            icon={ALIGN_ICONS[a]}
            aria-label={laterMessage(`inspector.align.${a}`)}
            isSelected={align.value === a}
            isDisabled={disabled}
            onChange={chooseOnPress(a, align.onChange)}
          />
        ))}
      </ToggleRow>
      {border === undefined ? null : <BorderWidthField border={border} disabled={disabled} />}
      {end === undefined ? null : (
        <ChoiceField
          label={laterMessage("inspector.lineEnd")}
          values={LINE_ENDINGS}
          prefix="inspector.ending."
          choice={end}
          disabled={disabled}
        />
      )}
      <p className="paper-comment-text-note">{laterMessage("inspector.richText")}</p>
      {props.fallback ? <Notice tone="warning" title={laterMessage("inspector.fallback")} /> : null}
      {props.simplified ? <Notice tone="warning" title={laterMessage("inspector.simplified")} /> : null}
      {props.onDone === undefined ? null : (
        <Button size={30} onPress={props.onDone}>
          {laterMessage("inspector.doneEditing")}
        </Button>
      )}
    </div>
  );
}
