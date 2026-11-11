// The command registry for tests: the reader's commands over a fake platform, with whatever
// reader, viewer and shell handlers a test gives, wrapped around the components that run them.

import { type ReactNode, useState } from "react";
import { registerEditCommands } from "../src/commands/edit-commands.ts";
import { CommandsProvider } from "../src/commands/react.tsx";
import { registerReaderCommands } from "../src/commands/reader-commands.ts";
import { CommandRegistry } from "../src/commands/registry.ts";
import type { CommandContext, CommandId } from "../src/commands/types.ts";
import type { KeyPlatform } from "../src/components/shortcut-chip.tsx";
import type { EditControls } from "../src/edit/edit-controls.ts";
import type { Platform } from "../src/platform/types.ts";
import type { ViewerControls } from "../src/reader/viewer-controls.ts";
import type { ExternalStore } from "../src/state/hooks.ts";
import type { ReaderState } from "../src/state/reader-machine.ts";
import { fakePlatform } from "./fakes.ts";

/** What a test gives the commands: the reader (any store of its state), the viewer, the shell's handlers. */
export interface CommandsSetup {
  readonly platform?: Platform;
  readonly reader?: ExternalStore<ReaderState> &
    Partial<Pick<NonNullable<CommandContext["reader"]>, "openFromPicker" | "submitPassword">>;
  readonly viewer?: ViewerControls;
  readonly labels?: readonly string[];
  readonly actions?: Partial<Record<CommandId, (args: unknown) => void>>;
  /** An edit session: the edit commands join the reader's. */
  readonly edits?: EditControls;
}

/** A registry with the reader's commands, and the context the setup describes. */
export function readerCommandsFor(setup: CommandsSetup = {}): { registry: CommandRegistry; context: CommandContext } {
  const registry = new CommandRegistry();
  registerReaderCommands(registry);
  if (setup.edits !== undefined) registerEditCommands(registry);
  const { reader } = setup;
  const context: CommandContext = {
    platform: setup.platform ?? fakePlatform(),
    ...(reader === undefined
      ? {}
      : {
          reader: {
            getState: () => reader.getState(),
            subscribe: (listener: () => void) => reader.subscribe(listener),
            openFromPicker: () => reader.openFromPicker?.() ?? Promise.resolve(),
            submitPassword: (password: string) => reader.submitPassword?.(password) ?? Promise.resolve(),
          },
        }),
    ...(setup.viewer === undefined ? {} : { viewer: setup.viewer }),
    ...(setup.labels === undefined ? {} : { labels: setup.labels }),
    ...(setup.actions === undefined ? {} : { actions: setup.actions }),
    ...(setup.edits === undefined ? {} : { edits: setup.edits }),
  };
  return { registry, context };
}

/** `children` under a `CommandsProvider` over `readerCommandsFor(setup)`. */
export function WithCommands({
  keys = "other",
  listen = false,
  children,
  ...setup
}: CommandsSetup & { keys?: KeyPlatform; listen?: boolean; children: ReactNode }): ReactNode {
  const [{ registry, context }] = useState(() => readerCommandsFor(setup));
  return (
    <CommandsProvider registry={registry} context={context} keys={keys} listen={listen}>
      {children}
    </CommandsProvider>
  );
}
