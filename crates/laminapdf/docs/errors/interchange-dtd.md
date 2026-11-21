---
title: "interchange-dtd"
version: "0.7.0"
url: "https://docs.laminapdf.com/reference/codes/interchange-dtd/"
---

# interchange-dtd

- Type: error
- Kind: `security` (encryption, permissions or signatures block the call, or look tampered with)

## What happened

An XFDF file declares a document type definition (`<!DOCTYPE …>`), so Lamina PDF refused to read it and imported nothing.

## Why

A DTD can define entities that expand without end (the "billion laughs" attack) or that read other files from the computer reading it (XXE). XFDF needs no DTD (ISO 19444-1), so a file with one is refused rather than read with the DTD ignored. XMP metadata is read with the same rule.

## How to fix

Export the comments again from the app that made the file. If you made the file yourself, delete the `<!DOCTYPE …>` declaration.

## Example

```xml
<?xml version="1.0"?>
<!DOCTYPE xfdf [<!ENTITY x "boom">]>
<xfdf xmlns="http://ns.adobe.com/xfdf/"/>
```

## Data fields

- `line`: where the declaration starts, counted from 1.
