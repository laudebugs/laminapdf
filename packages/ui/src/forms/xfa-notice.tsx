// The notice for a form with an XFA part (Paper P01, E8 plan §2.4): under Fill & Sign's toolbar.
// A dynamic form needs Adobe's apps and has nothing to fill; a static one fills as usual and loses
// its XFA version on the first change, or keeps it when the file is signed.

import type { ReactNode } from "react";
import { Button } from "../components/button.js";
import { Notice } from "../components/notice.js";
import { type LaterMessageId, laterMessage } from "../later-messages.js";

/**
 * What the notice says: `dynamic` (only Adobe's apps can fill it), `static` (the XFA version goes
 * with the first change), `signed` (a static form in a signed file, which keeps it) or `removed`
 * (the first change took it out).
 */
export type XfaNoticeKind = "dynamic" | "static" | "signed" | "removed";

/** Props of `XfaNotice`. */
export interface XfaNoticeProps {
  readonly kind: XfaNoticeKind;
  /** Undo the change that removed the XFA version; shown with `removed`. */
  readonly onUndo?: () => void;
}

/** The words and tone of each kind. */
const WORDS = {
  dynamic: { tone: "warning", title: "form.xfa.dynamic.title", body: "form.xfa.dynamic.body" },
  static: { tone: "neutral", title: "form.xfa.static.title", body: "form.xfa.static.body" },
  signed: { tone: "warning", title: "form.xfa.static.title", body: "form.xfa.signed.body" },
  removed: { tone: "neutral", title: "form.xfa.removed.title", body: "form.xfa.removed.body" },
} as const satisfies Record<
  XfaNoticeKind,
  { tone: "warning" | "neutral"; title: LaterMessageId; body: LaterMessageId }
>;

/** The XFA notice: a strip with the plain words for `kind`, and Undo once the XFA version is gone. */
export function XfaNotice({ kind, onUndo }: XfaNoticeProps): ReactNode {
  const words = WORDS[kind];
  return (
    <section className="paper-form-xfa" aria-label={laterMessage("form.xfa.region")}>
      <Notice tone={words.tone} title={laterMessage(words.title)}>
        {laterMessage(words.body)}
      </Notice>
      {kind === "removed" && onUndo !== undefined ? (
        <Button size={30} onPress={onUndo}>
          {laterMessage("form.xfa.undo")}
        </Button>
      ) : null}
    </section>
  );
}
