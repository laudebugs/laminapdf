// What the reader says about a damaged file (R04). `RepairDetails` is the docked panel behind the
// status bar's "Repaired on open": the notice, one row per kind of repair (a plain sentence over
// its code), the unsupported features found, and Copy details. `OpenFailure` is the page the
// canvas holds when nothing could be recovered, with its primary action focused; `RemoteFailure`
// the page for a link the reader can't open or a host that refuses it.

import { Icon } from "@laminapdf/icons/react";
import { type ReactNode, useEffect, useId, useRef, useState } from "react";
import { Button, IconButton } from "../components/button.js";
import { Notice } from "../components/notice.js";
import { message } from "../messages.js";
import type { ReaderError } from "../state/reader-machine.js";
import { failureSentence, repairSentence } from "./codes.js";

/** One kind of repair: its warning code, how many times it happened and on which pages (1-based). */
export interface RepairKind {
  readonly code: string;
  readonly pages?: readonly number[];
}

/** Props of `RepairDetails`. */
export interface RepairDetailsProps {
  fileName: string;
  /** The repairs, one entry per warning code, in the order to show them. */
  repairs: readonly RepairKind[];
  /** Unsupported features found, each a sentence: "This file contains JavaScript. Lamina PDF never runs it." */
  notRun?: readonly string[];
  /** Closes the panel; the shell puts focus back on the status bar's notice. */
  onClose: () => void;
  /** Copies the codes, counts and version as text. */
  onCopy: () => void;
}

/** How many kinds show before "Show all". */
const SHOWN = 5;

/** The repair details panel, for the inspector slot. Escape closes it. */
export function RepairDetails({ fileName, repairs, notRun = [], onClose, onCopy }: RepairDetailsProps): ReactNode {
  const title = useId();
  const [all, setAll] = useState(false);
  const firstMore = useRef<HTMLLIElement>(null);
  const shown = all ? repairs : repairs.slice(0, SHOWN);
  // "Show all" goes away once pressed: focus moves to the first row it revealed.
  useEffect(() => {
    if (all) firstMore.current?.focus();
  }, [all]);
  return (
    <aside
      className="paper-repair"
      aria-labelledby={title}
      onKeyDown={(event) => {
        if (event.key !== "Escape") return;
        event.stopPropagation();
        onClose();
      }}
    >
      <header className="paper-repair-header">
        <div>
          <h2 id={title} className="paper-repair-title">
            {message("repair.title")}
          </h2>
          <p className="paper-repair-file">{fileName}</p>
        </div>
        <IconButton icon="close" label={message("repair.close")} onPress={onClose} />
      </header>
      <Notice tone="warning" title={message("repair.notice.title")}>
        {message("repair.notice.body")}
      </Notice>
      <h3 className="paper-repair-overline">{message("repair.done")}</h3>
      <ul className="paper-repair-list">
        {shown.map((repair, index) => (
          <li
            key={repair.code}
            className="paper-repair-row"
            {...(index === SHOWN ? { ref: firstMore, tabIndex: -1 } : {})}
          >
            <span>{repairSentence(repair.code)}</span>
            <code className="paper-repair-code">
              {repair.pages === undefined || repair.pages.length === 0
                ? repair.code
                : `${repair.code} · ${message("repair.pages", { pages: repair.pages.join(", ") })}`}
            </code>
          </li>
        ))}
      </ul>
      {all || repairs.length <= SHOWN ? null : (
        <Button variant="ghost" onPress={() => setAll(true)}>
          {message("repair.showAll", { count: repairs.length })}
        </Button>
      )}
      {notRun.length === 0 ? null : (
        <>
          <h3 className="paper-repair-overline">{message("repair.notRun")}</h3>
          <ul className="paper-repair-list">
            {notRun.map((sentence) => (
              <li key={sentence} className="paper-repair-row">
                {sentence}
              </li>
            ))}
          </ul>
        </>
      )}
      <div className="paper-repair-actions">
        <Button icon="copy" onPress={onCopy}>
          {message("repair.copy")}
        </Button>
      </div>
    </aside>
  );
}

