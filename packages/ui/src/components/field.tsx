// Text fields and the select (§7.19): the label above, help or error text below, wired with
// `aria-describedby` and `aria-invalid`. React Aria's TextField does the wiring for inputs; the
// select is a native <select>, wired the same way by hand.

import { Icon } from "@laminapdf/icons/react";
import { type ReactNode, useId } from "react";
import {
  TextArea as AriaTextArea,
  TextField as AriaTextField,
  type TextFieldProps as AriaTextFieldProps,
  FieldError,
  Input,
  Label,
  Text,
} from "react-aria-components";

/** What every field shows around its control. */
export interface FieldText {
  /** The visible label, above the control. Never a placeholder instead. */
  label: string;
  /** Help text below the control. */
  description?: string;
  /** An error, shown below in `danger` in place of the help text, and marking the field invalid. */
  errorMessage?: string;
}

type TextFieldBase = Omit<AriaTextFieldProps, "className" | "style" | "children" | "isInvalid"> & FieldText;

/** Props of `TextInput` and `TextArea`. */
export interface TextInputProps extends TextFieldBase {
  /** A hint shown while the field is empty; the label still names it. */
  placeholder?: string;
}

function FieldShell({ label, description, errorMessage, children, ...rest }: TextFieldBase & { children: ReactNode }) {
  return (
    <AriaTextField {...rest} isInvalid={errorMessage !== undefined} className="paper-field">
      <Label className="paper-field-label">{label}</Label>
      {children}
      {description === undefined || errorMessage !== undefined ? null : (
        <Text slot="description" className="paper-field-help">
          {description}
        </Text>
      )}
      <FieldError className="paper-field-error">{errorMessage}</FieldError>
    </AriaTextField>
  );
}

/** A one-line text field: 32 px high on desktop, 44 px with 16 px text on touch. */
export function TextInput({ placeholder, ...rest }: TextInputProps): ReactNode {
  return (
    <FieldShell {...rest}>
      <Input className="paper-input" {...(placeholder === undefined ? {} : { placeholder })} />
    </FieldShell>
  );
}

/** A multi-line text field that grows only vertically. */
export function TextArea({ placeholder, ...rest }: TextInputProps): ReactNode {
  return (
    <FieldShell {...rest}>
      <AriaTextArea className="paper-input paper-textarea" {...(placeholder === undefined ? {} : { placeholder })} />
    </FieldShell>
  );
}

/** One choice in a `Select`. */
export interface SelectOption<Value extends string> {
  /** The value `onChange` receives. */
  value: Value;
  /** The visible text. */
  label: string;
}

/** Props of `Select`. */
export interface SelectProps<Value extends string> extends FieldText {
  /** The choices, in order. */
  options: readonly SelectOption<Value>[];
  /** The chosen value. */
  value: Value;
  /** Called with the value the user chose. */
  onChange: (value: Value) => void;
  /** Whether the select can't be used. */
  isDisabled?: boolean;
}

/** A native select with the Paper look and a chevron, so the platform's own picker opens. */
export function Select<Value extends string>({
  label,
  description,
  errorMessage,
  options,
  value,
  onChange,
  isDisabled = false,
}: SelectProps<Value>): ReactNode {
  const id = useId();
  const below = errorMessage ?? description;
  return (
    <div className="paper-field" data-invalid={errorMessage === undefined ? undefined : true}>
      <label className="paper-field-label" htmlFor={id}>
        {label}
      </label>
      <div className="paper-select">
        <select
          id={id}
          className="paper-input"
          value={value}
          disabled={isDisabled}
          aria-invalid={errorMessage === undefined ? undefined : true}
          aria-describedby={below === undefined ? undefined : `${id}-below`}
          onChange={(event) => onChange(event.currentTarget.value as Value)}
        >
          {options.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
        <Icon name="chevron-down" size={16} className="paper-select-chevron" />
      </div>
      {below === undefined ? null : (
        <span id={`${id}-below`} className={errorMessage === undefined ? "paper-field-help" : "paper-field-error"}>
          {below}
        </span>
      )}
    </div>
  );
}
