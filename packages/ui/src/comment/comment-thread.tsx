// One comment thread (Paper C04, C08): a header button with the type mark, author, type, date and
// status that goes to the comment; its quoted text, note and reply count; and, open, its replies,
// a Reply field, Mark as checked and the Status menu. The Comments panel lists them, and the
// phone's comment sheet shows the one selected.

import { Icon } from "@laminapdf/icons/react";
import { type ReactNode, useState } from "react";
import { Badge, type BadgeTone } from "../components/badge.js";
import { Button } from "../components/button.js";
import { Checkbox } from "../components/choice.js";
import { Select, TextArea } from "../components/field.js";
import { laterCountMessage, laterMessage } from "../later-messages.js";

/** A thread's status: open (none set), or the latest review state ("Done" is `Completed`). */
export type CommentStatus = "open" | "accepted" | "rejected" | "cancelled" | "done";

/** The statuses the Status menu sets, "None" clearing it. */
export const STATUS_CHOICES = ["none", "accepted", "rejected", "cancelled", "done"] as const;

/** A status the Status menu sets. */
export type StatusChoice = (typeof STATUS_CHOICES)[number];

/** One reply, as a thread lists it. */
export interface CommentReplyItem {
  readonly id: string;
  readonly author: string | undefined;
  /** When it was written, as the reader shows dates. */
  readonly when: string | undefined;
  readonly text: string;
}

/** One thread, ready to show. */
export interface CommentThreadItem {
  /** The comment's annotation ID. */
  readonly id: string;
  /** Its type, as people call it: "Highlight", "Rectangle". */
  readonly kindLabel: string;
  /** The type mark's colour, a CSS colour; none for a mark without one. */
  readonly colour: string | undefined;
  readonly author: string | undefined;
  readonly when: string | undefined;
  readonly status: CommentStatus;
  /** Who set the status. */
  readonly statusBy: string | undefined;
  /** Mark as checked. */
  readonly checked: boolean;
  /** For text markup, the text it marks. */
  readonly quoted: string | undefined;
  readonly note: string;
  readonly replies: readonly CommentReplyItem[];
}

/** What an open thread does. */
export interface CommentThreadActions {
  /** Adds a reply with `text`. */
  readonly onReply: (id: string, text: string) => void;
  /** Sets the status, "none" clearing it. */
  readonly onStatus: (id: string, status: StatusChoice) => void;
  /** Ticks or unticks Mark as checked. */
  readonly onChecked: (id: string, checked: boolean) => void;
}

const TONES: Record<Exclude<CommentStatus, "open">, BadgeTone> = {
  accepted: "positive",
  rejected: "danger",
  cancelled: "neutral",
  done: "accent",
};

const STATUS_ICONS = { accepted: "check", rejected: "close", done: "check", cancelled: undefined } as const;

/** A thread's status badge; an open thread has none (C04, design language §7.23). */
export function StatusBadge({ status }: { status: CommentStatus }): ReactNode {
  if (status === "open") return null;
  const icon = STATUS_ICONS[status];
  return (
    <Badge tone={TONES[status]} {...(icon === undefined ? {} : { icon })}>
      {laterMessage(`comments.status.${status}`)}
    </Badge>
  );
}

/** A person's initials for a reply's avatar: "Ada Lovelace" is "AL". */
function initials(name: string | undefined): string {
  const words = (name ?? "").trim().split(/\s+/).filter(Boolean);
  return words.length === 0
    ? "?"
    : words
        .map((w) => w[0]?.toLocaleUpperCase() ?? "")
        .slice(0, 2)
        .join("");
}

