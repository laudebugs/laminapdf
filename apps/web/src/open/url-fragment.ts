// The reader's address fragment: `#url=<encoded address>&page=<n>` (ADR 0011 §3: a fragment is
// never sent to our server). Every part of the app that reads or writes the fragment goes through
// here, so a shared link's `url` value stays byte for byte as its sender wrote it.
//
// Parsed by hand rather than with URLSearchParams, whose form rules turn `+` into a space and,
// when writing, encode `~ ! ' ( )` and write spaces as `+`: keys are split on `&`, each pair at its
// first `=`, and values decoded with decodeURIComponent, keeping each raw value beside its decoded
// one. Writing puts an untouched key's raw value back unchanged.

/** One `key=value` pair of the fragment: the value as written (`raw`) and decoded. */
export interface FragmentEntry {
  /** The pair exactly as written (`debug`, `url=https%3A…`), which an untouched key writes back. */
  readonly written: string;
  readonly key: string;
  readonly raw: string;
  /** Undefined when `raw` isn't valid percent-encoding. */
  readonly value: string | undefined;
}

/** A parsed fragment: its pairs in order. */
export interface Fragment {
  readonly entries: readonly FragmentEntry[];
}

/** Why a fragment's `url` can't be opened. */
export type RemoteUrlProblem =
  /** The address wasn't percent-encoded as a whole, so what belongs to it is ambiguous. */
  | "not-encoded"
  /** The decoded value isn't an absolute address. */
  | "not-absolute"
  /** Plain `http:`, which the page's security policy upgrades and most hosts then refuse. */
  | "insecure"
  /** Any scheme but `https:` (`javascript:`, `data:`, `file:` …). */
  | "scheme";

/** What the fragment's `url` key asks for. */
export type RemoteUrl =
  | { readonly kind: "none" }
  | { readonly kind: "url"; readonly url: string }
  | { readonly kind: "invalid"; readonly problem: RemoteUrlProblem };

/** Decodes a percent-encoded value; undefined when it isn't valid. `+` stays `+`. */
function decode(raw: string): string | undefined {
  try {
    return decodeURIComponent(raw);
  } catch {
    return undefined;
  }
}

/** Parses `hash` (with or without its leading `#`). Empty pairs are skipped; a key without `=` has an empty value. */
export function parseFragment(hash: string): Fragment {
  const text = hash.startsWith("#") ? hash.slice(1) : hash;
  const entries: FragmentEntry[] = [];
  for (const pair of text.split("&")) {
    if (pair === "") continue;
    const at = pair.indexOf("=");
    const key = at < 0 ? pair : pair.slice(0, at);
    const raw = at < 0 ? "" : pair.slice(at + 1);
    entries.push({ written: pair, key, raw, value: decode(raw) });
  }
  return { entries };
}

/** The first entry for `key`. */
function entry(fragment: Fragment, key: string): FragmentEntry | undefined {
  return fragment.entries.find((e) => e.key === key);
}

/** Whether the fragment has `key`, with or without a value (`#debug`). */
export function hasKey(fragment: Fragment, key: string): boolean {
  return entry(fragment, key) !== undefined;
}

/** The 1-based page the fragment names (`page=5`), when it names a whole number from 1. */
export function fragmentPage(fragment: Fragment): number | undefined {
  const value = entry(fragment, "page")?.value;
  if (value === undefined || !/^[1-9][0-9]{0,8}$/.test(value)) return undefined;
  return Number(value);
}

/** Hosts that plain `http:` may name in development builds (a local test server). */
const LOCAL_HOSTS = new Set(["localhost", "127.0.0.1"]);

/**
 * The remote address the fragment's `url` names, or why it can't be opened. A raw value holding
 * `?`, `#` or `/` was pasted without encoding (`#url=https://h/doc.pdf?x=1&page=2`, where `&page=2`
 * might be ours or the host's), so it is refused, never guessed.
 *
 * @param allowLocalHttp - accept `http://localhost` and `http://127.0.0.1` (development builds only).
 */
export function remoteUrl(fragment: Fragment, allowLocalHttp = false): RemoteUrl {
  const found = entry(fragment, "url");
  if (found === undefined) return { kind: "none" };
  if (/[?#/]/.test(found.raw) || found.value === undefined) return { kind: "invalid", problem: "not-encoded" };
  let url: URL;
  try {
    url = new URL(found.value);
  } catch {
    return { kind: "invalid", problem: "not-absolute" };
  }
  if (url.protocol === "https:") return { kind: "url", url: found.value };
  if (url.protocol === "http:") {
    if (allowLocalHttp && LOCAL_HOSTS.has(url.hostname)) return { kind: "url", url: found.value };
    return { kind: "invalid", problem: "insecure" };
  }
  return { kind: "invalid", problem: "scheme" };
}

/**
 * The fragment (without `#`) after `updates`: keys keep their order, untouched ones their raw
 * value; an updated value is encoded with encodeURIComponent, `undefined` removes the key, and a
 * new key goes at the end.
 */
export function buildFragment(current: Fragment, updates: Readonly<Record<string, string | undefined>>): string {
  const parts: string[] = [];
  const seen = new Set<string>();
  for (const { key, written } of current.entries) {
    if (!Object.hasOwn(updates, key)) {
      parts.push(written);
      continue;
    }
    // A key named twice keeps one entry, the first, with the update.
    if (seen.has(key)) continue;
    seen.add(key);
    const value = updates[key];
    if (value !== undefined) parts.push(`${key}=${encodeURIComponent(value)}`);
  }
  for (const [key, value] of Object.entries(updates)) {
    if (!seen.has(key) && value !== undefined) {
      parts.push(`${key}=${encodeURIComponent(value)}`);
    }
  }
  return parts.join("&");
}

/** The fragment that opens `url`, at `page` when given: what the share button writes. */
export function fragmentFor(url: string, page?: number): string {
  return buildFragment({ entries: [] }, { url, ...(page === undefined ? {} : { page: String(page) }) });
}
