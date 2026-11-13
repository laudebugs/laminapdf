// The link editor (Paper O12): where a link goes (a page of the file, a named destination, a web
// address or an email address), its description, and Follow link and Delete link. A change is
// committed when it is complete and valid: a radio or a name at once, a typed field on Enter or
// when it loses focus. An address that isn't an `http` or `https` one is refused in place, and the
// link keeps the target it had. A link read from a file that runs a script is shown as such, and
// can only be pointed somewhere else or deleted. The panel floats by the link on the page.

import { SPACE } from "@laminapdf/tokens";
import {
  type FocusEvent,
  type KeyboardEvent,
  type ReactNode,
  type RefObject,
  useEffect,
  useRef,
  useState,
} from "react";
import { Radio } from "react-aria-components";
import { createPortal } from "react-dom";
import { Button } from "../components/button.js";
import { RadioGroup } from "../components/choice.js";
import { Select, TextArea, TextInput } from "../components/field.js";
import { Notice } from "../components/notice.js";
import { type LaterMessageId, laterMessage } from "../later-messages.js";
import { useAnchoredPlacement } from "../overlays/anchored.js";
import { LayerScope } from "../overlays/layers.js";
import { type Box, placeBelow } from "../overlays/place.js";

/** The link editor's words an app needs: the tool, Make link, and the history's step labels. */
export type LinkWord = Extract<LaterMessageId, `link.${string}` | `edit.link.${string}`>;

/** The link editor's word `id`, filled in with `values`. */
export function linkWords(id: LinkWord, values: Readonly<Record<string, string>> = {}): string {
  return laterMessage(id, values);
}

/** Where a link goes, as the editor shows it. */
export type LinkGoal =
  /** Page `page` of this file (0-based). */
  | { readonly kind: "page"; readonly page: number }
  /** A named destination. */
  | { readonly kind: "named"; readonly name: string }
  /** An `http` or `https` address. */
  | { readonly kind: "web"; readonly address: string }
  /** An email address, with an optional subject. */
  | { readonly kind: "email"; readonly address: string; readonly subject: string }
  /** A link read from a file that runs a script (`JavaScript` or another action), never run. */
  | { readonly kind: "script"; readonly action: string };

/** The kinds of goal the editor offers. */
type Kind = Exclude<LinkGoal["kind"], "script">;

/** Props of `LinkEditor`. */
export interface LinkEditorProps {
  /** The linked text, for a link made on text; a drawn area has none. */
  readonly text?: string;
  /** Where it goes; undefined for a new link that doesn't go anywhere yet. */
  readonly goal: LinkGoal | undefined;
  /** The file's page count. */
  readonly pageCount: number;
  /** The file's named destinations. */
  readonly names: readonly string[];
  /** Its description (`/Contents`). */
  readonly contents: string;
  /** A new goal, complete and valid. */
  readonly onGoal: (goal: LinkGoal) => void;
  /** A new description; empty removes it. */
  readonly onContents: (contents: string) => void;
  /** Follows the link; left out while it goes nowhere or runs a script. */
  readonly onFollow?: () => void;
  /** Deletes the link (a new one is dropped). */
  readonly onDelete: () => void;
}

