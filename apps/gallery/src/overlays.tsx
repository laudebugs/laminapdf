// The overlays and the reader parts that `@laminapdf/ui/testing`'s state list can't show: they open
// over the page or need a reader controller. Each route shows one open state at a time (the
// `state` URL parameter), so a screenshot holds exactly one overlay.

import {
  BottomSheet,
  Button,
  CommandPalette,
  CommandRegistry,
  CommandsProvider,
  DebugMenu,
  Dialog,
  FocusModeItem,
  IconButton,
  Inspector,
  LeftRail,
  Menu,
  MenuTrigger,
  ModeRegistry,
  OpenDropZone,
  PasswordDialog,
  Popover,
  PopoverTrigger,
  ReaderController,
  ReaderShell,
  registerReaderCommands,
  SelectionToolbar,
  SelectionToolbarDivider,
  ShellLayout,
  Switch,
  shellActions,
  TextInput,
  ToastQueue,
  ToastRegion,
  Tooltip,
} from "@laminapdf/ui";
import { fakeFile, fakePlatform } from "@laminapdf/ui/testing";
import { type ReactNode, useEffect, useState } from "react";

const noop = () => {};
const SHORTCUT = { mac: "⌘F", other: "Ctrl+F" };

/** The dialogs: the three widths, a destructive confirm, and the reader's password prompt. */
export function Dialogs({ state }: { state: string }): ReactNode {
  if (state === "password") return <PasswordPrompt />;
  const size = state === "form" ? "form" : state === "wide" ? "wide" : "default";
  if (state === "alert") {
    return (
      <Dialog
        isOpen
        onOpenChange={noop}
        role="alertdialog"
        title="Delete 3 pages?"
        description="The pages go from this document. Undo brings them back until you close it."
        closeLabel="Close"
        actions={
          <>
            <Button variant="secondary" autoFocus>
              Cancel
            </Button>
            <Button variant="danger-filled">Delete pages</Button>
          </>
        }
      />
    );
  }
  return (
    <Dialog
      isOpen
      onOpenChange={noop}
      size={size}
      title={size === "form" ? "Document properties" : size === "wide" ? "Compare pages" : "Move pages"}
      description="Choose where the 3 selected pages go."
      closeLabel="Close"
      actions={
        <>
          <Button variant="secondary">Cancel</Button>
          <Button variant="primary">Move</Button>
        </>
      }
    >
      <TextInput label="Before page" defaultValue="12" />
    </Dialog>
  );
}

/** The reader's password prompt, over a controller that has just asked for a password. */
function PasswordPrompt(): ReactNode {
  const [reader] = useState(
    () =>
      new ReaderController(fakePlatform(), async () => {
        throw Object.assign(new Error("locked"), { code: "password-required" });
      }),
  );
  useEffect(() => {
    void reader.open(fakeFile("locked-report.pdf"));
  }, [reader]);
  return <PasswordDialog reader={reader} />;
}

/** A bottom sheet at each detent, and the non-modal one beside a page in use. */
export function Sheets({ state }: { state: string }): ReactNode {
  const pages = <p>24 pages, three selected.</p>;
  if (state === "non-modal") {
    return (
      <BottomSheet isOpen isModal={false} onOpenChange={noop} title="Pages" count="24 pages" closeLabel="Close pages">
        {pages}
      </BottomSheet>
    );
  }
  return (
    <BottomSheet
      isOpen
      onOpenChange={noop}
      title="Pages"
      count="24 pages"
      closeLabel="Close pages"
      resizeLabel="Resize sheet"
      detent={state === "large" ? "large" : "medium"}
      footer={<Button variant="primary">Done</Button>}
    >
      {pages}
    </BottomSheet>
  );
}

/** A menu and a popover open from their buttons; a tooltip shows on keyboard focus (the test tabs to it). */
export function Menus({ state }: { state: string }): ReactNode {
  if (state === "popover") {
    return (
      <PopoverTrigger defaultOpen>
        <Button icon="settings">View settings</Button>
        <Popover aria-label="View settings">
          <Switch>Show page labels</Switch>
        </Popover>
      </PopoverTrigger>
    );
  }
  if (state === "tooltip") {
    return (
      <Tooltip text="Find in document" shortcut={SHORTCUT} platform="other">
        <IconButton label="Find in document" icon="find" />
      </Tooltip>
    );
  }
  return (
    <MenuTrigger defaultOpen>
      <Button icon="more">More</Button>
      <Menu
        aria-label="More"
        platform="other"
        onAction={noop}
        items={[
          { id: "open", label: "Open…", icon: "folder", shortcut: { mac: "⌘O", other: "Ctrl+O" } },
          { id: "rotate", label: "Rotate page", icon: "rotate-right" },
          { id: "print", label: "Print…", isDisabled: true, disabledReason: "This file doesn't allow printing" },
          { id: "close", label: "Close document", isDestructive: true },
        ]}
      />
    </MenuTrigger>
  );
}

