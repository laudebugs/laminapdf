// The contents page dialog (O11, second artboard): where the page goes, how many levels of
// bookmarks it lists, its title and whether it gets a bookmark of its own, with a preview of the
// entries and the page labels they will show once the page is in. Create is one edit, so one undo
// step. The app reads the preview for each choice and makes the page; the dialog only asks.

import { type ReactNode, useEffect, useState } from "react";
import { Button } from "../components/button.js";
import { Checkbox, Radio, RadioGroup } from "../components/choice.js";
import { TextInput } from "../components/field.js";
import { laterMessage } from "../later-messages.js";
import { Dialog } from "../overlays/dialog.js";

/** Where the contents page goes. */
export type ContentsInsert = "start" | "after-first" | "end";

/** What the person chose in the dialog. */
export interface ContentsChoice {
  readonly insert: ContentsInsert;
  readonly levels: 1 | 2;
  /** The heading at the page's top, never empty. */
  readonly title: string;
  /** Whether the contents page gets a bookmark of its own, first in the outline. */
  readonly bookmark: boolean;
}

/** One entry of the preview: a bookmark and the label its page will show. */
export interface ContentsPreviewEntry {
  /** 1 for a top-level bookmark, 2 for one inside it. */
  readonly level: number;
  readonly title: string;
  /** The page label after the insert. */
  readonly label: string;
}

/** Props of `ContentsPageDialog`. */
export interface ContentsPageDialogProps {
  /** Whether the dialog shows. */
  readonly isOpen: boolean;
  /** The dialog asks to open or close. */
  readonly onOpenChange: (isOpen: boolean) => void;
  /** The first page's label, which names the first two places. */
  readonly firstPage: string;
  /** The entries a contents page placed as `choice` would list. */
  readonly preview: (choice: ContentsChoice) => Promise<readonly ContentsPreviewEntry[]>;
  /** Make the page; the dialog closes. */
  readonly onCreate: (choice: ContentsChoice) => void;
}

/** What the preview shows: entries, or why there are none. */
type Preview =
  | { readonly state: "reading" }
  | { readonly state: "failed" }
  | { readonly state: "ready"; readonly entries: readonly ContentsPreviewEntry[] };

/**
 * The preview for `choice`, read again while the dialog is open whenever its place or levels
 * change: the title and the bookmark don't change the entries.
 */
function usePreview(props: ContentsPageDialogProps, choice: ContentsChoice): Preview {
  const [preview, setPreview] = useState<Preview>({ state: "reading" });
  const { isOpen, preview: read } = props;
  const { insert, levels, title, bookmark } = choice;
  // biome-ignore lint/correctness/useExhaustiveDependencies: title and bookmark are passed, not followed
  useEffect(() => {
    if (!isOpen) return;
    let current = true;
    setPreview({ state: "reading" });
    read({ insert, levels, title, bookmark }).then(
      (entries) => current && setPreview({ state: "ready", entries }),
      () => current && setPreview({ state: "failed" }),
    );
    return () => {
      current = false;
    };
  }, [isOpen, read, insert, levels]);
  return preview;
}

/** The generated page in miniature: its title, then each entry with dot leaders to its label. */
function PreviewPage({ title, preview }: { readonly title: string; readonly preview: Preview }): ReactNode {
  const body =
    preview.state === "reading" ? (
      <p className="paper-contents-preview-note">{laterMessage("contents.preview.reading")}</p>
    ) : preview.state === "failed" ? (
      <p className="paper-contents-preview-note">{laterMessage("contents.preview.failed")}</p>
    ) : preview.entries.length === 0 ? (
      <p className="paper-contents-preview-note">{laterMessage("contents.preview.none")}</p>
    ) : (
      <ol className="paper-contents-preview-entries">
        {preview.entries.map((entry, i) => (
          // Titles can repeat, so the index keys them; the list is rebuilt whole each time.
          // biome-ignore lint/suspicious/noArrayIndexKey: entries have no identity of their own
          <li key={i} className="paper-contents-preview-entry" data-level={entry.level}>
            <span className="paper-contents-preview-title">{entry.title}</span>
            <span className="paper-contents-preview-leader" aria-hidden="true" />
            <span className="paper-contents-preview-label">{entry.label}</span>
          </li>
        ))}
      </ol>
    );
  return (
    <section className="paper-contents-preview" aria-label={laterMessage("contents.preview")}>
      <div className="paper-contents-preview-page">
        <p className="paper-contents-preview-heading">{title}</p>
        {body}
      </div>
    </section>
  );
}

