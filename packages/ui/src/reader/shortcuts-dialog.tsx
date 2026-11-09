// Keyboard shortcuts (R12): every shortcut this shell binds, generated from the command registry
// (`useShortcutMap`), searchable, with the single-key setting (WCAG 2.1.4) and the browser keys
// the reader takes while a document is open.

import { type ReactNode, useState } from "react";
import { useCommands, useShortcutMap } from "../commands/react.js";
import { isSingleKey, OVERRIDES, shellOf } from "../commands/shortcuts.js";
import { Button } from "../components/button.js";
import { Switch } from "../components/choice.js";
import { TextInput } from "../components/field.js";
import { type KeyPlatform, ShortcutChip } from "../components/shortcut-chip.js";
import { message } from "../messages.js";
import { Dialog } from "../overlays/dialog.js";

/** Props of `ShortcutsDialog`. */
export interface ShortcutsDialogProps {
  readonly isOpen: boolean;
  readonly onOpenChange: (open: boolean) => void;
  /** Whether single-key shortcuts are on, and the switch that turns them off. */
  readonly singleKeys: boolean;
  readonly onSingleKeysChange: (on: boolean) => void;
  /** Opens the command palette, offered when nothing matches. */
  readonly onOpenPalette: () => void;
}

/** The keyboard shortcuts dialog. Render it inside the `CommandsProvider`. */
export function ShortcutsDialog(props: ShortcutsDialogProps): ReactNode {
  const { keys: platform, context } = useCommands();
  const groups = useShortcutMap();
  const [query, setQuery] = useState("");
  const needle = query.trim().toLowerCase();
  const shown = groups
    .map((g) => ({ ...g, commands: g.commands.filter((c) => c.title.toLowerCase().includes(needle)) }))
    .filter((g) => g.commands.length > 0);
  const count = shown.reduce((sum, g) => sum + g.commands.length, 0);
  const close = () => props.onOpenChange(false);
  return (
    <Dialog
      isOpen={props.isOpen}
      onOpenChange={props.onOpenChange}
      title={message("help.shortcuts.title")}
      description={message(platform === "mac" ? "help.shortcuts.mac" : "help.shortcuts.other")}
      closeLabel={message("settings.close")}
      size="wide"
    >
      <div className="paper-shortcuts">
        <TextInput label={message("help.shortcuts.search")} value={query} onChange={setQuery} />
        <p className="paper-visually-hidden" aria-live="polite">
          {message(count === 1 ? "help.shortcuts.count.one" : "help.shortcuts.count", { count })}
        </p>
        {count === 0 ? (
          <NoMatch
            query={query.trim()}
            onOpenPalette={() => {
              close();
              props.onOpenPalette();
            }}
          />
        ) : (
          <div className="paper-shortcuts-groups">
            {shown.map((group) => (
              <section key={group.group} className="paper-shortcuts-group" aria-label={group.title}>
                <h3 className="paper-shortcuts-heading">{group.title}</h3>
                <dl>
                  {group.commands.map((command) => (
                    <ShortcutRow key={command.id} command={command} platform={platform} singleKeys={props.singleKeys} />
                  ))}
                </dl>
              </section>
            ))}
          </div>
        )}
        <BrowserKeys platform={platform} shell={shellOf(context)} />
        <footer className="paper-shortcuts-footer">
          <span className="paper-field-help">{message("help.shortcuts.palette")}</span>
          <Switch isSelected={props.singleKeys} onChange={props.onSingleKeysChange}>
            {message("appearance.singleKeys")}
          </Switch>
        </footer>
      </div>
    </Dialog>
  );
}

type Row = ReturnType<typeof useShortcutMap>[number]["commands"][number];

/** One command: its name and its keys; a single key reads "Off" while single keys are off. */
function ShortcutRow({ command, platform, singleKeys }: { command: Row; platform: KeyPlatform; singleKeys: boolean }) {
  return (
    <div className="paper-shortcuts-row">
      <dt>{command.title}</dt>
      <dd>
        {command.keys.map((keys) => {
          const off = !singleKeys && isSingleKey(keys[platform], platform);
          return (
            <span key={keys[platform]} className="paper-shortcuts-keys" data-off={off || undefined}>
              <ShortcutChip keys={keys} platform={platform} />
              {off ? <span className="paper-shortcuts-off">{message("help.shortcuts.off")}</span> : null}
            </span>
          );
        })}
      </dd>
    </div>
  );
}

/** Nothing matches: say so and offer the palette. */
function NoMatch({ query, onOpenPalette }: { query: string; onOpenPalette: () => void }): ReactNode {
  return (
    <div className="paper-shortcuts-none">
      <p>{message("help.shortcuts.none", { query })}</p>
      <Button variant="ghost" onPress={onOpenPalette}>
        {message("help.shortcuts.openPalette")}
      </Button>
    </div>
  );
}

/** The browser's own keys the reader takes, and why (the help documents the decision). */
function BrowserKeys({ platform, shell }: { platform: KeyPlatform; shell: "web" | "desktop" }): ReactNode {
  if (shell !== "web") return null;
  return (
    <details className="paper-shortcuts-browser">
      <summary>{message("help.shortcuts.browser")}</summary>
      <dl>
        {OVERRIDES.web.map((rule) => (
          <div key={rule.keys} className="paper-shortcuts-row">
            <dt>
              <ShortcutChip keys={{ mac: rule.keys, other: rule.keys }} platform={platform} />
            </dt>
            <dd>{rule.why}</dd>
          </div>
        ))}
      </dl>
    </details>
  );
}
