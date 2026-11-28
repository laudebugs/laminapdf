// The Header & footer tab (Paper A03): a grid of six slots, the tokens that go into the slot being
// typed in, the size, the margins at the top and the bottom, and how the date is written. Tokens
// are shown in square brackets ("[Page] of [Pages]") and written as braces when the mark is added.

import { type ReactNode, useRef } from "react";
import { Button } from "../components/button.js";
import { Select, TextInput } from "../components/field.js";
import { laterMessage } from "../later-messages.js";
import { LookRow, type MarkFont, NumberField } from "./fields.js";
import {
  DATE_FORMATS,
  type DateFormat,
  formatDate,
  type HeaderFooterValues,
  MARK_SLOTS,
  MARK_TOKENS,
  type MarkSlot,
} from "./model.js";

/** Props of `HeaderFooterTab`. */
export interface HeaderFooterTabProps {
  readonly values: HeaderFooterValues;
  readonly onChange: (change: Partial<HeaderFooterValues>) => void;
  readonly fonts: readonly MarkFont[];
  readonly missing: readonly string[];
  /** Today, for the date formats' examples. */
  readonly today: Date;
  /** The slot the overlap notice describes. */
  readonly overlapping: MarkSlot | undefined;
}

/** The id of a slot's input, which a token button focuses again. */
const slotId = (slot: MarkSlot) => `paper-marks-slot-${slot}`;

/** Where the caret was in the slot last typed in. */
interface Caret {
  readonly slot: MarkSlot;
  readonly start: number;
  readonly end: number;
}

/** The Header & footer tab's form. */
export function HeaderFooterTab(props: HeaderFooterTabProps): ReactNode {
  const { values, onChange } = props;
  const caret = useRef<Caret>({ slot: "bottomRight", start: values.slots[5].length, end: values.slots[5].length });
  const setSlot = (slot: MarkSlot, text: string) => {
    const at = MARK_SLOTS.indexOf(slot);
    const slots = values.slots.map((was, i) => (i === at ? text : was)) as unknown as HeaderFooterValues["slots"];
    onChange({ slots });
  };
  const insert = (token: string) => {
    const { slot, start, end } = caret.current;
    const text = values.slots[MARK_SLOTS.indexOf(slot)] ?? "";
    setSlot(slot, text.slice(0, start) + token + text.slice(end));
    const at = start + token.length;
    caret.current = { slot, start: at, end: at };
    // The slot's input is back in focus, with the caret after the token.
    requestAnimationFrame(() => {
      const input = document.getElementById(slotId(slot)) as HTMLInputElement | null;
      input?.focus();
      input?.setSelectionRange(at, at);
    });
  };
  return (
    <div className="paper-marks-fields">
      <fieldset className="paper-marks-slots">
        <legend className="paper-visually-hidden">{laterMessage("pagemarks.slots")}</legend>
        {MARK_SLOTS.map((slot, i) => (
          <TextInput
            key={slot}
            id={slotId(slot)}
            label={laterMessage(`pagemarks.slot.${slot}`)}
            value={values.slots[i] ?? ""}
            onChange={(text) => setSlot(slot, text)}
            onBlur={(event) => {
              const input = event.target as HTMLInputElement;
              caret.current = { slot, start: input.selectionStart ?? 0, end: input.selectionEnd ?? 0 };
            }}
            {...(props.overlapping === slot ? { description: laterMessage("pagemarks.slot.overlap") } : {})}
          />
        ))}
      </fieldset>
      <fieldset className="paper-marks-tokens">
        <legend className="paper-visually-hidden">{laterMessage("pagemarks.tokens")}</legend>
        <span className="paper-marks-tokens-label" aria-hidden="true">
          {laterMessage("pagemarks.tokens")}
        </span>
        {MARK_TOKENS.map((token) => (
          <Button key={token.id} size={30} onPress={() => insert(token.shown)}>
            {token.shown}
          </Button>
        ))}
      </fieldset>
      <p className="paper-marks-help">{laterMessage("pagemarks.tokens.help")}</p>
      <LookFields {...props} />
    </div>
  );
}

/** Font, size, the two margins and how the date is written. */
function LookFields({ values, onChange, fonts, missing, today }: HeaderFooterTabProps): ReactNode {
  return (
    <>
      <LookRow fonts={fonts} missing={missing} values={values} onChange={onChange} />
      <div className="paper-marks-row">
        <NumberField
          label={laterMessage("pagemarks.margin.top")}
          value={values.top}
          min={0}
          max={500}
          onChange={(top) => onChange({ top })}
        />
        <NumberField
          label={laterMessage("pagemarks.margin.bottom")}
          value={values.bottom}
          min={0}
          max={500}
          onChange={(bottom) => onChange({ bottom })}
        />
      </div>
      <Select
        label={laterMessage("pagemarks.date")}
        value={values.dateFormat}
        onChange={(dateFormat: DateFormat) => onChange({ dateFormat })}
        options={DATE_FORMATS.map((format) => ({ value: format, label: formatDate(today, format) }))}
      />
    </>
  );
}
