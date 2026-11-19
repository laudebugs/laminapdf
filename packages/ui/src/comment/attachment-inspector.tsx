// The attached file's fields in the annotation inspector (Paper C05): a card with the file's
// name, size, type and date added; Save file as… (a download where the browser has no save
// dialog) and Replace…; the line that Lamina PDF never opens attached files; the Description and
// the Icon. A sound comment instead shows Play, which plays its recording. Each change is one
// edit, made by the app.

import type { ReactNode } from "react";
import { Button } from "../components/button.js";
import { Select, TextArea } from "../components/field.js";
import { laterMessage } from "../later-messages.js";
import { ATTACHMENT_ICONS, type AttachmentIconName } from "./commands.js";
import type { InspectorChoice } from "./inspector.js";

/** Props of `AttachmentFields`. */
export interface AttachmentFieldsProps {
  /** The file's name. */
  readonly name: string;
  /** "12.4 KB · text/csv · Added 29 Oct 2026": what the card says under the name. */
  readonly facts: string;
  /** "Save file as…", or "Download file" where the browser saves by downloading. */
  readonly saveLabel: string;
  readonly onSave: () => void;
  readonly onReplace: () => void;
  /** `/Desc`; committed when the field loses focus. */
  readonly description: { readonly value: string; readonly onCommit: (text: string) => void };
  readonly icon: InspectorChoice<AttachmentIconName>;
  readonly disabled: boolean;
}

/** An attached file's fields. */
export function AttachmentFields(props: AttachmentFieldsProps): ReactNode {
  const { icon, description, disabled } = props;
  return (
    <div className="paper-comment-attachment">
      <div className="paper-comment-file">
        <p className="paper-comment-file-name">{props.name}</p>
        <p className="paper-comment-file-facts">{props.facts}</p>
      </div>
      <div className="paper-comment-file-actions">
        <Button size={30} icon="download" onPress={props.onSave}>
          {props.saveLabel}
        </Button>
        <Button size={30} icon="replace" isDisabled={disabled} onPress={props.onReplace}>
          {laterMessage("attachment.replace")}
        </Button>
      </div>
      <p className="paper-comment-text-note">{laterMessage("attachment.never")}</p>
      <TextArea
        key={description.value}
        label={laterMessage("attachment.description")}
        defaultValue={description.value}
        isDisabled={disabled}
        onBlur={(event) => {
          const text = (event.target as unknown as HTMLTextAreaElement).value;
          if (text !== description.value) description.onCommit(text);
        }}
      />
      <Select
        label={laterMessage("attachment.icon")}
        options={ATTACHMENT_ICONS.map((i) => ({ value: i, label: laterMessage(`attachment.icon.${i}`) }))}
        value={icon.value ?? "pushPin"}
        isDisabled={disabled}
        onChange={icon.onChange}
      />
    </div>
  );
}

/** Props of `SoundFields`. */
export interface SoundFieldsProps {
  /** "Play sound comment, 0:12". */
  readonly label: string;
  readonly playing: boolean;
  readonly onPlay: () => void;
}

/** A sound comment's Play button. */
export function SoundFields({ label, playing, onPlay }: SoundFieldsProps): ReactNode {
  return (
    <Button size={30} icon="play" isDisabled={playing} onPress={onPlay}>
      {playing ? laterMessage("sound.playing") : label}
    </Button>
  );
}
