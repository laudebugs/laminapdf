// The command palette's shell (§7.10): a modal over the scrim, 96 px from the top, with a search
// input that keeps focus while the arrow keys move through the list (an APG combobox:
// aria-activedescendant names the active row), rows grouped under overlines, an empty state, a
// page that replaces the list for commands that ask for arguments, and a footer of hints. The app
// owns the commands, the ranking and what a query matches; the shell only shows them.

import { Icon } from "@laminapdf/icons/react";
import type { ReactNode } from "react";
import {
  Dialog as AriaDialog,
  Autocomplete,
  Collection,
  Header,
  Input,
  ListBox,
  ListBoxItem,
  ListBoxSection,
  Modal,
  ModalOverlay,
  Text,
  TextField,
} from "react-aria-components";
import { type KeyPlatform, ShortcutChip, type ShortcutKeys } from "../components/shortcut-chip.js";
import { markModal } from "./aria-modal.js";
import { LayerScope } from "./layers.js";

/** One row of the palette. */
export interface PaletteItem {
  /** Its id, passed to `onAction`. */
  id: string;
  /** The command's title, which is also its name. */
  title: string;
  /** Where the command lives: "Edit", "View". */
  group?: string;
  /** Its shortcut, shown as a chip. */
  shortcut?: ShortcutKeys;
  /** Whether it can't run now; give `disabledReason`. */
  isDisabled?: boolean;
  /** Why it can't run now, shown in place of the group (the group stays when this is left out). */
  disabledReason?: string;
  /** The parts of the title the query matched, as [start, end) offsets: drawn in 600. */
  matches?: readonly (readonly [number, number])[];
}

/** A group of rows under an overline: "Recent", "Commands". */
export interface PaletteSection {
  /** Its id. */
  id: string;
  /** The overline. */
  title: string;
  /** Its rows, in order. */
  items: readonly PaletteItem[];
}

/** Props of `CommandPalette`. */
export interface CommandPaletteProps {
  /** Whether the palette is open. */
  isOpen: boolean;
  /** Called with `false` when Escape or the scrim closes it. */
  onOpenChange: (isOpen: boolean) => void;
  /** The palette's name, which the input takes too: "Commands". */
  "aria-label": string;
  /** The input's placeholder: "Type a command or search". */
  placeholder: string;
  /** The query. */
  query: string;
  /** Called as the query changes; the app filters and ranks, then passes the new `sections`. */
  onQueryChange: (query: string) => void;
  /** The rows to show, already filtered and ranked. */
  sections: readonly PaletteSection[];
  /** Called with a row's id when it is chosen with Enter or a click. */
  onAction: (id: string) => void;
  /** How shortcut chips are written: the platform the app runs on. */
  platform: KeyPlatform;
  /** Shown when no row matches: an `EmptyState`. */
  empty?: ReactNode;
  /** A page that replaces the list, for a command that needs arguments. */
  page?: ReactNode;
  /** The hints along the bottom: "↑↓ to move · Enter to run". */
  footer?: ReactNode;
}

const ESCAPE: ShortcutKeys = { mac: "Escape", other: "Escape" };

/** The command palette's shell. */
export function CommandPalette(props: CommandPaletteProps): ReactNode {
  const { isOpen, onOpenChange, "aria-label": label, placeholder, query, onQueryChange, sections, platform } = props;
  return (
    <ModalOverlay isOpen={isOpen} onOpenChange={onOpenChange} isDismissable className="paper-palette-overlay">
      <Modal className="paper-palette-frame">
        <AriaDialog className="paper-palette" aria-label={label} ref={markModal}>
          <LayerScope dismiss={() => onOpenChange(false)}>
            <Autocomplete inputValue={query} onInputChange={onQueryChange}>
              <TextField aria-label={label} className="paper-palette-field" autoFocus>
                <Icon name="find" size={18} />
                {/* React Aria's autocomplete leaves the combobox role and its expanded state to us (APG combobox). */}
                <Input
                  className="paper-palette-input"
                  placeholder={placeholder}
                  role="combobox"
                  aria-expanded={props.page === undefined}
                />
                <ShortcutChip keys={ESCAPE} platform={platform} />
              </TextField>
              {props.page ?? (
                <ListBox
                  className="paper-palette-list"
                  aria-label={label}
                  // A section the query emptied shows no overline, and no rows at all shows `empty`.
                  items={sections.filter((section) => section.items.length > 0)}
                  onAction={(key) => props.onAction(String(key))}
                  renderEmptyState={() => props.empty ?? null}
                >
                  {(section) => (
                    <ListBoxSection id={section.id} className="paper-palette-section">
                      <Header className="paper-palette-overline">{section.title}</Header>
                      <Collection items={section.items}>{(item) => <Row item={item} platform={platform} />}</Collection>
                    </ListBoxSection>
                  )}
                </ListBox>
              )}
            </Autocomplete>
            {props.footer === undefined ? null : <div className="paper-palette-footer">{props.footer}</div>}
          </LayerScope>
        </AriaDialog>
      </Modal>
    </ModalOverlay>
  );
}

/** One row: the title with its matches in 600, then the group (or why it is disabled) and the chip. */
function Row({ item, platform }: { item: PaletteItem; platform: KeyPlatform }): ReactNode {
  const note = item.isDisabled === true ? (item.disabledReason ?? item.group) : item.group;
  return (
    <ListBoxItem
      id={item.id}
      textValue={item.title}
      className="paper-palette-row"
      {...(item.isDisabled === true ? { isDisabled: true } : {})}
    >
      <Text slot="label" className="paper-palette-title">
        <Matched text={item.title} matches={item.matches ?? []} />
      </Text>
      {/* The group, or the reason, and the chip describe the row; display: contents lays them out in the row's grid. */}
      {note === undefined && item.shortcut === undefined ? null : (
        <Text slot="description" className="paper-palette-meta">
          {note === undefined ? null : <span className="paper-palette-group">{note}</span>}
          {item.shortcut === undefined ? null : <ShortcutChip keys={item.shortcut} platform={platform} />}
        </Text>
      )}
    </ListBoxItem>
  );
}

/** `text` with the `matches` ranges in `<mark>`, which palette.css draws in 600; overlapping ranges merge. */
function Matched({ text, matches }: { text: string; matches: readonly (readonly [number, number])[] }): ReactNode {
  const parts: ReactNode[] = [];
  let at = 0;
  for (const [start, end] of merged(matches)) {
    if (start > at) parts.push(text.slice(at, start));
    parts.push(
      <mark key={start} className="paper-palette-match">
        {text.slice(start, end)}
      </mark>,
    );
    at = end;
  }
  if (at < text.length) parts.push(text.slice(at));
  return parts;
}

/** The ranges sorted, with empty ones dropped and overlapping or touching ones joined. */
function merged(ranges: readonly (readonly [number, number])[]): [number, number][] {
  const out: [number, number][] = [];
  for (const [start, end] of [...ranges].filter(([s, e]) => e > s).sort((a, b) => a[0] - b[0])) {
    const last = out.at(-1);
    if (last !== undefined && start <= last[1]) last[1] = Math.max(last[1], end);
    else out.push([start, end]);
  }
  return out;
}
