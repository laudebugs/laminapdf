// Which buffers a message between the page and the worker can move instead of copying.

/** The buffer of `value` if transferring it moves exactly `value` (not a larger or shared buffer). */
export function whole(value: unknown): ArrayBuffer[] {
  if (value instanceof ArrayBuffer) return [value];
  if (!ArrayBuffer.isView(value)) return [];
  const { buffer } = value;
  return buffer instanceof ArrayBuffer && value.byteOffset === 0 && value.byteLength === buffer.byteLength
    ? [buffer]
    : [];
}
