// The app's one Settings dialog (plan §5.5, design S07's section list) and the View settings
// popover (R10). Later work adds a section instead of a dialog of its own: `register` a section
// with an id, a title and what it renders, and the dialog lists it in registration order. On a
// phone the list is a screen of its own, and each section opens as the next screen.

import { Icon } from "@laminapdf/icons/react";
import { type ReactNode, useEffect, useId, useLayoutEffect, useRef, useState, useSyncExternalStore } from "react";
import { Button, IconButton } from "../components/button.js";
import { message } from "../messages.js";
import { Dialog } from "../overlays/dialog.js";
import { Popover, PopoverTrigger } from "../overlays/popover.js";
import { usePhone } from "../shell/phone.js";
import { AppearanceFields, type AppearanceFieldsProps } from "./appearance.js";

/** One section of the Settings dialog. */
export interface SettingsSection {
  /** A stable id (`appearance`, `storage`). */
  readonly id: string;
  /** The section's heading and its name in the sections list. */
  readonly title: string;
  /** The section's fields. */
  readonly render: () => ReactNode;
  /** One line under its name in a phone's list of sections, saying what it is set to ("Match system · Green"). */
  readonly summary?: () => string;
}

/** The Settings dialog's sections, in the order they were registered. */
export class SettingsRegistry {
  #sections: readonly SettingsSection[] = [];
  readonly #listeners = new Set<() => void>();

  /** Who registered each section now: its remover removes it only while it is still theirs. */
  readonly #owners = new Map<string, symbol>();

