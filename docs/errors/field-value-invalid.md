# `field-value-invalid`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

A `SetFieldValue` op gave a value that doesn't fit its field, so nothing was changed.

## Why

Each kind of field takes its own kind of value (ISO 32000-2 §12.7.5). A text field takes text, no longer than its maximum length when it has one. A check box or a radio group takes one of the states its buttons show, or off. A list or combo box takes options from its list, and only a list that allows several choices takes more than one. A combo box marked editable also takes text that isn't one of its options. A push button has no value at all.

## How to fix

Read the field's kind, options, states and maximum length from the form, and give a value it accepts. The data field `reason` says which rule the value broke.

## Example

```rust
let name = doc.form()?.fields.into_iter().find(|f| f.name == "name").ok_or("no name field")?;
let op = SetFieldValue::new(name.id.parse()?, FieldValue::Button(Some("Yes".into())));
doc.apply(vec![Op::from(op)]); // field-value-invalid: a text field takes text
```

## Data fields

- `reason`: the rule the value broke: `kind` (a value for another kind of field, or any value for a push button), `max-len` (text longer than the field's maximum length), `option` (a choice that isn't one of the field's options), `state` (a button state none of its buttons shows) or `count` (several choices for a field that takes one, or none for a combo box).
