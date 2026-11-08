// The command palette over the registry (ADR 0024 §3): Cmd/Ctrl+K opens it; it ranks the
// commands this shell has by title and keywords, recent ones first, with each row's shortcut and
// disabled ones showing why; it parses arguments inline (`go to 12`, `zoom 150`), and a command
// that needs them and got none pushes a page that asks. The shell (`CommandPalette`) draws it.

import { type ReactNode, useState } from "react";
import { EmptyState } from "../components/empty-state.js";
import { message } from "../messages.js";
import { CommandPalette, type PaletteItem, type PaletteSection } from "../overlays/palette.js";
import { RecentCommands, rankCommands } from "./rank.js";
import { chipKeys, useCommands } from "./react.js";
import type { CommandRegistry } from "./registry.js";
import type { AnyCommand, CommandContext, CommandId } from "./types.js";

/** Props of `RegistryPalette`. */
export interface RegistryPaletteProps {
  /** Whether the palette is open. */
  readonly isOpen: boolean;
  /** Called when the palette asks to open or close: a shortcut, Escape, or a command chosen. */
  readonly onOpenChange: (isOpen: boolean) => void;
  /** Where recent commands are kept; one on this device's local storage by default. */
  readonly recent?: RecentCommands;
}

/** A row's id for an inline argument: the command's id, then what was typed after it. */
const INLINE = "::";

/** A title without its trailing ellipsis: "Go to page…" asks, "Go to page: 12" answers. */
const bare = (title: string) => title.replace(/…$/, "");

/** What follows one of `phrases` and a space at the start of `query`, without regard to case. */
function argumentText(query: string, phrases: readonly string[]): string | undefined {
  const lower = query.toLocaleLowerCase();
  for (const phrase of phrases) {
    const start = `${phrase.toLocaleLowerCase()} `;
    if (lower.startsWith(start) && query.length > start.length) return query.slice(start.length).trim();
  }
  return undefined;
}

/** The registry and the context its commands are read in. */
interface Scope {
  readonly registry: CommandRegistry;
  readonly context: CommandContext;
}

/** `command` as a palette row: disabled with its reason, or with its prompt for a secret argument. */
function paletteItem({ registry, context }: Scope, command: AnyCommand, extra: Partial<PaletteItem> = {}): PaletteItem {
  const availability = registry.availability(command.id, context);
  const shortcut = chipKeys(command, context);
  const reason =
    availability.state === "disabled"
      ? availability.reason
      : command.args?.secret === true
        ? command.args.prompt
        : undefined;
  return {
    id: command.id,
    title: registry.title(command, context),
    group: message(`commandGroup.${command.group}`),
    ...(shortcut === undefined ? {} : { shortcut }),
    ...(reason === undefined ? {} : { isDisabled: true, disabledReason: reason }),
    ...extra,
  };
}

/** A row for each command whose arguments `query` types inline: its summary, or why they don't parse. */
function inlineRows(scope: Scope, shown: readonly AnyCommand[], query: string): PaletteItem[] {
  const titles = shown.map((command) => scope.registry.title(command, scope.context).toLocaleLowerCase());
  return shown.flatMap((command) => {
    if (command.args === undefined || command.args.secret === true) return [];
    const title = scope.registry.title(command, scope.context);
    const text = argumentText(query, [bare(title), ...(command.args.prefixes ?? [])]);
    if (text === undefined) return [];
    const parsed = command.args.parse(text, scope.context);
    // "zoom in" starts Zoom in's title: that is a search for it, not arguments gone wrong.
    if (!parsed.ok && titles.some((t) => t.startsWith(query.trim().toLocaleLowerCase()))) return [];
    const row = paletteItem(scope, command, {
      id: `${command.id}${INLINE}${text}`,
      title: parsed.ok ? message("palette.argRow", { title: bare(title), summary: parsed.summary }) : title,
    });
    return [parsed.ok || row.isDisabled === true ? row : { ...row, isDisabled: true, disabledReason: parsed.error }];
  });
}

/** The palette's sections for `query`: inline-argument rows, then the ranked commands; Recent first when empty. */
function paletteSections(
  scope: Scope,
  shown: readonly AnyCommand[],
  query: string,
  recent: readonly string[],
): PaletteSection[] {
  const rankable = shown.map((command) => ({
    id: command.id,
    title: scope.registry.title(command, scope.context),
    keywords: command.keywords ?? [],
    command,
  }));
  const ranked = rankCommands(query, rankable, recent).map(({ item, ranges }) =>
    paletteItem(scope, item.command, { matches: ranges }),
  );
  const commands = message("palette.commands");
  if (query.trim() !== "")
    return [{ id: "commands", title: commands, items: [...inlineRows(scope, shown, query), ...ranked] }];
  const recents = recent.flatMap((id) =>
    shown.filter((command) => command.id === id).map((c) => paletteItem(scope, c)),
  );
  return [
    { id: "recent", title: message("palette.recent"), items: recents },
    { id: "commands", title: commands, items: ranked },
  ];
}

