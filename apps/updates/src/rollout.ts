// Staged rollout (ADR 0014 §2). An install's bucket for a release is
// u32_be(SHA-256(install_id_lowercase ‖ ":" ‖ release_version)[0..4]) mod 10 000, and it gets the
// release when its bucket is below rollout_percent × 100. Hashing with the version gives every
// release its own cohorts, so the same installs aren't always first.

/** The number of buckets: one per hundredth of a percent. */
export const BUCKETS = 10_000;

// A UUID of version 4 (RFC 9562 §5.4): version nibble 4, variant 10xx. Either case.
const UUID_V4 = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;

/**
 * The install ID from the `Lamina-Install-Id` header, lower-cased, or `undefined` when the header is
 * missing or isn't a UUID v4: such a request gets only releases at 100 %. The value is never logged
 * and never leaves this Worker.
 */
export function installIdOf(header: string | null): string | undefined {
  if (header === null || !UUID_V4.test(header)) return undefined;
  return header.toLowerCase();
}

/** The install's bucket, 0 to 9 999, for `version`. `installId` must come from {@link installIdOf}. */
export async function bucketFor(installId: string, version: string): Promise<number> {
  const bytes = new TextEncoder().encode(`${installId}:${version}`);
  const digest = new DataView(await crypto.subtle.digest("SHA-256", bytes));
  return digest.getUint32(0, false) % BUCKETS;
}

/** Whether an install in `bucket` (or with no usable ID) gets a release at `rolloutPercent`. */
export function inRollout(bucket: number | undefined, rolloutPercent: number): boolean {
  if (rolloutPercent >= 100) return true;
  if (bucket === undefined) return false;
  return bucket < Math.round(rolloutPercent * 100);
}
