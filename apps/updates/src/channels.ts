// The channels and the R2 bucket each one lives in (ADR 0014 §1, §5): the one list the Worker's entry,
// the manifest tool and the tests read. wrangler.jsonc binds the same buckets, and channels.test.ts
// holds the two together.

import { isOneOf } from "./one-of.ts";

/** The channels an install can follow, plus `staging`, which only tests use and the app never offers. */
export const CHANNELS = ["stable", "beta", "nightly", "staging"] as const;

/** One of {@link CHANNELS}. */
export type Channel = (typeof CHANNELS)[number];

/** The Worker's R2 binding names, as wrangler.jsonc declares them. */
type BucketBinding = "RELEASES" | "NIGHTLY" | "STAGING";

/** Where a channel's manifest and files live: the Worker's binding and the bucket's name. */
export interface ChannelBucket {
  readonly binding: BucketBinding;
  readonly bucket: string;
}

/**
 * Each channel's bucket. Stable and beta share the releases bucket, so promoting between them
 * moves no files; nightly and staging have their own, so their write tokens can't touch releases.
 */
export const CHANNEL_BUCKETS: Readonly<Record<Channel, ChannelBucket>> = {
  stable: { binding: "RELEASES", bucket: "lamina-pdf-releases" },
  beta: { binding: "RELEASES", bucket: "lamina-pdf-releases" },
  nightly: { binding: "NIGHTLY", bucket: "lamina-pdf-nightly" },
  staging: { binding: "STAGING", bucket: "lamina-pdf-staging" },
};

/** Whether `value` names a channel. */
export function isChannel(value: unknown): value is Channel {
  return isOneOf(CHANNELS, value);
}
