// The update test's pure parts (D5c): the platform key the Worker serves, the staging manifest of
// one step, the bytes of a tampered bundle, and the check of what reached the Worker. update-test.ts
// runs them against the app; test/update-e2e.test.ts tests them.

/** The channel the test serves: the Worker's `staging`, which the app never offers. */
export const CHANNEL = "staging";

/** The version installed first, and the one it updates to. Versions below 0.1.0 are this test's. */
export const OLD_VERSION = "0.0.1";
export const NEW_VERSION = "0.0.2";

/** The header that carries the install ID (ADR 0014 §3). */
export const INSTALL_ID_HEADER = "lamina-install-id";

const UUID_V4 = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;

/** The updater's `{{target}}` and `{{arch}}` for a Node `platform` and `arch`. */
export function platformOf(platform: string, arch: string): { target: string; arch: string } {
  const target = { darwin: "darwin", win32: "windows", linux: "linux" }[platform];
  const cpu = { arm64: "aarch64", x64: "x86_64" }[arch];
  if (target === undefined || cpu === undefined) throw new Error(`no update platform for ${platform} ${arch}`);
  return { target, arch: cpu };
}

/** The staging manifest that offers `NEW_VERSION` at `url`, signed with `signature` (a `.sig` file). */
export function manifestFor(key: string, url: string, signature: string, now: Date): object {
  return {
    schema: 1,
    channel: CHANNEL,
    paused: false,
    release: {
      version: NEW_VERSION,
      pub_date: now.toISOString().replace(/\.\d+Z$/, "Z"),
      notes: "The desktop update test's release (D5c)",
      rollout_percent: 100,
      platforms: { [key]: { url, signature: signature.trim() } },
    },
  };
}

/** The staging manifest that offers nothing, which the staging run leaves behind. */
export function emptyManifest(): object {
  return { schema: 1, channel: CHANNEL, paused: true };
}

/** `bytes` with one byte in the middle flipped: a bundle whose signature no longer matches. */
export function tampered(bytes: Uint8Array): Uint8Array {
  if (bytes.length === 0) throw new Error("the bundle is empty");
  const copy = Uint8Array.from(bytes);
  const at = Math.floor(copy.length / 2);
  copy[at] = (copy[at] ?? 0) ^ 0xff;
  return copy;
}

/** A request the app made to the update server, as the test's proxy saw it. */
export interface SeenRequest {
  /** The path and query. */
  readonly url: string;
  /** The `Lamina-Install-Id` header, if any. */
  readonly installId: string | undefined;
}

/**
 * What is wrong with the app's update requests: each must ask the staging endpoint for this
 * platform at `version`, with a UUID v4 install ID in the header and nowhere in the URL.
 */
export function requestProblems(
  seen: readonly SeenRequest[],
  platform: { target: string; arch: string },
  version: string,
): string[] {
  if (seen.length === 0) return ["the app made no update request"];
  const path = `/${CHANNEL}/${platform.target}/${platform.arch}/${version}`;
  const problems: string[] = [];
  for (const request of seen) {
    if (request.url !== path) problems.push(`asked for ${request.url}, not ${path}`);
    if (request.installId === undefined) {
      problems.push(`${request.url} came without the ${INSTALL_ID_HEADER} header`);
      continue;
    }
    if (!UUID_V4.test(request.installId)) problems.push(`${request.url} sent an install ID that isn't a UUID v4`);
    if (request.url.toLowerCase().includes(request.installId.toLowerCase())) {
      problems.push(`${request.url} holds the install ID`);
    }
  }
  return problems;
}