  /** Adds `section`, or replaces the one with its id; returns a function that removes it. */
  register(section: SettingsSection): () => void {
    const owner = Symbol(section.id);
    this.#owners.set(section.id, owner);
    this.#put(section);
    return () => {
      if (this.#owners.get(section.id) !== owner) return;
      this.#owners.delete(section.id);
      this.#sections = this.#sections.filter((s) => s.id !== section.id);
      this.#changed();
    };
  }

  /**
   * Has the dialog draw section `id` again (a new object, so nothing drawn before is reused): its
   * fields close over state that changed.
   */
  refresh(id: string): void {
    const section = this.#sections.find((s) => s.id === id);
    if (section !== undefined) this.#put({ ...section });
  }

  #put(section: SettingsSection): void {
    const at = this.#sections.findIndex((s) => s.id === section.id);
    this.#sections = at < 0 ? [...this.#sections, section] : this.#sections.map((s, i) => (i === at ? section : s));
    this.#changed();
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

/** What `useSettingsSection` may add to a section besides its fields. */
export interface SettingsSectionOptions {
  /**
   * The values its fields show: when one changes, the dialog draws the section again, so a switch
   * shows the state it was just set to.
   */
  readonly shows?: readonly unknown[];
  /** The line under its name in a phone's list of sections, read when the dialog draws. */
  readonly summary?: () => string;
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
  { shows = [], summary }: SettingsSectionOptions = {},
): void {
  const latest = useRef(render);
  const latestSummary = useRef(summary);
  useLayoutEffect(() => {
    latest.current = render;
    latestSummary.current = summary;
  });
  const hasSummary = summary !== undefined;
  useEffect(
    () =>
      registry.register({
        id,
        title,
        render: () => latest.current(),
        ...(hasSummary ? { summary: () => latestSummary.current?.() ?? "" } : {}),
      }),
    [registry, id, title, hasSummary],
  );
  // biome-ignore lint/correctness/useExhaustiveDependencies: `shows` is the caller's list of what the section shows.
  useEffect(() => registry.refresh(id), [registry, id, ...shows]);
}

/** Props of `SettingsDialog`. */
export interface SettingsDialogProps {
  /** The sections to show. */
  readonly registry: SettingsRegistry;
  /** Whether the dialog is open. */
  readonly isOpen: boolean;
  /** Called with false when the dialog closes. */
  readonly onOpenChange: (open: boolean) => void;
}

/** Props of `SettingsRow`. */
interface SettingsRowProps {
  readonly section: SettingsSection;
  /** Whether it is the section shown beside the list (never on a phone's list screen). */
  readonly current: boolean;
  /** The id its summary takes, which names it as the row's description. */
  readonly summaryId: string;
  readonly onPress: () => void;
}

/** One row of the sections list: the name, and on a phone its summary and a chevron. */
function SettingsRow({ section, current, summaryId, onPress }: SettingsRowProps): ReactNode {
  return (
    <Button
      variant="ghost"
      aria-current={current ? "page" : false}
      {...(section.summary === undefined ? {} : { "aria-describedby": summaryId })}
      onPress={onPress}
    >
      <span className="paper-settings-nav-name">{section.title}</span>
      {section.summary === undefined ? null : (
        // Hidden from the name: the row's description reads it instead, so the name stays short.
        <span id={summaryId} className="paper-settings-summary" aria-hidden="true">
          {section.summary()}
        </span>
      )}
      <Icon name="chevron-right" size={16} className="paper-settings-chevron" />
    </Button>
  );
}

/**
 * The Settings dialog: a list of sections beside the chosen one's fields. On a phone's sheet the
 * list is the first screen, with each section's summary, and a section opens as the next screen
 * with a way back (settings.css shows one screen at a time from `data-screen`).
 */
export function SettingsDialog({ registry, isOpen, onOpenChange }: SettingsDialogProps): ReactNode {
  const sections = useSyncExternalStore(
    (listener) => registry.subscribe(listener),
    () => registry.sections(),
  );
  const [chosen, setChosen] = useState<string | undefined>(undefined);
  // Whether a phone shows the chosen section rather than the list; each opening starts at the list.
  const [opened, setOpened] = useState(false);
  useEffect(() => {
    if (!isOpen) setOpened(false);
  }, [isOpen]);
  const shown = sections.find((s) => s.id === chosen) ?? sections[0];
  const listed = sections.length > 1;
  const summaryId = useId();
  // The phone's two screens; wider, the list stays beside the fields and nothing here applies.
  const phone = usePhone();
  // Where focus goes once a phone's screen changes: the hidden screen took the pressed button with it.
  const back = useRef<HTMLDivElement>(null);
  const rows = useRef(new Map<string, HTMLLIElement>());
  const focusNext = useRef<"back" | "row" | undefined>(undefined);
  // After every render: only a press sets `focusNext`, and the screen it asks for is drawn by then.
  useEffect(() => {
    const next = focusNext.current;
    focusNext.current = undefined;
    const target = next === "back" ? back.current : next === "row" && shown ? rows.current.get(shown.id) : undefined;
    target?.querySelector("button")?.focus();
  });
  return (
    <Dialog
      isOpen={isOpen}
      onOpenChange={onOpenChange}
      title={message("settings.title")}
      closeLabel={message("settings.close")}
      size="wide"
    >
      <div className="paper-settings" data-screen={listed && !opened ? "list" : "section"}>
        {listed ? (
          <nav className="paper-settings-nav" aria-label={message("settings.sections")}>
            <ul>
              {sections.map((section) => (
                <li
                  key={section.id}
                  ref={(row) =>
                    void (row === null ? rows.current.delete(section.id) : rows.current.set(section.id, row))
                  }
                >
                  <SettingsRow
                    section={section}
                    current={!phone && section === shown}
                    summaryId={`${summaryId}-${section.id}`}
                    onPress={() => {
                      setChosen(section.id);
                      if (!phone) return;
                      setOpened(true);
                      focusNext.current = "back";
                    }}
                  />
                </li>
              ))}
            </ul>
          </nav>
        ) : null}
        {shown === undefined ? null : (
          <section className="paper-settings-section" aria-label={shown.title}>
            {listed ? (
              <div ref={back} className="paper-settings-back">
                <Button
                  variant="ghost"
                  icon="chevron-left"
                  onPress={() => {
                    setOpened(false);
                    focusNext.current = "row";
                  }}
                >
                  {message("settings.back")}
                </Button>
              </div>
            ) : null}
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
  /** Whether the popover is open. */
  readonly isOpen: boolean;
  /** Called when the button or Escape opens or closes the popover. */
  readonly onOpenChange: (open: boolean) => void;
  /** Opens the Settings dialog ("More settings…"). */
  readonly onMoreSettings: () => void;
  /** The Page appearance section (`PageAppearanceFields`), after theme and accent. */
  readonly page?: ReactNode;
}

/** The top bar's View button and its popover (R10): theme, accent, page appearance, then "More settings…". */
export function ViewSettingsButton(props: ViewSettingsButtonProps): ReactNode {
  return (
    <PopoverTrigger isOpen={props.isOpen} onOpenChange={props.onOpenChange}>
      <IconButton icon="view-settings" label={message("command.view.settings")} aria-haspopup="dialog" />
      <Popover aria-label={message("view.title")} placement="bottom end">
        <div className="paper-view-settings">
          <h2 className="paper-view-settings-title">{message("view.title")}</h2>
          <AppearanceFields appearance={props.appearance} onChange={props.onChange} />
          {props.page}
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
