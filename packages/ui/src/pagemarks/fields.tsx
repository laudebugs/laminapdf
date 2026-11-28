// The fields the Page marks dialog's tabs share: a number that tells when it is out of range, the
// font select, and the pages (all, or a range of numbers and labels).

import { type ReactNode, useState } from "react";
import { Radio, RadioGroup } from "../components/choice.js";
import { Select, TextInput } from "../components/field.js";
import { laterCountMessage, laterMessage } from "../later-messages.js";
import { type PagesChoice, readNumber } from "./model.js";

/** A font the selects offer: its id, and its name with what fetching it costs. */
export interface MarkFont {
  readonly id: string;
  readonly label: string;
}

/** Props of `NumberField`. */
export interface NumberFieldProps {
  readonly label: string;
  readonly value: number;
  readonly min: number;
  readonly max: number;
  /** The number typed, or `NaN` while the text isn't a number in range. */
  readonly onChange: (value: number) => void;
  readonly description?: string;
}

/** A field of one number: shown as typed, in danger with the range while it isn't one in range. */
export function NumberField({ label, value, min, max, onChange, description }: NumberFieldProps): ReactNode {
  const [text, setText] = useState(Number.isFinite(value) ? String(value) : "");
  const invalid = readNumber(text, min, max) === undefined;
  return (
    <TextInput
      label={label}
      value={text}
      inputMode="decimal"
      onChange={(next) => {
        setText(next);
        onChange(readNumber(next, min, max) ?? Number.NaN);
      }}
      {...(invalid ? { errorMessage: laterMessage("pagemarks.number.range", { min, max }) } : {})}
      {...(description === undefined || invalid ? {} : { description })}
    />
  );
}

/** Props of `FontField`. */
interface FontFieldProps {
  readonly fonts: readonly MarkFont[];
  readonly value: string;
  readonly onChange: (id: string) => void;
  /** The characters of the text that no font has, which the primary button waits on. */
  readonly missing: readonly string[];
  /** A note under the select. */
  readonly description?: string;
}

/** The font select; characters no font has are named under it. */
function FontField({ fonts, value, onChange, missing, description }: FontFieldProps): ReactNode {
  return (
    <Select
      label={laterMessage("pagemarks.font")}
      value={value}
      onChange={onChange}
      options={fonts.map((font) => ({ value: font.id, label: font.label }))}
      {...(missing.length > 0
        ? {
            errorMessage: laterMessage("pagemarks.font.missing", {
              characters: missing.map((c) => `“${c}”`).join(" "),
            }),
          }
        : description === undefined
          ? {}
          : { description })}
    />
  );
}

/** Props of `PagesField`. */
export interface PagesFieldProps {
  readonly value: PagesChoice;
  readonly onChange: (value: PagesChoice) => void;
  readonly total: number;
  /** Why the range names no page, once it has text. */
  readonly error: string | undefined;
}

/** Pages: all of them, or a range ("2–24, A-1"; page labels work). */
export function PagesField({ value, onChange, total, error }: PagesFieldProps): ReactNode {
  return (
    <div className="paper-marks-pages">
      <RadioGroup
        label={laterMessage("pagemarks.pages")}
        value={value.all ? "all" : "range"}
        onChange={(next) => onChange({ ...value, all: next === "all" })}
      >
        <Radio value="all">
          {laterCountMessage(total, { one: "pagemarks.pages.allOne", many: "pagemarks.pages.all" })}
        </Radio>
        <Radio value="range">{laterMessage("pagemarks.pages.range")}</Radio>
      </RadioGroup>
      {value.all ? null : (
        <TextInput
          label={laterMessage("pagemarks.pages.field")}
          value={value.range}
          onChange={(range) => onChange({ ...value, range })}
          description={laterMessage("pagemarks.pages.help", { total })}
          {...(error === undefined ? {} : { errorMessage: error })}
        />
      )}
    </div>
  );
}

/** Props of `LookRow`. */
export interface LookRowProps {
  readonly fonts: readonly MarkFont[];
  readonly missing: readonly string[];
  readonly values: { readonly fontId: string; readonly size: number };
  readonly onChange: (change: { fontId?: string; size?: number }) => void;
}

/** The font select and the size beside it, which every tab has. */
export function LookRow({ fonts, missing, values, onChange }: LookRowProps): ReactNode {
  return (
    <div className="paper-marks-row">
      <FontField fonts={fonts} value={values.fontId} onChange={(fontId) => onChange({ fontId })} missing={missing} />
      <NumberField
        label={laterMessage("pagemarks.size")}
        value={values.size}
        min={1}
        max={500}
        onChange={(size) => onChange({ size })}
      />
    </div>
  );
}
