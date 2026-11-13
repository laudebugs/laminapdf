// The address fragment, read when the page loads and on every `hashchange` and `popstate`, acted
// on through the engine (navigation.ts decides what each change asks for). An entry the reader's
// own history made (history.ts) goes back to its exact place instead. The remote address
// stays in the fragment only: it is never put in a path, a query or the window's title. When the
// reader lets a remote file go (closed, or replaced by a local file), or leaves an unusable link's
// page, the `url` key is removed with replaceState, so a reload doesn't open it again and no
// history entry is added. A change that would close the document asks the close guard first; when
// the reader keeps the document (Cancel in the unsaved-changes dialog), the `url` key goes back to
// the address it holds, so the address still names the open file.

import type { ReaderState } from "@laminapdf/ui";
import type { Engine } from "../session.ts";
import { followHistory } from "./history.ts";
import { fragmentAction } from "./navigation.ts";
import { remoteUrlOf } from "./remote-file.ts";
import { buildFragment, parseFragment, remoteUrl } from "./url-fragment.ts";

/** Development builds accept `http://localhost` links (a local test server); production never does. */
const ALLOW_LOCAL_HTTP = import.meta.env.DEV;

/** The remote address the reader holds (opening, asking for a password, open or failed). */
function heldUrl(state: ReaderState): string | undefined {
  return state.phase === "empty" ? undefined : remoteUrlOf(state.file);
}

/** Starts following `win`'s fragment for `engine`; returns the function that stops it. */
export function watchFragment(win: Window, engine: Engine): () => void {
  const { reader } = engine;
  const history = followHistory(win, engine);
  /** Sets the fragment's `url` key to `url` (none: removed) with replaceState, adding no history entry. */
  const writeUrl = (url: string | undefined) => {
    const fragment = parseFragment(win.location.hash);
    if (remoteUrl(fragment, ALLOW_LOCAL_HTTP).kind === "none" && url === undefined) return;
    const next = buildFragment(fragment, { url });
    const { pathname, search } = win.location;
    win.history.replaceState(
      win.history.state,
      "",
      next === "" ? `${pathname}${search}` : `${pathname}${search}#${next}`,
    );
  };
  /** After the close guard kept the document: the address names it again. */
  const kept = (closed: boolean) => {
    if (!closed) writeUrl(heldUrl(reader.getState()));
  };
  const apply = () => {
    // Back or Forward to a place a jump left from or landed on: the view goes there itself.
    if (history.restore()) return;
    const action = fragmentAction(parseFragment(win.location.hash), heldUrl(reader.getState()), ALLOW_LOCAL_HTTP);
    switch (action.type) {
      case "open":
        void engine.openRemote(action.url, action.place);
        return;
      case "close":
        void reader.requestClose().then(kept);
        return;
      case "navigate":
        engine.goTo(action.place);
        return;
      case "invalid":
        void engine.showLinkProblem(action.problem).then(kept);
        return;
      case "none":
        return;
    }
  };
  const forget = () => {
    const fragment = parseFragment(win.location.hash);
    const remote = remoteUrl(fragment, ALLOW_LOCAL_HTTP);
    if (remote.kind === "none") return;
    // A usable link stays while the reader holds its file; an unusable one while its page shows.
    if (remote.kind === "url" && heldUrl(reader.getState()) === remote.url) return;
    if (remote.kind === "invalid" && engine.getState().linkProblem !== undefined) return;
    writeUrl(undefined);
  };
  apply();
  win.addEventListener("hashchange", apply);
  win.addEventListener("popstate", apply);
  // After the transitions of one call settle: a backend switch closes and reopens the same file.
  const later = () => queueMicrotask(forget);
  const unsubscribe = [reader.subscribe(later), engine.subscribe(later)];
  return () => {
    win.removeEventListener("hashchange", apply);
    win.removeEventListener("popstate", apply);
    for (const stop of unsubscribe) stop();
    history.stop();
  };
}
