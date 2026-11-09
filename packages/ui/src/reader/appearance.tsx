// The app's look (R10, R11, plan §5.5): the UI theme, the accent and reduced motion, set as
// attributes on the root element so switching only swaps CSS custom properties (no re-render).
// The tokens read `data-theme` (none: match the system) and `data-accent`; the shell's styles read
// `data-motion="reduce"` as they read `prefers-reduced-motion`. The page's own colours are the page
// appearance's, never these.

import { Icon } from "@laminapdf/icons/react";
import type { ReactNode } from "react";
import { Radio as AriaRadio, RadioGroup as AriaRadioGroup, Label, Text } from "react-aria-components";
import { Radio, RadioGroup, Switch } from "../components/choice.js";
import { message } from "../messages.js";
import { useMediaQuery } from "../state/hooks.js";

/** The UI themes; `system` follows the system's light or dark. */
export const UI_THEMES = ["light", "dark", "high-contrast", "system"] as const;
/** A UI theme. */
export type UiTheme = (typeof UI_THEMES)[number];

/** The accent colours (design language §2.8). */
export const ACCENTS = ["teal", "blue", "plum", "green"] as const;
/** An accent colour. */
export type Accent = (typeof ACCENTS)[number];

/** Motion: the system's setting, or reduced whatever the system says. */
export const MOTIONS = ["system", "reduce"] as const;
/** A motion setting. */
export type Motion = (typeof MOTIONS)[number];

/** The app-wide look and the keyboard setting stored beside it. */
export interface Appearance {
  readonly theme: UiTheme;
  readonly accent: Accent;
  readonly motion: Motion;
  /** Whether single-key shortcuts (Page Down, +, ?) are on (WCAG 2.1.4). */
  readonly singleKeys: boolean;
}

/** The look before anyone chooses: the system's theme, teal, the system's motion, single keys on. */
export const DEFAULT_APPEARANCE: Appearance = { theme: "system", accent: "teal", motion: "system", singleKeys: true };

const oneOf = <T extends string>(values: readonly T[], value: unknown, fallback: T): T =>
  values.includes(value as T) ? (value as T) : fallback;

/** An appearance from stored JSON; anything missing or unknown takes its default. */
export function parseAppearance(stored: string | null): Appearance {
  let value: Record<string, unknown> = {};
  try {
    const parsed: unknown = stored === null ? {} : JSON.parse(stored);
    if (typeof parsed === "object" && parsed !== null) value = parsed as Record<string, unknown>;
  } catch {
    // A damaged entry is the default look.
  }
  const d = DEFAULT_APPEARANCE;
  return {
    theme: oneOf(UI_THEMES, value.theme, d.theme),
    accent: oneOf(ACCENTS, value.accent, d.accent),
    motion: oneOf(MOTIONS, value.motion, d.motion),
    singleKeys: typeof value.singleKeys === "boolean" ? value.singleKeys : d.singleKeys,
  };
}

/** Sets `appearance` on `root` (the document element): what the tokens and styles read. */
export function applyAppearance(root: HTMLElement, appearance: Appearance): void {
  if (appearance.theme === "system") root.removeAttribute("data-theme");
  else root.setAttribute("data-theme", appearance.theme);
  root.setAttribute("data-accent", appearance.accent);
  if (appearance.motion === "reduce") root.setAttribute("data-motion", "reduce");
  else root.removeAttribute("data-motion");
}

/** Props of the appearance controls. */
export interface AppearanceFieldsProps {
  readonly appearance: Appearance;
  readonly onChange: (appearance: Appearance) => void;
}

/** Theme and accent (R10's first two sections): each choice applies at once. */
export function AppearanceFields({ appearance, onChange }: AppearanceFieldsProps): ReactNode {
  const dark = useMediaQuery("(prefers-color-scheme: dark)");
  const highContrast = appearance.theme === "high-contrast";
  return (
    <>
      <RadioGroup
        label={message("appearance.theme")}
        value={appearance.theme}
        onChange={(theme) => onChange({ ...appearance, theme: theme as UiTheme })}
        {...(appearance.theme === "system"
          ? { description: message(dark ? "appearance.theme.following.dark" : "appearance.theme.following.light") }
          : {})}
      >
        {UI_THEMES.map((theme) => (
          <Radio key={theme} value={theme}>
            {message(`appearance.theme.${theme}`)}
          </Radio>
        ))}
      </RadioGroup>
      <AccentGroup
        value={appearance.accent}
        isDisabled={highContrast}
        onChange={(accent) => onChange({ ...appearance, accent })}
      />
    </>
  );
}

/** The accent swatches: radios named by their colour, each drawn in its own accent. */
function AccentGroup(props: { value: Accent; isDisabled: boolean; onChange: (accent: Accent) => void }): ReactNode {
  return (
    <AriaRadioGroup
      className="paper-radio-group paper-accent-group"
      orientation="horizontal"
      value={props.value}
      isDisabled={props.isDisabled}
      onChange={(accent) => props.onChange(accent as Accent)}
    >
      <Label className="paper-field-label">{message("appearance.accent")}</Label>
      <div className="paper-accent-options">
        {ACCENTS.map((accent) => (
          // The swatch carries its own accent, so it shows that colour in any theme.
          <AriaRadio
            key={accent}
            value={accent}
            className="paper-accent-swatch"
            data-accent={accent}
            aria-label={message(`appearance.accent.${accent}`)}
          >
            <span className="paper-accent-dot" aria-hidden="true">
              {props.value === accent ? <Icon name="check" /> : null}
            </span>
          </AriaRadio>
        ))}
        <span className="paper-accent-name" aria-hidden="true">
          {message(`appearance.accent.${props.value}`)}
        </span>
      </div>
      <Text slot="description" className="paper-field-help">
        {message(props.isDisabled ? "appearance.accent.highContrast" : "appearance.accent.help")}
      </Text>
    </AriaRadioGroup>
  );
}

/** Settings' "Appearance and accessibility" section: theme, accent, motion and single-key shortcuts. */
export function AppearanceSettings({ appearance, onChange }: AppearanceFieldsProps): ReactNode {
  return (
    <div className="paper-settings-fields">
      <AppearanceFields appearance={appearance} onChange={onChange} />
      <RadioGroup
        label={message("appearance.motion")}
        value={appearance.motion}
        description={message("appearance.motion.help")}
        onChange={(motion) => onChange({ ...appearance, motion: motion as Motion })}
      >
        {MOTIONS.map((motion) => (
          <Radio key={motion} value={motion}>
            {message(`appearance.motion.${motion}`)}
          </Radio>
        ))}
      </RadioGroup>
      <Switch isSelected={appearance.singleKeys} onChange={(singleKeys) => onChange({ ...appearance, singleKeys })}>
        {message("appearance.singleKeys")}
      </Switch>
    </div>
  );
}