/** An `http` or `https` address with a host. */
const WEB = /^https?:\/\/[^\s/?#]+/i;
/** A scheme that would run a script. */
const SCRIPT = /^\s*(javascript|vbscript|data):/i;
/** An email address, loosely: something@something, no spaces. */
const EMAIL = /^[^\s@]+@[^\s@]+$/;

/** The parts of a `mailto:` address. */
export function mailParts(uri: string): { address: string; subject: string } {
  const rest = uri.replace(/^mailto:/i, "");
  const [address = "", query = ""] = rest.split("?", 2);
  const subject = new URLSearchParams(query).get("subject") ?? "";
  return { address: decodeURIComponent(address), subject };
}

/** A `mailto:` address for `address` and `subject`. */
export function mailUri(address: string, subject: string): string {
  return subject === "" ? `mailto:${address}` : `mailto:${address}?subject=${encodeURIComponent(subject)}`;
}

/** The editor's fields. */
export function LinkEditor(props: LinkEditorProps): ReactNode {
  const { goal } = props;
  const [kind, setKind] = useState<Kind>(goal === undefined || goal.kind === "script" ? "web" : goal.kind);
  return (
    <div className="paper-link-editor">
      <p className="paper-link-context">
        {props.text === undefined ? laterMessage("link.area") : laterMessage("link.text", { text: props.text })}
      </p>
      {goal?.kind === "script" ? <ScriptGoal action={goal.action} /> : null}
      <RadioGroup label={laterMessage("link.goesTo")} value={kind} onChange={(v) => setKind(v as Kind)}>
        <Radio value="page" className="paper-radio">
          {laterMessage("link.page")}
        </Radio>
        <Radio value="named" className="paper-radio">
          {laterMessage("link.named")}
        </Radio>
        <Radio value="web" className="paper-radio">
          {laterMessage("link.web")}
        </Radio>
        <Radio value="email" className="paper-radio">
          {laterMessage("link.email")}
        </Radio>
      </RadioGroup>
      <GoalFields kind={kind} {...props} />
      <TextArea
        label={laterMessage("link.description")}
        description={laterMessage("link.descriptionHelp")}
        defaultValue={props.contents}
        onBlur={(e) => {
          const { value } = e.target;
          if (value !== props.contents) props.onContents(value);
        }}
      />
      <div className="paper-link-actions">
        <Button
          variant="secondary"
          size={30}
          icon="external-link"
          isDisabled={props.onFollow === undefined}
          onPress={() => props.onFollow?.()}
        >
          {laterMessage("link.follow")}
        </Button>
        <Button variant="danger" size={30} icon="delete" onPress={props.onDelete}>
          {laterMessage("link.delete")}
        </Button>
      </div>
    </div>
  );
}

/** The flagged script link (O12): what it runs, in `mono`, and that Lamina PDF never runs it. */
function ScriptGoal({ action }: { action: string }): ReactNode {
  return (
    <div className="paper-link-script">
      <p className="paper-link-script-action">{laterMessage("link.script", { action })}</p>
      <Notice tone="danger" title={laterMessage("link.scriptNotice")} />
    </div>
  );
}

/** The fields of the chosen kind of goal. */
function GoalFields({ kind, ...props }: LinkEditorProps & { kind: Kind }): ReactNode {
  const { goal } = props;
  switch (kind) {
    case "page":
      return (
        <PageField
          pageCount={props.pageCount}
          page={goal?.kind === "page" ? goal.page : undefined}
          onPage={(page) => props.onGoal({ kind: "page", page })}
        />
      );
    case "named":
      return props.names.length === 0 ? (
        <p className="paper-link-hint">{laterMessage("link.noNames")}</p>
      ) : (
        <Select
          label={laterMessage("link.name")}
          options={props.names.map((name) => ({ value: name, label: name }))}
          value={goal?.kind === "named" ? goal.name : ""}
          onChange={(name) => props.onGoal({ kind: "named", name })}
        />
      );
    case "web":
      return (
        <WebField
          address={goal?.kind === "web" ? goal.address : ""}
          onAddress={(address) => props.onGoal({ kind: "web", address })}
        />
      );
    case "email":
      return (
        <EmailFields
          goal={goal?.kind === "email" ? goal : undefined}
          onEmail={(address, subject) => props.onGoal({ kind: "email", address, subject })}
        />
      );
  }
}

/** Calls `commit` with the field's value on Enter and when it loses focus. */
function committing(commit: (value: string) => void) {
  return {
    onBlur: (e: FocusEvent) => commit((e.target as HTMLInputElement).value),
    onKeyDown: (e: KeyboardEvent) => {
      if (e.key === "Enter") commit((e.target as HTMLInputElement).value);
    },
  };
}

function PageField({
  pageCount,
  page,
  onPage,
}: {
  pageCount: number;
  page: number | undefined;
  onPage: (page: number) => void;
}): ReactNode {
  const [error, setError] = useState<string>();
  const commit = (value: string) => {
    const n = Number(value.trim());
    if (!Number.isInteger(n) || n < 1 || n > pageCount) {
      setError(laterMessage("link.pageInvalid", { count: pageCount }));
      return;
    }
    setError(undefined);
    if (n - 1 !== page) onPage(n - 1);
  };
  return (
    <TextInput
      label={laterMessage("link.pageNumber")}
      description={laterMessage("link.pageHelp", { count: pageCount })}
      inputMode="numeric"
      defaultValue={page === undefined ? "" : String(page + 1)}
      {...(error === undefined ? {} : { errorMessage: error })}
      {...committing(commit)}
    />
  );
}

function WebField({ address, onAddress }: { address: string; onAddress: (address: string) => void }): ReactNode {
  const [error, setError] = useState<string>();
  const commit = (value: string) => {
    const typed = value.trim();
    if (typed === "" || typed === address) return setError(undefined);
    if (!WEB.test(typed)) {
      setError(laterMessage(SCRIPT.test(typed) ? "link.addressRefused" : "link.addressInvalid"));
      return;
    }
    setError(undefined);
    onAddress(typed);
  };
  return (
    <>
      <TextInput
        label={laterMessage("link.address")}
        placeholder="https://"
        inputMode="url"
        defaultValue={address}
        {...(error === undefined ? {} : { errorMessage: error })}
        {...committing(commit)}
      />
      <p className="paper-link-hint">{laterMessage("link.webNote")}</p>
    </>
  );
}

function EmailFields({
  goal,
  onEmail,
}: {
  goal: { address: string; subject: string } | undefined;
  onEmail: (address: string, subject: string) => void;
}): ReactNode {
  const [error, setError] = useState<string>();
  const address = useRef(goal?.address ?? "");
  const subject = useRef(goal?.subject ?? "");
  const commit = () => {
    if (address.current === "") return setError(undefined);
    if (!EMAIL.test(address.current)) {
      setError(laterMessage("link.emailInvalid"));
      return;
    }
    setError(undefined);
    if (address.current !== goal?.address || subject.current !== goal?.subject) {
      onEmail(address.current, subject.current);
    }
  };
  return (
    <>
      <TextInput
        label={laterMessage("link.emailAddress")}
        inputMode="email"
        defaultValue={address.current}
        {...(error === undefined ? {} : { errorMessage: error })}
        {...committing((v) => {
          address.current = v.trim();
          commit();
        })}
      />
      <TextInput
        label={laterMessage("link.subject")}
        defaultValue={subject.current}
        {...committing((v) => {
          subject.current = v;
          commit();
        })}
      />
    </>
  );
}

/** Props of `LinkPanel`. */
export interface LinkPanelProps {
  /** The link's box in the viewport, or null while it is out of sight. */
  readonly anchor: () => Box | null;
  /** Escape, or the panel's own Done: the app closes it. */
  readonly onDismiss: () => void;
  /** The editor. */
  readonly children: ReactNode;
}

/** The gap between the link and the panel. */
const GAP = SPACE["8"];

/**
 * The link editor's panel (O12's inspector, by the link while the reader has no Edit mode): below
 * the link, or above it when there is no room, kept there as the page scrolls. Focus moves into it
 * when it opens; Escape closes it.
 */
export function LinkPanel({ anchor, onDismiss, children }: LinkPanelProps): ReactNode {
  const ref = useRef<HTMLElement>(null);
  const place = useAnchoredPlacement(ref, { anchor, place: placeBelow, gap: GAP });
  useFocusOnOpen(ref);
  return createPortal(
    <LayerScope dismiss={onDismiss}>
      <section
        ref={ref}
        role="dialog"
        aria-label={laterMessage("link.panel")}
        className="paper-link-panel"
        data-side={place?.side}
        style={place === null ? { visibility: "hidden" } : { top: place.top, left: place.left }}
      >
        <header className="paper-link-panel-header">
          <h2 className="paper-link-panel-title">{laterMessage("link.panel")}</h2>
          <Button variant="ghost" size={30} onPress={onDismiss}>
            {laterMessage("link.done")}
          </Button>
        </header>
        {children}
      </section>
    </LayerScope>,
    document.body,
  );
}

/** Puts focus on the panel's first control once, when it opens. */
function useFocusOnOpen(ref: RefObject<HTMLElement | null>): void {
  useEffect(() => {
    ref.current?.querySelector<HTMLElement>("input, textarea, button")?.focus();
  }, [ref]);
}