/** The selection toolbar over a fixed stand-in selection. */
export function Selection(): ReactNode {
  return (
    <div className="g-selection">
      <p>
        The <mark className="g-selected">selected words</mark> sit under the toolbar.
      </p>
      <SelectionToolbar
        aria-label="Selection"
        anchor={() => ({ top: 160, left: 120, width: 140, height: 22 })}
        onDismiss={noop}
      >
        <IconButton label="Copy" icon="copy" />
        <IconButton label="Highlight" icon="highlight" />
        <SelectionToolbarDivider />
        <IconButton label="Comment" icon="comment" />
      </SelectionToolbar>
    </div>
  );
}

/** The command palette with results, and with none. */
export function Palette({ state }: { state: string }): ReactNode {
  const empty = state === "empty";
  return (
    <CommandPalette
      isOpen
      onOpenChange={noop}
      aria-label="Command palette"
      placeholder="Type a command"
      query={empty ? "zzz" : "ro"}
      onQueryChange={noop}
      onAction={noop}
      platform="other"
      empty={<p>No command matches "zzz".</p>}
      sections={
        empty
          ? []
          : [
              {
                id: "commands",
                title: "Commands",
                items: [
                  { id: "rotate", title: "Rotate page", matches: [[0, 2]], shortcut: { mac: "⌘R", other: "Ctrl+R" } },
                  {
                    id: "rotate-all",
                    title: "Rotate all pages",
                    matches: [[0, 2]],
                    isDisabled: true,
                    disabledReason: "Read-only file",
                  },
                ],
              },
            ]
      }
    />
  );
}

/** The toast region with two toasts that stay until dismissed (a timeout past the queue's longest timer). */
export function Toasts(): ReactNode {
  const [queue] = useState(() => {
    const q = new ToastQueue();
    q.add({ message: "Saved a copy to Downloads", timeout: Number.POSITIVE_INFINITY });
    q.add({ message: "3 pages deleted", action: { label: "Undo", onPress: noop }, timeout: Number.POSITIVE_INFINITY });
    return q;
  });
  return <ToastRegion queue={queue} aria-label="Notifications" />;
}

/** The reader's frame, its drop zone and its debug menu, over a controller that opens nothing. */
export function ReaderParts(): ReactNode {
  const [reader] = useState(() => new ReaderController(fakePlatform(), async () => ({ pageCount: 24 })));
  return (
    <div className="g-shell">
      <ReaderShell topBar={<strong>report.pdf</strong>} status={<span>24 pages</span>}>
        <OpenDropZone reader={reader} keys="other">
          <p>Drop a PDF here, or open one.</p>
        </OpenDropZone>
      </ReaderShell>
      <DebugMenu backend="pdfium" onBackendChange={noop} keys="other" />
    </div>
  );
}

/**
 * The shell's regions (UI/UX plan §5.1): the rail open with its tabs, a selection's inspector
 * docked beside the canvas, and focus mode's status item (from a second layout in focus mode, since
 * focus mode hides the rail).
 */
export function ShellRegions(): ReactNode {
  const [shell] = useState(() => {
    const layout = new ShellLayout();
    layout.toggleRail();
    layout.openInspector({ id: "highlight", selection: true });
    const focus = new ShellLayout();
    focus.toggleFocus();
    const registry = new CommandRegistry();
    registerReaderCommands(registry);
    const platform = fakePlatform();
    const reader = new ReaderController(platform, async () => ({ pageCount: 24 }));
    const context = { platform, reader, actions: shellActions(focus, new ModeRegistry(), { rail: true }) };
    return { layout, focus, registry, context };
  });
  const tabs = [
    { id: "pages", label: "Pages", panel: <p>24 pages</p> },
    { id: "outline", label: "Outline", panel: <p>1 Summary · 2 Regions · 3 Outlook</p> },
  ];
  return (
    <CommandsProvider registry={shell.registry} context={shell.context} keys="other">
      <div className="g-shell g-shell-regions">
        <ReaderShell
          layout={shell.layout}
          topBar={<strong>report.pdf</strong>}
          rail={<LeftRail layout={shell.layout} tabs={tabs} />}
          inspector={
            <Inspector layout={shell.layout} label="Highlight">
              <p>Yellow · 2 lines</p>
            </Inspector>
          }
          status={<FocusModeItem layout={shell.focus} />}
        >
          <p>The page</p>
        </ReaderShell>
      </div>
    </CommandsProvider>
  );
}
