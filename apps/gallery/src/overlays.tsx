// The overlays and the reader parts that `@laminapdf/ui/testing`'s state list can't show: they open
// over the page or need a reader controller. Each route shows one open state at a time (the
// `state` URL parameter), so a screenshot holds exactly one overlay.

import {
  AppearanceSettings,
  BottomSheet,
  Button,
  CommandPalette,
  CommandRegistry,
  CommandsProvider,
  DEFAULT_APPEARANCE,
  DebugMenu,
  Dialog,
  ExternalLinkDialog,
  FocusModeItem,
  IconButton,
  Inspector,
  LeftRail,
  LinkPreview,
  type LinkPreviewImage,
  Menu,
  MenuTrigger,
  ModeRegistry,
  MoreSheet,
  OpenDropZone,
  PasswordDialog,
  PhoneBottomBar,
  Popover,
  PopoverTrigger,
  ReaderController,
  ReaderShell,
  registerReaderCommands,
  SelectionActionBar,
  SelectionToolbar,
  SelectionToolbarDivider,
  SettingsDialog,
  SettingsRegistry,
  ShellLayout,
  ShortcutsDialog,
  Switch,
  shellActions,
  TextInput,
  ToastQueue,
  ToastRegion,
  Tooltip,
  useSettingsSection,
  ViewSettingsButton,
} from "@laminapdf/ui";
import { PrintDialog, PrintProgressDialog, PropertiesDialog } from "@laminapdf/ui/dialogs";
import { ConformanceDialog, NamedDestinationsDialog, RemoveSignaturesDialog } from "@laminapdf/ui/editing";
import { fakeFile, fakePlatform } from "@laminapdf/ui/testing";
import { type ReactNode, useEffect, useState } from "react";

const noop = () => {};
const SHORTCUT = { mac: "⌘F", other: "Ctrl+F" };

/** R08's sample file: a 24-page report, labelled 1–21 then A-1–A-3, with two fonts and two signatures. */
const PROPERTIES = {
  title: "Q3 Board Report",
  author: "Finance team",
  creator: "Northwind report template",
  producer: "Northwind print service",
  created: "2026-10-14T09:12:00",
  modified: "2026-10-28T16:04:00",
  version: "1.7",
  pageCount: 24,
  fileSize: 1_262_480,
  linearized: true,
  tagged: true,
  fonts: [
    { name: "SourceSerif4-Regular", kind: "Type0", embedded: true, subset: true },
    { name: "Helvetica", kind: "Type1", embedded: false, subset: false },
  ],
  signatures: 2,
  contains: { javascript: false, attachments: 3, forms: false },
};
const LABELS = [...Array.from({ length: 21 }, (_, i) => String(i + 1)), "A-1", "A-2", "A-3"];
const PAGES = Array.from({ length: 24 }, (_, i) =>
  i < 22 ? { width: 612, height: 792 } : { width: 792, height: 1224 },
);

/** Print (R09), restricted or not, and its progress; Document properties (R08). */
function OutputDialog({ state }: { state: string }): ReactNode {
  if (state === "print-progress") return <PrintProgressDialog progress={{ ready: 8, total: 24 }} onCancel={noop} />;
  if (state === "properties") {
    return (
      <PropertiesDialog
        isOpen
        fileName="Q3-board-report.pdf"
        state={{ status: "ready", properties: PROPERTIES }}
        context={{ pages: PAGES, labels: LABELS }}
        onCopy={noop}
        onClose={noop}
      />
    );
  }
  return (
    <PrintDialog
      isOpen
      pageCount={24}
      currentPage={2}
      labels={LABELS}
      {...(state === "print-restricted" ? { permissions: { print: false, printHighQuality: false } } : {})}
      onPrint={noop}
      onClose={noop}
    />
  );
}

/** Settings, Keyboard shortcuts (inside the reader's commands) and the View settings popover. */
function SettingsDialogs({ state }: { state: string }): ReactNode {
  const [appearance, setAppearance] = useState(DEFAULT_APPEARANCE);
  const [settings] = useState(() => new SettingsRegistry());
  const [commands] = useState(() => {
    const registry = new CommandRegistry();
    registerReaderCommands(registry);
    return { registry, context: { platform: fakePlatform(), actions: { "help.shortcuts": noop } } };
  });
  useSettingsSection(
    settings,
    "appearance",
    "Appearance and accessibility",
    () => <AppearanceSettings appearance={appearance} onChange={setAppearance} />,
    { shows: [appearance] },
  );
  if (state === "settings") return <SettingsDialog registry={settings} isOpen onOpenChange={noop} />;
  if (state === "view-settings") {
    return (
      <ViewSettingsButton
        appearance={appearance}
        onChange={setAppearance}
        isOpen
        onOpenChange={noop}
        onMoreSettings={noop}
      />
    );
  }
  return (
    <CommandsProvider registry={commands.registry} context={commands.context} keys="other">
      <ShortcutsDialog isOpen onOpenChange={noop} singleKeys onSingleKeysChange={noop} onOpenPalette={noop} />
    </CommandsProvider>
  );
}

