// A document's fingerprint, the key its reading position is kept under: a hash of the file's head,
// up to the end of its first revision. An incremental update appends to the file (ISO 32000-2
// §7.5.6) and leaves the head alone, so a file reopens at its place after someone adds a note or
// a signature; a full rewrite changes the head, so its output starts without one. `/ID[0]` stays
// out of it: a file without one gains one on its first save, which would change the key.

/** How much of the file's start is read: the first revision's end is looked for in it. */
export const HEAD_BYTES = 64 * 1024;

/** The fingerprint's version, so a later scheme can live beside this one. */
const PREFIX = "fp1:";

/** What the hash starts with: the scheme's name and a zero byte. */
const TAG = new TextEncoder().encode("laminapdf.fp/1\0");

/** `%%EOF`, the marker that ends a revision (§7.5.5). */
const EOF_MARKER = new TextEncoder().encode("%%EOF");

/** The bytes the fingerprint covers: `head` through the end of its first `%%EOF`, else all of it. */
export function fingerprintedBytes(head: Uint8Array): Uint8Array {
  const at = indexOf(head, EOF_MARKER);
  return at < 0 ? head : head.subarray(0, at + EOF_MARKER.length);
}

/**
 * The fingerprint of a file whose first bytes are `head` (its first {@link HEAD_BYTES}, or the
 * whole file when it is smaller): `"fp1:"` and a SHA-256 in hex.
 */
export async function documentFingerprint(head: Uint8Array): Promise<string> {
  const covered = fingerprintedBytes(head.subarray(0, HEAD_BYTES));
  const input = new Uint8Array(TAG.length + covered.length);
  input.set(TAG);
  input.set(covered, TAG.length);
  const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", input));
  return PREFIX + Array.from(digest, (b) => b.toString(16).padStart(2, "0")).join("");
}

/** Where `needle` first starts in `bytes`, or −1. */
function indexOf(bytes: Uint8Array, needle: Uint8Array): number {
  for (let i = 0; i + needle.length <= bytes.length; i++) {
    let j = 0;
    while (j < needle.length && bytes[i + j] === needle[j]) j++;
    if (j === needle.length) return i;
  }
  return -1;
}
