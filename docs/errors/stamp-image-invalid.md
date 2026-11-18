# `stamp-image-invalid`

- Type: error
- Kind: `usage` (the caller misused the API)

## What happened

An image stamp names its picture by the ID of bytes added to the session, and those bytes aren't a JPEG or PNG image Lamina PDF can read.

## Why

An image stamp draws its picture inside the stamp's appearance, the same way a picture placed on a page is drawn. Lamina PDF writes JPEG files as they are and PNG files with their transparency, and it reads no other image format.

## How to fix

Add the bytes of a JPEG or PNG file, and name that blob's ID as the stamp's image. Convert other formats, such as WebP or HEIC, to PNG first.

## Example

An image stamp whose `image` is the ID of a WebP file added with `addBlob`.

## Data fields

None.

## For contributors

The decisions and todos behind this code:

- [E5c](../todos/done/E5c-stamps-and-attachments.md): stamps, file attachments and audio comments
