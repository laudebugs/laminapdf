# `content-object-invalid`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

An op adds an image or a page of another PDF to a page, and the bytes it names can't be read as one. They aren't a JPEG or PNG image, or they are a PDF that can't be opened or doesn't have the page the op asks for.

## Why

Lamina PDF writes a JPEG image as it is and a PNG image with its transparency, and it reads no other image format. A page of another PDF is copied in with its content and resources, so that PDF must open without a password and have the page.

## How to fix

Pass the bytes of a JPEG, a PNG or a PDF to `addBlob`, and use the blob it returns as the object's `blob`. Convert other image formats, such as WebP or HEIC, to PNG first. For a PDF, count its pages from 0.

## Example

```json
{ "op": "addContentObject", "args": { "id": "seal", "page": "p3.0", "object": { "form": { "blob": "…", "page": 4 } }, "matrix": [72, 0, 0, 72, 400, 680], "modified": "D:20261118120000Z" } }
```

The blob is a PDF with two pages, so it has no page 4.

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E6b](../todos/done/E6b-place-images-and-shapes.md): images, shapes, logos and stamps placed as page content
