// The stamp picker (Paper C05): the popover under Stamp ▾, with tabs Standard · Dynamic · Custom.
// Standard stamps show their names in their colours; dynamic stamps show the line the name and
// time will write; custom stamps are the images kept on this device, each with Rename and Remove,
// after Add image…. Choosing a stamp closes the picker, and the stamp then rides the pointer
// until a click places it (the app's Stamp tool). Each stamp is a button named by its full text.

import { type ReactNode, useState } from "react";
import { Button } from "../components/button.js";
import { TextInput } from "../components/field.js";
import { SegmentedTabs, TabPanel } from "../components/tabs.js";
import { laterMessage } from "../later-messages.js";
import { DYNAMIC_STAMPS, type DynamicStampName, PICKER_STAMPS, type PickerStamp } from "./commands.js";

/** A custom stamp as the picker lists it. */
export interface CustomStampItem {
  readonly id: string;
  readonly name: string;
  /** An object URL of its image, for the thumbnail. */
  readonly url: string;
}

/** What the person chose. */
export type StampChoice =
  | { readonly kind: "standard"; readonly name: PickerStamp }
  | { readonly kind: "dynamic"; readonly name: DynamicStampName }
  | { readonly kind: "custom"; readonly id: string };

/** Props of `StampPicker`. */
export interface StampPickerProps {
  /** The line a dynamic stamp writes under its title now: "by R. Osei · 29 Oct 2026, 10:14". */
  readonly dynamicLine: string;
  readonly custom: readonly CustomStampItem[];
  readonly onChoose: (choice: StampChoice) => void;
  /** Add image…: the app asks for a PNG or JPEG. */
  readonly onAdd: () => void;
  readonly onRename: (id: string, name: string) => void;
  readonly onRemove: (id: string) => void;
}

/** A stamp's tone, which picks its colour. */
type Tone = "approve" | "refuse" | "note";

/** Each standard stamp's tone, as the appearance colours it: green approves, red refuses, blue notes the rest. */
const STANDARD_TONE: Readonly<Record<PickerStamp, Tone>> = {
  approved: "approve",
  notApproved: "refuse",
  reviewed: "note",
  draft: "note",
  final: "note",
  confidential: "refuse",
  forComment: "note",
  void: "refuse",
};

const DYNAMIC_TONE: Readonly<Record<DynamicStampName, Tone>> = {
  approved: "approve",
  reviewed: "note",
  received: "note",
  notApproved: "refuse",
};

type Tab = "standard" | "dynamic" | "custom";

/** A custom stamp's row: its picture and name, with Rename (in place) and Remove. */
function CustomRow({
  stamp,
  onChoose,
  onRename,
  onRemove,
}: {
  stamp: CustomStampItem;
  onChoose: () => void;
  onRename: (name: string) => void;
  onRemove: () => void;
}): ReactNode {
  const [naming, setNaming] = useState(false);
  const [name, setName] = useState(stamp.name);
  return (
    <li className="paper-stamp-custom">
      {naming ? (
        <form
          className="paper-stamp-rename"
          onSubmit={(event) => {
            event.preventDefault();
            if (name.trim() !== "") onRename(name.trim());
            setNaming(false);
          }}
        >
          <TextInput
            label={laterMessage("stamps.custom.renameField", { name: stamp.name })}
            value={name}
            onChange={setName}
            autoFocus
          />
        </form>
      ) : (
        <Button variant="ghost" onPress={onChoose}>
          <img className="paper-stamp-thumb" src={stamp.url} alt="" />
          {stamp.name}
        </Button>
      )}
      <Button size={26} variant="ghost" onPress={() => setNaming(!naming)}>
        {laterMessage("stamps.custom.rename")}
      </Button>
      <Button size={26} variant="ghost" onPress={onRemove}>
        {laterMessage("stamps.custom.remove")}
      </Button>
    </li>
  );
}

/** The picker's content, inside the Stamp ▾ popover. */
export function StampPicker(props: StampPickerProps): ReactNode {
  const [tab, setTab] = useState<Tab>("standard");
  const tabs = (["standard", "dynamic", "custom"] as const).map((id) => ({
    id,
    label: laterMessage(`stamps.tab.${id}`),
  }));
  return (
    <div className="paper-stamp-picker">
      <SegmentedTabs tabs={tabs} selectedKey={tab} onSelectionChange={setTab} aria-label={laterMessage("stamps.tabs")}>
        <TabPanel id="standard">
          <ul className="paper-stamp-list">
            {PICKER_STAMPS.map((name) => (
              <li key={name}>
                <button
                  type="button"
                  className="paper-stamp-choice"
                  data-tone={STANDARD_TONE[name]}
                  onClick={() => props.onChoose({ kind: "standard", name })}
                >
                  {laterMessage(`stamps.standard.${name}`)}
                </button>
              </li>
            ))}
          </ul>
        </TabPanel>
        <TabPanel id="dynamic">
          <ul className="paper-stamp-list">
            {DYNAMIC_STAMPS.map((name) => (
              <li key={name}>
                <button
                  type="button"
                  className="paper-stamp-choice"
                  data-tone={DYNAMIC_TONE[name]}
                  onClick={() => props.onChoose({ kind: "dynamic", name })}
                >
                  <span className="paper-stamp-title">{laterMessage(`stamps.dynamic.${name}`)}</span>
                  <span className="paper-stamp-line">{props.dynamicLine}</span>
                </button>
              </li>
            ))}
          </ul>
          <p className="paper-stamp-note">{laterMessage("stamps.dynamic.note")}</p>
        </TabPanel>
        <TabPanel id="custom">
          <Button size={30} icon="image" onPress={props.onAdd}>
            {laterMessage("stamps.custom.add")}
          </Button>
          {props.custom.length === 0 ? (
            <p className="paper-stamp-note">{laterMessage("stamps.custom.empty")}</p>
          ) : (
            <ul className="paper-stamp-list">
              {props.custom.map((stamp) => (
                <CustomRow
                  key={stamp.id}
                  stamp={stamp}
                  onChoose={() => props.onChoose({ kind: "custom", id: stamp.id })}
                  onRename={(name) => props.onRename(stamp.id, name)}
                  onRemove={() => props.onRemove(stamp.id)}
                />
              ))}
            </ul>
          )}
          <p className="paper-stamp-note">{laterMessage("stamps.custom.note")}</p>
        </TabPanel>
      </SegmentedTabs>
    </div>
  );
}
