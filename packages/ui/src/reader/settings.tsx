// The app's one Settings dialog (plan §5.5, design S07's section list) and the View settings
// popover (R10). Later work adds a section instead of a dialog of its own: `register` a section
// with an id, a title and what it renders, and the dialog lists it in registration order.

import { type ReactNode, useEffect, useLayoutEffect, useRef, useState, useSyncExternalStore } from "react";
import { Button, IconButton } from "../components/button.js";
import { message } from "../messages.js";
import { Dialog } from "../overlays/dialog.js";
import { Popover, PopoverTrigger } from "../overlays/popover.js";
import { AppearanceFields, type AppearanceFieldsProps } from "./appearance.js";

/** One section of the Settings dialog. */
export interface SettingsSection {
  /** A stable id (`appearance`, `storage`). */
  readonly id: string;
  /** The section's heading and its name in the sections list. */
  readonly title: string;
  /** The section's fields. */
  readonly render: () => ReactNode;
}

/** The Settings dialog's sections, in the order they were registered. */
export class SettingsRegistry {
  #sections: readonly SettingsSection[] = [];
  readonly #listeners = new Set<() => void>();

  /** Adds `section`, or replaces the one with its id; returns a function that removes it. */
  register(section: SettingsSection): () => void {
    const at = this.#sections.findIndex((s) => s.id === section.id);
    this.#sections = at < 0 ? [...this.#sections, section] : this.#sections.map((s, i) => (i === at ? section : s));
    this.#changed();
    return () => {
      this.#sections = this.#sections.filter((s) => s !== section);
      this.#changed();
    };
  }

  /** The sections now (a new array after each change). */
  sections(): readonly SettingsSection[] {
    return this.#sections;
  }

  /** Calls `listener` after every change. */
  subscribe(listener: () => void): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  #changed(): void {
    for (const listener of this.#listeners) listener();
  }
}

/**
 * Registers a section for as long as the calling component is mounted. `render` is read when the
 * dialog draws, so it may close over the caller's latest state.
 */
export function useSettingsSection(
  registry: SettingsRegistry,
  id: string,
  title: string,
  render: () => ReactNode,
): void {
  const latest = useRef(render);
  useLayoutEffect(() => {
    latest.current = render;
  });
  useEffect(() => registry.register({ id, title, render: () => latest.current() }), [registry, id, title]);
}

/** Props of `SettingsDialog`. */
export interface SettingsDialogProps {
  readonly registry: SettingsRegistry;
  readonly isOpen: boolean;
  readonly onOpenChange: (open: boolean) => void;
}

/** The Settings dialog: a list of sections beside the chosen one's fields. */
export function SettingsDialog({ registry, isOpen, onOpenChange }: SettingsDialogProps): ReactNode {
  const sections = useSyncExternalStore(
    (listener) => registry.subscribe(listener),
    () => registry.sections(),
  );
  const [chosen, setChosen] = useState<string | undefined>(undefined);
  const shown = sections.find((s) => s.id === chosen) ?? sections[0];
  return (
    <Dialog
      isOpen={isOpen}
      onOpenChange={onOpenChange}
      title={message("settings.title")}
      closeLabel={message("settings.close")}
      size="wide"
    >
      <div className="paper-settings">
        {sections.length < 2 ? null : (
          <nav className="paper-settings-nav" aria-label={message("settings.sections")}>
            <ul>
              {sections.map((section) => (
                <li key={section.id}>
                  <Button
                    variant="ghost"
                    aria-current={section === shown ? "page" : false}
                    onPress={() => setChosen(section.id)}
                  >
                    {section.title}
                  </Button>
                </li>
              ))}
            </ul>
          </nav>
        )}
        {shown === undefined ? null : (
          <section className="paper-settings-section" aria-label={shown.title}>
            <h3 className="paper-settings-heading">{shown.title}</h3>
            {shown.render()}
          </section>
        )}
      </div>
    </Dialog>
  );
}

/** Props of `ViewSettingsButton`. */
export interface ViewSettingsButtonProps extends AppearanceFieldsProps {
  readonly isOpen: boolean;
  readonly onOpenChange: (open: boolean) => void;
  /** Opens the Settings dialog ("More settings…"). */
  readonly onMoreSettings: () => void;
}

/** The top bar's View button and its popover (R10): theme and accent, then "More settings…". */
export function ViewSettingsButton(props: ViewSettingsButtonProps): ReactNode {
  return (
    <PopoverTrigger isOpen={props.isOpen} onOpenChange={props.onOpenChange}>
      <IconButton icon="view-settings" label={message("command.view.settings")} aria-haspopup="dialog" />
      <Popover aria-label={message("view.title")} placement="bottom end">
        <div className="paper-view-settings">
          <h2 className="paper-view-settings-title">{message("view.title")}</h2>
          <AppearanceFields appearance={props.appearance} onChange={props.onChange} />
          <div className="paper-view-settings-footer">
            <span className="paper-field-help">{message("view.local")}</span>
            <Button
              variant="ghost"
              onPress={() => {
                props.onOpenChange(false);
                props.onMoreSettings();
              }}
            >
              {message("view.more")}
            </Button>
          </div>
        </div>
      </Popover>
    </PopoverTrigger>
  );
}
