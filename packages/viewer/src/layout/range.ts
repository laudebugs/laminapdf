/** `first..last`, both included (empty when `last` is before `first`). */
export function range(first: number, last: number): number[] {
  return Array.from({ length: Math.max(0, last - first + 1) }, (_, k) => first + k);
}