/** Props of `OpenFailure`. */
export interface OpenFailureProps {
  /** Why the document couldn't open. */
  error: ReaderError;
  /** A note after the code, such as how much was read: "1.8 MB read". */
  detail?: string;
  /** Back to the home screen to choose another file. */
  onTryAnother: () => void;
  /** Copies the error's details as text. */
  onCopy: () => void;
}

/** The page the canvas holds when a document can't be opened; announced, its primary action focused. */
export function OpenFailure({ error, detail, onTryAnother, onCopy }: OpenFailureProps): ReactNode {
  const page = useRef<HTMLDivElement>(null);
  // Focus goes to Try another file, the primary action, whenever a new failure shows.
  // biome-ignore lint/correctness/useExhaustiveDependencies: runs again for each new failure's code
  useEffect(() => {
    page.current?.querySelector<HTMLElement>(".paper-button[data-variant='primary']")?.focus();
  }, [error.code]);
  return (
    <div ref={page} role="alert" className="paper-failure">
      <Icon name="warning" size={32} />
      <h2 className="paper-failure-title">{message("failure.title")}</h2>
      <p className="paper-failure-body">{failureSentence(error.code)}</p>
      <p className="paper-failure-code">{detail === undefined ? error.code : `${error.code} · ${detail}`}</p>
      <div className="paper-failure-actions">
        <Button variant="primary" onPress={onTryAnother}>
          {message("failure.tryAnother")}
        </Button>
        <Button icon="copy" onPress={onCopy}>
          {message("failure.copy")}
        </Button>
      </div>
    </div>
  );
}

/**
 * Why a link's document can't be opened: the host refused to let this page read it (`cors`,
 * which the browser can't tell apart from being offline), or the link's address is unusable.
 */
export type RemoteFailureReason = "cors" | "not-encoded" | "not-absolute" | "insecure" | "scheme";

/** Props of `RemoteFailure`. */
export interface RemoteFailureProps {
  reason: RemoteFailureReason;
  /** The document's address, for Download the file (`cors` only); it opens with no referrer. */
  url?: string;
  /** Tries the link again (`cors` only). */
  onRetry?: () => void;
  /** Back to the home screen to open a file instead. */
  onChooseFile: () => void;
}

/** The page the canvas holds when a link's document can't be opened; announced, its primary action focused. */
export function RemoteFailure({ reason, url, onRetry, onChooseFile }: RemoteFailureProps): ReactNode {
  const page = useRef<HTMLDivElement>(null);
  // biome-ignore lint/correctness/useExhaustiveDependencies: runs again for each new reason
  useEffect(() => {
    page.current?.querySelector<HTMLElement>(".paper-button[data-variant='primary']")?.focus();
  }, [reason]);
  const cors = reason === "cors";
  return (
    <div ref={page} role="alert" className="paper-failure">
      <Icon name="warning" size={32} />
      <h2 className="paper-failure-title">{message("remote.title")}</h2>
      <p className="paper-failure-body">{message(`remote.${reason}`)}</p>
      {cors ? <p className="paper-failure-body">{message("remote.cors.offline")}</p> : null}
      <div className="paper-failure-actions">
        {cors && url !== undefined ? (
          <a
            className="paper-button"
            data-variant="primary"
            data-size={32}
            href={url}
            target="_blank"
            rel="noreferrer noopener"
          >
            {message("remote.download")}
          </a>
        ) : null}
        {cors && onRetry !== undefined ? <Button onPress={onRetry}>{message("remote.retry")}</Button> : null}
        <Button variant={cors ? "secondary" : "primary"} onPress={onChooseFile}>
          {message("remote.chooseFile")}
        </Button>
      </div>
    </div>
  );
}
