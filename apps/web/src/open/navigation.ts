// What a change of the address fragment asks the reader to do (`hashchange`, `popstate` and the
// first load). Only a changed `url` (added, removed or another address) opens or closes a
// document; a fragment that differs only in `page` moves within the open one, so Back and Forward
// through page entries never fetch or open again.

import { type Fragment, fragmentPage, type RemoteUrlProblem, remoteUrl } from "./url-fragment.ts";

/** The reader's response to a fragment. */
export type FragmentAction =
  | { readonly type: "open"; readonly url: string; readonly page: number | undefined }
  | { readonly type: "close" }
  | { readonly type: "navigate"; readonly page: number }
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
  const page = fragmentPage(fragment);
  if (remote.kind === "invalid") return { type: "invalid", problem: remote.problem };
  if (remote.kind === "none") {
    if (openUrl !== undefined) return { type: "close" };
    return page === undefined ? { type: "none" } : { type: "navigate", page };
  }
  if (remote.url !== openUrl) return { type: "open", url: remote.url, page };
  return page === undefined ? { type: "none" } : { type: "navigate", page };
}