/** A thread's header and body: what a list row and the phone's sheet show of it. */
export function CommentThreadSummary({ thread }: { thread: CommentThreadItem }): ReactNode {
  return (
    <>
      {thread.quoted === undefined || thread.quoted === "" ? null : (
        <blockquote className="paper-comment-thread-quote">{thread.quoted}</blockquote>
      )}
      {thread.note === "" ? null : <p className="paper-comment-thread-note">{thread.note}</p>}
      {thread.status === "open" || thread.statusBy === undefined ? null : (
        <p className="paper-comment-thread-by">
          {laterMessage("comments.status.by", {
            status: laterMessage(`comments.status.${thread.status}`),
            name: thread.statusBy,
          })}
        </p>
      )}
    </>
  );
}

/** The header line: type mark, author, type, date and status. */
export function CommentThreadHeading({ thread }: { thread: CommentThreadItem }): ReactNode {
  return (
    <span className="paper-comment-thread-heading">
      <span
        className="paper-comment-thread-mark"
        aria-hidden="true"
        style={thread.colour === undefined ? undefined : { background: thread.colour }}
      />
      <span className="paper-comment-thread-author">{thread.author ?? laterMessage("comments.unnamed")}</span>
      <span className="paper-comment-thread-meta">
        {thread.kindLabel}
        {thread.when === undefined ? null : ` · ${thread.when}`}
      </span>
      <StatusBadge status={thread.status} />
    </span>
  );
}

/** An open thread's replies, Reply field, Mark as checked and Status menu. */
export function CommentThreadDetail({
  thread,
  actions,
  focusReply = false,
}: {
  thread: CommentThreadItem;
  actions: CommentThreadActions;
  /** Whether the Reply field takes the focus as it appears: Reply asked for it (C04). */
  focusReply?: boolean;
}): ReactNode {
  const [reply, setReply] = useState("");
  const send = () => {
    const text = reply.trim();
    if (text === "") return;
    actions.onReply(thread.id, text);
    setReply("");
  };
  return (
    <div className="paper-comment-thread-detail">
      {thread.replies.length === 0 ? null : (
        <ol className="paper-comment-thread-replies">
          {thread.replies.map((r) => (
            <li key={r.id} className="paper-comment-reply">
              <span className="paper-comment-reply-avatar" aria-hidden="true">
                {initials(r.author)}
              </span>
              <span className="paper-comment-reply-body">
                <span className="paper-comment-reply-meta">
                  <span className="paper-comment-thread-author">{r.author ?? laterMessage("comments.unnamed")}</span>
                  {r.when === undefined ? null : <span className="paper-comment-thread-meta">{r.when}</span>}
                </span>
                <span className="paper-comment-reply-text">{r.text}</span>
              </span>
            </li>
          ))}
        </ol>
      )}
      <div className="paper-comment-thread-reply">
        <TextArea
          label={
            thread.author === undefined
              ? laterMessage("comments.reply.field.anonymous")
              : laterMessage("comments.reply.field", { author: thread.author })
          }
          value={reply}
          onChange={setReply}
          autoFocus={focusReply}
          onKeyDown={(e) => {
            // Mod+Enter sends, as a message field does; Enter alone starts a new line.
            if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) send();
          }}
        />
        <Button size={32} onPress={send} isDisabled={reply.trim() === ""}>
          {laterMessage("comments.reply")}
        </Button>
      </div>
      <Checkbox isSelected={thread.checked} onChange={(on) => actions.onChecked(thread.id, on)}>
        {laterMessage("comments.checked")}
      </Checkbox>
      <Select
        label={laterMessage("comments.status")}
        options={STATUS_CHOICES.map((s) => ({ value: s, label: laterMessage(`comments.status.${s}`) }))}
        value={thread.status === "open" ? "none" : thread.status}
        onChange={(s) => actions.onStatus(thread.id, s)}
      />
    </div>
  );
}

/** The reply count under a closed thread: "2 replies". */
export function ReplyCount({ count }: { count: number }): ReactNode {
  if (count === 0) return null;
  return (
    <span className="paper-comment-thread-count">
      <Icon name="comment" size={16} />
      {laterCountMessage(count, { one: "comments.replies.one", many: "comments.replies" })}
    </span>
  );
}
