// Checkboxes, radio groups and switches (§7.19). Each is a real input, visually hidden inside its
// label, with a drawn box, circle or track beside it; React Aria provides the input, keyboard and
// state, so the whole label row is the target.

import { Icon } from "@laminapdf/icons/react";
import type { ReactNode } from "react";
import {
  Checkbox as AriaCheckbox,
  type CheckboxProps as AriaCheckboxProps,
  Radio as AriaRadio,
  RadioGroup as AriaRadioGroup,
  type RadioGroupProps as AriaRadioGroupProps,
  type RadioProps as AriaRadioProps,
  Switch as AriaSwitch,
  type SwitchProps as AriaSwitchProps,
  FieldError,
  Label,
  Text,
} from "react-aria-components";

/** Props of `Checkbox`. */
export interface CheckboxProps extends Omit<AriaCheckboxProps, "children" | "className" | "style"> {
  /** The visible label. */
  children: ReactNode;
}

/** A checkbox: checked, unchecked or mixed (`isIndeterminate`). */
export function Checkbox({ children, ...rest }: CheckboxProps): ReactNode {
  return (
    <AriaCheckbox {...rest} className="paper-choice paper-checkbox">
      <span className="paper-checkbox-box" aria-hidden="true">
        <Icon name="check" size={16} strokeWidth={2} className="paper-checkbox-check" />
        <span className="paper-checkbox-dash" />
      </span>
      <span className="paper-choice-label">{children}</span>
    </AriaCheckbox>
  );
}

/** Props of `RadioGroup`. */
export interface RadioGroupProps extends Omit<AriaRadioGroupProps, "children" | "className" | "style" | "isInvalid"> {
  /** The group's visible label. */
  label: string;
  /** Help text below the group. */
  description?: string;
  /** An error below the group, marking it invalid. */
  errorMessage?: string;
  /** The `Radio`s. */
  children: ReactNode;
}

/** A labelled group of radios: arrow keys move between them and select as they go. */
export function RadioGroup({ label, description, errorMessage, children, ...rest }: RadioGroupProps): ReactNode {
  return (
    <AriaRadioGroup {...rest} isInvalid={errorMessage !== undefined} className="paper-radio-group">
      <Label className="paper-field-label">{label}</Label>
      <div className="paper-radio-options">{children}</div>
      {description === undefined || errorMessage !== undefined ? null : (
        <Text slot="description" className="paper-field-help">
          {description}
        </Text>
      )}
      <FieldError className="paper-field-error">{errorMessage}</FieldError>
    </AriaRadioGroup>
  );
}

/** Props of `Radio`. */
export interface RadioProps extends Omit<AriaRadioProps, "children" | "className" | "style"> {
  /** The visible label. */
  children: ReactNode;
}

/** One choice in a `RadioGroup`. */
export function Radio({ children, ...rest }: RadioProps): ReactNode {
  return (
    <AriaRadio {...rest} className="paper-choice paper-radio">
      <span className="paper-radio-circle" aria-hidden="true" />
      <span className="paper-choice-label">{children}</span>
    </AriaRadio>
  );
}

/** Props of `Switch`. */
export interface SwitchProps extends Omit<AriaSwitchProps, "children" | "className" | "style"> {
  /** The visible label, before the switch. */
  children: ReactNode;
}

/** A switch (`role="switch"`), only for settings that apply at once. */
export function Switch({ children, ...rest }: SwitchProps): ReactNode {
  return (
    <AriaSwitch {...rest} className="paper-choice paper-switch">
      <span className="paper-choice-label">{children}</span>
      <span className="paper-switch-track" aria-hidden="true">
        <span className="paper-switch-knob" />
      </span>
    </AriaSwitch>
  );
}