/** The sentence saying what the insert does to the pages after it. */
function consequence(insert: ContentsInsert, firstPage: string): string {
  if (insert === "end") return laterMessage("contents.consequence.end");
  const id = insert === "start" ? "contents.consequence.before" : "contents.consequence.after";
  return laterMessage(id, { page: firstPage });
}

/**
 * The contents page dialog. Each time it opens it starts from O11's defaults: before the first
 * page, two levels, titled "Contents", with its bookmark.
 */
export function ContentsPageDialog(props: ContentsPageDialogProps): ReactNode {
  // Counts openings: a new count remounts the form, which drops the last opening's choices. It
  // changes only as the dialog opens, so a closing dialog keeps its form while it fades out.
  const [opening, setOpening] = useState(0);
  const [wasOpen, setWasOpen] = useState(props.isOpen);
  if (props.isOpen !== wasOpen) {
    setWasOpen(props.isOpen);
    if (props.isOpen) setOpening(opening + 1);
  }
  return <ContentsForm key={opening} {...props} />;
}

/** The dialog with its choices, from the defaults. */
function ContentsForm(props: ContentsPageDialogProps): ReactNode {
  const fallback = laterMessage("contents.pageTitle.default");
  const [insert, setInsert] = useState<ContentsInsert>("start");
  const [levels, setLevels] = useState<1 | 2>(2);
  const [typed, setTyped] = useState(fallback);
  const [bookmark, setBookmark] = useState(true);
  const title = typed.trim() === "" ? fallback : typed.trim();
  const choice: ContentsChoice = { insert, levels, title, bookmark };
  const preview = usePreview(props, choice);
  const canCreate = preview.state === "ready" && preview.entries.length > 0;
  return (
    <Dialog
      isOpen={props.isOpen}
      onOpenChange={props.onOpenChange}
      title={laterMessage("contents.title")}
      description={laterMessage("contents.intro")}
      closeLabel={laterMessage("contents.close")}
      size="form"
      actions={
        <>
          <Button variant="secondary" onPress={() => props.onOpenChange(false)}>
            {laterMessage("contents.cancel")}
          </Button>
          <Button
            variant="primary"
            isDisabled={!canCreate}
            onPress={() => {
              props.onCreate(choice);
              props.onOpenChange(false);
            }}
          >
            {laterMessage("contents.create")}
          </Button>
        </>
      }
    >
      <div className="paper-contents-form">
        <div className="paper-contents-fields">
          <RadioGroup
            label={laterMessage("contents.insert")}
            value={insert}
            onChange={(v) => setInsert(v as ContentsInsert)}
          >
            <Radio value="start">{laterMessage("contents.insert.start", { page: props.firstPage })}</Radio>
            <Radio value="after-first">{laterMessage("contents.insert.afterFirst", { page: props.firstPage })}</Radio>
            <Radio value="end">{laterMessage("contents.insert.end")}</Radio>
          </RadioGroup>
          <RadioGroup
            label={laterMessage("contents.levels")}
            value={String(levels)}
            onChange={(v) => setLevels(v === "1" ? 1 : 2)}
          >
            <Radio value="1">{laterMessage("contents.levels.one")}</Radio>
            <Radio value="2">{laterMessage("contents.levels.two")}</Radio>
          </RadioGroup>
          <TextInput label={laterMessage("contents.pageTitle")} value={typed} onChange={setTyped} />
          <Checkbox isSelected={bookmark} onChange={setBookmark}>
            {laterMessage("contents.bookmark", { title })}
          </Checkbox>
        </div>
        <PreviewPage title={title} preview={preview} />
      </div>
      <p className="paper-contents-consequence">{consequence(insert, props.firstPage)}</p>
    </Dialog>
  );
}
