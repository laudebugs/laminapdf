# `submit-url-refused`

- Type: error
- Kind: `security` (encryption, permissions or signatures block the call, or look tampered with)

## What happened

A form's button sends its answers to an address that isn't an `https:` address, so Lamina PDF built no request and sent nothing.

## Why

Answers sent over `http:` can be read and changed on their way. Lamina PDF sends only to `https:` addresses with a host and no user name or password in them. A `mailto:` address is the exception: it opens the mail app and sends no data.

## How to fix

Ask the form's sender for an `https:` address, or export the form data and send that file yourself.

## Example

A Send button whose address is `http://forms.example.com/submit`.

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E8i](../todos/done/E8i-form-actions-and-submit.md): push buttons, form actions and submit
- [E8 plan §2.3](../plans/editor/e8-form-filling.md#23-behaviour-actions-and-submit)
- [ADR 0011](../decisions/0011-cloudflare-topology-and-privacy.md): privacy topology
