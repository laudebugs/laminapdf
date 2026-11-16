// The Organise mode's toolbar (Paper O06, §8.1), under the top bar while Organise is the mode:
// Insert ▾ · Rotate left · Rotate right · Delete │ Move to page… │ More page actions (⋯) · Page setup ▾ · the note
// on the selection, and the thumbnail size control at the end. Each button is its command's view
// (useCommand), so it is disabled, with the reason, when the command can't run.

import type { ReactNode } from "react";
import { useCommand, useCommands } from "../commands/react.js";
import type { CommandId } from "../commands/types.js";
import { Button, IconButton } from "../components/button.js";
import type { KeyPlatform } from "../components/shortcut-chip.js";
import { laterMessage } from "../later-messages.js";
import { Menu, type MenuEntry, MenuTrigger } from "../overlays/popover.js";
import { INSERT_COMMANDS, MORE_COMMANDS, PAGE_SETUP_COMMANDS } from "./commands.js";

/** Props of `OrganiseToolbar`. */
export interface OrganiseToolbarProps {
  /** "2 pages selected", "24 pages" (dialogs.tsx's `selectionNote`). */
  readonly note: string;
  /** Thumbnail size, 0.75–2. */
  readonly scale: number;
  readonly onScale: (scale: number) => void;
  readonly platform: KeyPlatform;
}

/** The steps of the thumbnail size control, as fractions of the tile's size at 100%. */
export const THUMBNAIL_SCALES = [0.75, 1, 1.25, 1.5, 2] as const;

/** One command as a toolbar button: an icon button with its title as name, or a text button. */
function CommandButton({ id, icon }: { id: CommandId; icon?: Parameters<typeof IconButton>[0]["icon"] }): ReactNode {
  const view = useCommand(id);
  if (view.availability.state === "hidden") return null;
  const disabled = view.availability.state === "disabled";
  return icon === undefined ? (
    <Button size={30} isDisabled={disabled} onPress={() => view.run()}>
      {view.title}
    </Button>
  ) : (
    <IconButton icon={icon} label={view.title} size={32} isDisabled={disabled} onPress={() => view.run()} />
  );
}

/** A menu of commands, as Insert ▾ and More page actions are. */
function CommandMenu({
  label,
  icon,
  ids,
  platform,
}: {
  label: string;
  icon?: Parameters<typeof IconButton>[0]["icon"];
  ids: readonly CommandId[];
  platform: KeyPlatform;
}): ReactNode {
  const { registry, context } = useCommands();
  const items: MenuEntry<CommandId>[] = [];
  for (const id of ids) {
    const command = registry.get(id);
    const availability = registry.availability(id, context);
    if (command === undefined || availability.state === "hidden") continue;
    items.push({
      id,
      label: registry.title(command, context),
      ...(availability.state === "disabled" ? { isDisabled: true, disabledReason: availability.reason } : {}),
    });
  }
  return (
    <MenuTrigger>
      {icon === undefined ? (
        <Button size={30} icon="chevron-down">
          {label}
        </Button>
      ) : (
        <IconButton icon={icon} label={label} size={32} />
      )}
      <Menu
        aria-label={label}
        items={items}
        platform={platform}
        onAction={(id) => void registry.run(id, context).catch(() => undefined)}
      />
    </MenuTrigger>
  );
}

/** The toolbar. */
export function OrganiseToolbar({ note, scale, onScale, platform }: OrganiseToolbarProps): ReactNode {
  const at = THUMBNAIL_SCALES.indexOf(scale as (typeof THUMBNAIL_SCALES)[number]);
  return (
    <div className="paper-organise-toolbar" role="toolbar" aria-label={laterMessage("command.mode.organise")}>
      <CommandMenu label={laterMessage("organise.insert")} ids={INSERT_COMMANDS} platform={platform} />
      <CommandButton id="pages.rotate-left" icon="rotate-left" />
      <CommandButton id="pages.rotate-right" icon="rotate-right" />
      <CommandButton id="pages.delete" icon="delete" />
      <span className="paper-organise-divider" aria-hidden="true" />
      <CommandButton id="pages.move-to" />
      <CommandMenu label={laterMessage("organise.more")} icon="more" ids={MORE_COMMANDS} platform={platform} />
      <CommandMenu label={laterMessage("organise.setup")} ids={PAGE_SETUP_COMMANDS} platform={platform} />
      <span className="paper-organise-note" aria-live="polite">
        {note}
      </span>
      {/* biome-ignore lint/a11y/useSemanticElements: a toolbar group of buttons, as Undo and Redo's is; <fieldset> holds form controls. */}
      <span className="paper-organise-size" role="group" aria-label={laterMessage("organise.size")}>
        <IconButton
          icon="zoom-out"
          label={laterMessage("organise.size.smaller")}
          size={26}
          isDisabled={at <= 0}
          onPress={() => onScale(THUMBNAIL_SCALES[Math.max(0, at - 1)] ?? 1)}
        />
        <span className="paper-organise-size-value">{Math.round(scale * 100)}%</span>
        <IconButton
          icon="zoom-in"
          label={laterMessage("organise.size.larger")}
          size={26}
          isDisabled={at >= THUMBNAIL_SCALES.length - 1}
          onPress={() => onScale(THUMBNAIL_SCALES[Math.min(THUMBNAIL_SCALES.length - 1, at + 1)] ?? 1)}
        />
      </span>
    </div>
  );
}
