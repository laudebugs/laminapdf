# `submit-pdf-refused`

- Type: error
- Kind: `security` (encryption, permissions or signatures block the call, or look tampered with)

## What happened

A form's button asks to send the whole PDF to a server (a SubmitForm action with the SubmitPDF flag), so Lamina PDF built no request and sent nothing.

## Why

Lamina PDF never sends a document's bytes anywhere. A form can send the answers in its fields, after the person has seen them and said yes, but not the file: it may hold comments, earlier revisions and other things the person didn't mean to share.

## How to fix

Export the form data, or save the file, and send it yourself to the address the form's sender gave you.

## Example

Pressing a "Submit" button whose action carries the SubmitPDF flag (bit 9 of `/Flags`).

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E8i](../todos/ongoing/E8i-form-actions-and-submit.md): push buttons, form actions and submit
- [E8 plan §2.3](../plans/editor/e8-form-filling.md#23-behaviour-actions-and-submit)
- [ADR 0011](../decisions/0011-cloudflare-topology-and-privacy.md): privacy topology, guarantee 1
