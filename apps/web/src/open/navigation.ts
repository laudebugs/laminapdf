// What a change of the address fragment asks the reader to do (`hashchange`, `popstate` and the
// first load). Only a changed `url` (added, removed or another address) opens or closes a
// document; a fragment that differs only in `page` or `nameddest` moves within the open one, so
// Back and Forward through page entries never fetch or open again.

import { type Fragment, fragmentDestination, fragmentPage, type RemoteUrlProblem, remoteUrl } from "./url-fragment.ts";

/** Where in a document a fragment goes: a 1-based page, or a named destination. */
export type FragmentPlace = { readonly page: number } | { readonly dest: string };

/** The place `fragment` names: `nameddest` before `page`, as pdf.js reads them. */
export function fragmentPlace(fragment: Fragment): FragmentPlace | undefined {
  const dest = fragmentDestination(fragment);
  if (dest !== undefined) return { dest };
  const page = fragmentPage(fragment);
  return page === undefined ? undefined : { page };
}

/** The reader's response to a fragment. */
export type FragmentAction =
  | { readonly type: "open"; readonly url: string; readonly place: FragmentPlace | undefined }
  | { readonly type: "close" }
  | { readonly type: "navigate"; readonly place: FragmentPlace }
  | { readonly type: "invalid"; readonly problem: RemoteUrlProblem }
  | { readonly type: "none" };

/**
 * What `fragment` asks for, given the remote address open now (`undefined` when none is, or the
 * open document is a local file).
 *
 * @param allowLocalHttp - see {@link remoteUrl}.
 */
export function fragmentAction(
  fragment: Fragment,
  openUrl: string | undefined,
  allowLocalHttp = false,
): FragmentAction {
  const remote = remoteUrl(fragment, allowLocalHttp);
  const place = fragmentPlace(fragment);
  if (remote.kind === "invalid") return { type: "invalid", problem: remote.problem };
  if (remote.kind === "none") {
    if (openUrl !== undefined) return { type: "close" };
    return place === undefined ? { type: "none" } : { type: "navigate", place };
  }
  if (remote.url !== openUrl) return { type: "open", url: remote.url, place };
  return place === undefined ? { type: "none" } : { type: "navigate", place };
}