/** A row id as the command's id and, for an inline-argument row, the text typed after it. */
function parseRowId(id: string): [CommandId, string | undefined] {
  const at = id.indexOf(INLINE);
  return at < 0 ? [id as CommandId, undefined] : [id.slice(0, at) as CommandId, id.slice(at + INLINE.length)];
}

/** The page a command that needs arguments pushes: its title, what to type, and why the last try failed. */
function ArgumentsPage({
  title,
  prompt,
  error,
}: {
  title: string;
  prompt: string;
  error: string | undefined;
}): ReactNode {
  return (
    <div className="paper-palette-page">
      <p className="paper-palette-page-title">{title}</p>
      <p className="paper-palette-page-prompt">{prompt}</p>
      {error === undefined ? null : (
        <p className="paper-palette-page-error" role="alert">
          {error}
        </p>
      )}
    </div>
  );
}

/** The hints, and the result count for screen readers while the list shows. */
function Footer({ asking, count }: { asking: boolean; count: number }): ReactNode {
  return (
    <>
      <span>{message(asking ? "palette.pageFooter" : "palette.footer")}</span>
      <span className="paper-visually-hidden" role="status">
        {asking ? "" : message(count === 1 ? "palette.countOne" : "palette.count", { count })}
      </span>
    </>
  );
}

/** The palette over the provider's registry. */
export function RegistryPalette({ isOpen, onOpenChange, recent: given }: RegistryPaletteProps): ReactNode {
  const { registry, context, keys } = useCommands();
  const scope = { registry, context };
  const [recent] = useState(() => given ?? new RecentCommands());
  const [query, setQuery] = useState("");
  /** The command whose arguments the page asks for, and why the last try didn't parse. */
  const [asking, setAsking] = useState<{ id: CommandId; error?: string } | undefined>(undefined);
  const close = () => {
    onOpenChange(false);
    setQuery("");
    setAsking(undefined);
  };
  const run = (id: CommandId, text?: string) => {
    close();
    recent.add(id);
    registry.run(id, context, text === undefined ? {} : { text }).catch(() => undefined);
  };
  // Worked out on each render: a few dozen commands, and their availability follows the reader.
  const shown = registry.list().filter((command) => registry.availability(command.id, context).state !== "hidden");
  const sections = asking === undefined ? paletteSections(scope, shown, query, recent.list()) : [];
  const onAction = (id: string) => {
    const [commandId, text] = parseRowId(id);
    const args = registry.get(commandId)?.args;
    if (text !== undefined) return run(commandId, text);
    if (args !== undefined && args.optional !== true && args.secret !== true) {
      setQuery("");
      return setAsking({ id: commandId });
    }
    run(commandId);
  };
  const askingCommand = asking === undefined ? undefined : registry.get(asking.id);
  const askingArgs = askingCommand?.args;
  const submitPage = () => {
    if (askingCommand === undefined || askingArgs === undefined) return;
    const parsed = askingArgs.parse(query, context);
    if (parsed.ok) run(askingCommand.id, query);
    else setAsking({ id: askingCommand.id, error: parsed.error });
  };
  const count = sections.find((section) => section.id === "commands")?.items.length ?? 0;
  return (
    <CommandPalette
      isOpen={isOpen}
      onOpenChange={(open) => (open ? onOpenChange(true) : close())}
      aria-label={message("reader.commands")}
      placeholder={askingArgs?.prompt ?? message("palette.placeholder")}
      query={query}
      onQueryChange={(next) => {
        setQuery(next);
        if (asking?.error !== undefined) setAsking({ id: asking.id });
      }}
      sections={sections}
      onAction={onAction}
      platform={keys}
      empty={<EmptyState title={message("palette.empty")} headingLevel={2} />}
      {...(askingCommand === undefined || askingArgs === undefined
        ? {}
        : {
            page: (
              <ArgumentsPage
                title={registry.title(askingCommand, context)}
                prompt={askingArgs.prompt}
                error={asking?.error}
              />
            ),
            onPageSubmit: submitPage,
          })}
      footer={<Footer asking={askingCommand !== undefined} count={count} />}
    />
  );
}
