// Which link addresses a reader may open (web-app §2: external links confirm before leaving; R07:
// only `http(s):` and `mailto:` open), and how one is shown so it can't pass for another. The
// address comes from the file and is never trusted: `javascript:`, `file:`, `data:` and every other
// scheme are refused, as is anything `URL` can't parse. A host is shown as `URL` gives it: an
// international name in punycode (`xn--…`), so a look-alike host reads as what it is.

/** The schemes a link may open. */
const OPENABLE = new Set(["http:", "https:", "mailto:"]);

/** A link address as a reader treats it. */
export type ExternalLink =
  /** May open after the reader confirms: `href` is the normalised address. */
  | {
      readonly kind: "open";
      readonly href: string;
      /** The address in three parts, the host (or a mailto address's domain) emphasised. */
      readonly parts: { readonly before: string; readonly host: string; readonly after: string };
    }
  /** Never opens: `uri` is the address as the file stores it, for saying so. */
  | { readonly kind: "refused"; readonly uri: string };

/** What a reader does with the link address `uri`. */
export function externalLink(uri: string): ExternalLink {
  let url: URL;
  try {
    url = new URL(uri.trim());
  } catch {
    return { kind: "refused", uri };
  }
  if (!OPENABLE.has(url.protocol)) return { kind: "refused", uri };
  const { href } = url;
  if (url.protocol === "mailto:") {
    // `mailto:` has no host: the domain after the last `@` is what a reader checks.
    const at = href.lastIndexOf("@");
    const end = href.search(/[?#]/);
    const stop = end < 0 ? href.length : end;
    if (at < 0 || at > stop) return { kind: "open", href, parts: { before: href, host: "", after: "" } };
    return {
      kind: "open",
      href,
      parts: { before: href.slice(0, at + 1), host: href.slice(at + 1, stop), after: href.slice(stop) },
    };
  }
  const start = href.indexOf(url.host, url.protocol.length);
  return {
    kind: "open",
    href,
    parts: {
      before: href.slice(0, start),
      host: url.host,
      after: href.slice(start + url.host.length),
    },
  };
}
