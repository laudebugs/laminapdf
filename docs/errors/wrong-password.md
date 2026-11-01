# `wrong-password`

- Type: error
- Kind: `security` (encryption, permissions or signatures block the call, or look tampered with)

## What happened

The given password is neither the user nor the owner password.

## Why

The password is neither the user nor the owner password. Lamina PDF tries it as both (owner first for R5/R6), and in TS also its NFKC form.

## How to fix

Ask for the password again. Passwords are case-sensitive.

## Example

```ts
Document.open(bytes, { password: "nope" });
// LaminaPdfError { code: "wrong-password", kind: "security",
//   hint: "ask for the password again and retry with the correct one", … }
```

## Data fields

None.
