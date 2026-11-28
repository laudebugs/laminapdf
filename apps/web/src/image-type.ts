// Which image a file's bytes are, judged by their signature and not by the file's name or type:
// the stamp library and the signature images keep only a real PNG or JPEG.

/** A picture type a PDF can draw: PNG or JPEG. */
export type ImageType = "image/png" | "image/jpeg";

/** The PNG and JPEG signatures. */
const PNG = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
const JPEG = [0xff, 0xd8, 0xff];

/** `bytes`' image type, from their signature; undefined for anything but PNG and JPEG. */
export function imageType(bytes: Uint8Array): ImageType | undefined {
  const starts = (signature: readonly number[]) => signature.every((b, i) => bytes[i] === b);
  if (starts(PNG)) return "image/png";
  if (starts(JPEG)) return "image/jpeg";
  return undefined;
}