/** O11's third artboard: the names, where each goes and how many bookmarks use it. */
function NamedDestinations(): ReactNode {
  const destinations = [
    { name: "chair-summary", page: "2", usedBy: 1 },
    { name: "repairs-backlog", page: "5", usedBy: 3 },
    { name: "repairs-by-region", page: "6", usedBy: 0 },
  ];
  return (
    <NamedDestinationsDialog
      isOpen
      onOpenChange={noop}
      destinations={destinations}
      onGoTo={noop}
      onRename={noop}
      onDelete={noop}
      onAdd={noop}
    />
  );
}

/** The editor's dialogs: the save's two confirmations and the named destinations. */
const EDITING_DIALOGS = new Set(["remove-signatures", "conformance", "named-destinations"]);

/** One of {@link EDITING_DIALOGS}. */
function EditingDialogs({ state }: { state: string }): ReactNode {
  if (state === "remove-signatures") {
    // O02's confirmation before a save that invalidates a certification and a signature.
    const signatures = [{ kind: "certification" }, { kind: "approval" }];
    return <RemoveSignaturesDialog signatures={signatures} onAnswer={noop} />;
  }
  if (state === "conformance") {
    const warnings = ["The file claims PDF/A-2b, and the saved file may no longer meet it."];
    return <ConformanceDialog warnings={warnings} onAnswer={noop} />;
  }
  return <NamedDestinations />;
}

/** The dialogs: the three widths, a destructive confirm, the reader's password prompt, print, properties and the save confirmations. */
export function Dialogs({ state }: { state: string }): ReactNode {
  if (state === "password") return <PasswordPrompt />;
  if (state.startsWith("print") || state === "properties") return <OutputDialog state={state} />;
  if (state === "settings" || state === "shortcuts" || state === "view-settings")
    return <SettingsDialogs state={state} />;
  if (EDITING_DIALOGS.has(state)) return <EditingDialogs state={state} />;
  if (state === "external-link") {
    // R07's drawn state: the page's text shows a shorter address than the link's real target.
    return (
      <ExternalLinkDialog
        link={{
          href: "https://surveys.example.org/2026/capacity?region=north",
          parts: { before: "https://", host: "surveys.example.org", after: "/2026/capacity?region=north" },
        }}
        onOpen={noop}
        onCopy={noop}
        onClose={noop}
      />
    );
  }
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
  if (state === "more") {
    return <MoreSheet isOpen onOpenChange={noop} more={MORE} keys="other" />;
  }
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

/** The phone's More items: the reader's More menu. */
const MORE = {
  items: [
    { id: "open", label: "Open another file…" },
    { id: "rotate", label: "Rotate clockwise", icon: "rotate-right" as const },
    { id: "save", label: "Save a copy…", icon: "download" as const },
  ],
  onAction: noop,
};

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

/** A stand-in crop of a reference list, drawn at the device's pixels: what `Viewer.preview` returns. */
async function standInCrop(): Promise<LinkPreviewImage> {
  const [width, height] = [300, 64];
  const dpr = window.devicePixelRatio || 1;
  const canvas = document.createElement("canvas");
  canvas.width = Math.round(width * dpr);
  canvas.height = Math.round(height * dpr);
  const g = canvas.getContext("2d");
  if (g !== null) {
    g.scale(dpr, dpr);
    g.fillStyle = "#fff";
    g.fillRect(0, 0, width, height);
    g.fillStyle = "#222";
    g.font = "11px serif";
    g.fillText("[12] D. E. Knuth. The TeXbook. Addison-Wesley,", 10, 22);
    g.fillText("Reading, Massachusetts, 1984.", 28, 38);
    g.fillText("[13] L. Lamport. LaTeX: A Document Preparation", 10, 56);
  }
  return { bitmap: await createImageBitmap(canvas), width, height };
}

/** The link preview over a stand-in citation: the popover (mouse and keyboard), and the touch sheet. */
export function LinkPreviews({ state }: { state: string }): ReactNode {
  const [anchor, setAnchor] = useState<HTMLElement | null>(null);
  const [picture, setPicture] = useState<LinkPreviewImage | undefined>(undefined);
  useEffect(() => {
    void standInCrop().then(setPicture);
  }, []);
  const text = "[12] D. E. Knuth. The TeXbook. Addison-Wesley, Reading, Massachusetts, 1984.";
  return (
    <div className="g-selection">
      <p>
        As shown in{" "}
        <a ref={setAnchor} href="#cite.12">
          [12]
        </a>
        , previews help.
      </p>
      {anchor === null ? null : (
        <LinkPreview
          anchor={anchor}
          pointer={state === "sheet" ? "touch" : "fine"}
          title="References"
          caption="Link to page 7 · Enter to go"
          picture={picture}
          pictureLabel={text}
          text={text}
          goLabel="Go to page 7"
          closeLabel="Close"
          onFollow={noop}
          onDismiss={noop}
          onHold={noop}
        />
      )}
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
    // The selection bar's buttons need handlers to show; here they do nothing.
    const selection = { "edit.copy": () => undefined, "selection.search": () => undefined };
    const actions = { ...shellActions(focus, new ModeRegistry(), { rail: true }), ...selection };
    const context = { platform, reader, actions };
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
        <PhoneBottomBar layout={shell.layout} rail more={MORE} keys="other" />
        <SelectionActionBar more={MORE} keys="other" />
      </div>
    </CommandsProvider>
  );
}
