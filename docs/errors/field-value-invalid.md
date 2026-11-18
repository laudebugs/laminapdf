# `field-value-invalid`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

A `SetFieldValue` op gave a value that doesn't fit its field, so nothing was changed.

## Why

Each kind of field takes its own kind of value (ISO 32000-2 §12.7.5). A text field takes text, no longer than its maximum length when it has one. A check box or a radio group takes one of the states its buttons show, or off. A list or combo box takes options from its list, and only a list that allows several choices takes more than one. A combo box marked editable also takes text that isn't one of its options. A push button has no value at all. A field whose scripts are Acrobat's form helpers also checks the text as Acrobat does: a number field takes a number, a date field a date in its pattern, a zip code or phone field digits that fit its mask, and a field with a range a number inside it. Lamina PDF does these checks itself and runs no script.

## How to fix

Read the field's kind, options, states and maximum length from the form, and give a value it accepts. For a field with a format or a range, `field_behaviour` says what it takes, and `validate` checks a value before you set it. The data field `reason` says which rule the value broke.

## Example

```rust
let name = doc.form()?.fields.into_iter().find(|f| f.name == "name").ok_or("no name field")?;
let op = SetFieldValue::new(name.id.parse()?, FieldValue::Button(Some("Yes".into())));
doc.apply(vec![Op::from(op)]); // field-value-invalid: a text field takes text
```

## Data fields

- `reason`: the rule the value broke, one of:
  - `kind`: a value for another kind of field, or any value for a push button
  - `max-len`: text longer than the field's maximum length
  - `option`: a choice that isn't one of the field's options
  - `state`: a button state none of the field's buttons shows
  - `count`: several choices where the field takes one, or none for a combo box
  - `format`: text the field's number, date, time or special format refuses
  - `range`: a number outside the field's range
