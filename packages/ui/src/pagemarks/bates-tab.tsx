// The Bates numbers tab (Paper A03): prefix, start number, digits, suffix and the slot, with the
// first and last number the pages get.

import type { ReactNode } from "react";
import { Select, TextInput } from "../components/field.js";
import { laterMessage } from "../later-messages.js";
import { LookRow, type MarkFont, NumberField } from "./fields.js";
import { type BatesValues, MARK_SLOTS, type MarkSlot } from "./model.js";

/** Props of `BatesTab`. */
export interface BatesTabProps {
  readonly values: BatesValues;
  readonly onChange: (change: Partial<BatesValues>) => void;
  readonly fonts: readonly MarkFont[];
  readonly missing: readonly string[];
  /** The first and last number, as the pages will show them. */
  readonly range: string;
  /** Whether the slot holds text already (the overlap notice says what). */
  readonly overlapping: boolean;
}

/** The Bates numbers tab's form. */
export function BatesTab({ values, onChange, fonts, missing, range, overlapping }: BatesTabProps): ReactNode {
  return (
    <div className="paper-marks-fields">
      <div className="paper-marks-row">
        <TextInput
          label={laterMessage("pagemarks.bates.prefix")}
          value={values.prefix}
          onChange={(prefix) => onChange({ prefix })}
          autoFocus
        />
        <TextInput
          label={laterMessage("pagemarks.bates.suffix")}
          value={values.suffix}
          onChange={(suffix) => onChange({ suffix })}
        />
      </div>
      <div className="paper-marks-row">
        <NumberField
          label={laterMessage("pagemarks.bates.start")}
          value={values.start}
          min={0}
          max={9_007_199_254_740_991}
          onChange={(start) => onChange({ start })}
        />
        <NumberField
          label={laterMessage("pagemarks.bates.digits")}
          value={values.digits}
          min={0}
          max={20}
          description={laterMessage("pagemarks.bates.digits.help")}
          onChange={(digits) => onChange({ digits })}
        />
      </div>
      <Select
        label={laterMessage("pagemarks.bates.slot")}
        value={values.slot}
        onChange={(slot: MarkSlot) => onChange({ slot })}
        options={MARK_SLOTS.map((slot) => ({ value: slot, label: laterMessage(`pagemarks.slot.${slot}`) }))}
        {...(overlapping ? { description: laterMessage("pagemarks.slot.overlap") } : {})}
      />
      <LookRow fonts={fonts} missing={missing} values={values} onChange={onChange} />
      <p className="paper-marks-help" role="status">
        {range}
      </p>
    </div>
  );
}
