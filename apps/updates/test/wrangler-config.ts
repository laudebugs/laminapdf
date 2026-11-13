// wrangler.jsonc as the tests read it, so they check the Worker's real settings rather than copies.
import { readFileSync } from "node:fs";

/** The R2 bindings and the compatibility date of one of wrangler.jsonc's Workers. */
export interface WorkerConfig {
  readonly compatibility_date: string;
  readonly r2_buckets: readonly { readonly binding: string; readonly bucket_name: string }[];
}

/** wrangler.jsonc: its top-level Worker (production) and each `env`. Its comments take whole lines. */
export function wranglerConfig(): WorkerConfig & { readonly env: Readonly<Record<string, Partial<WorkerConfig>>> } {
  const text = readFileSync(new URL("../wrangler.jsonc", import.meta.url), "utf8");
  const json = text
    .split("\n")
    .filter((line) => !line.trimStart().startsWith("//"))
    .join("\n");
  return JSON.parse(json);
}
