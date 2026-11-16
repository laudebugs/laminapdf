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
  CrashRecoveredDialog,
  CrashReportDialog,
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
  LocateDifferentDialog,
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
import {
  ExportDialog,
  ExportProgressDialog,
  exportProgressTitle,
  ImagesToPdfDialog,
  InitialViewDialog,
  PrintDialog,
  PrintProgressDialog,
  PropertiesDialog,
} from "@laminapdf/ui/dialogs";
import {
  type AddressCheck,
  BlankPagesDialog,
  ConformanceDialog,
  ContentsPageDialog,
  type ContentsPreviewEntry,
  CropDialog,
  LinkEditor,
  type LinkGoal,
  LinkPanel,
  MergeDialog,
  MoveToPageDialog,
  NamedDestinationsDialog,
  PageBoxesDialog,
  PageLabelsDialog,
  PagesFromFileDialog,
  RemoveSignaturesDialog,
  ResizeDialog,
  SignedPageDialog,
  SplitDialog,
} from "@laminapdf/ui/editing";
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

/** Print (R09), restricted or not, and its progress; Document properties (R08); Initial view (O13). */
function OutputDialog({ state }: { state: string }): ReactNode {
  if (state.startsWith("initial-view")) return <InitialView state={state} />;
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

/** Initial view (O13): a file that asks for its outline, or one whose open action runs a script. */
function InitialView({ state }: { state: string }): ReactNode {
  const preferences = {
    hideToolbar: false,
    hideMenubar: false,
    hideWindowUi: false,
    fitWindow: false,
    displayDocTitle: true,
  };
  const view =
    state === "initial-view-action"
      ? { open: { kind: "action" as const, action: "JavaScript" }, preferences }
      : {
          open: { kind: "page" as const, page: 2, zoom: "fitWidth" as const },
          pageMode: "outlines" as const,
          preferences,
        };
  return (
    <InitialViewDialog
      isOpen
      view={view}
      pageCount={24}
      current={() => ({ page: 4, zoom: 100 })}
      fitWindowHasNoEffect
      onApply={() => Promise.resolve()}
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

/** O11's second artboard's entries, with the labels they show once the page is before page 1. */
const CONTENTS_ENTRIES: readonly ContentsPreviewEntry[] = [
  { level: 1, title: "Chair’s summary", label: "3" },
  { level: 1, title: "Repairs", label: "4" },
  { level: 2, title: "Backlog", label: "6" },
  { level: 2, title: "By region", label: "7" },
  { level: 1, title: "Budget", label: "12" },
  { level: 1, title: "Appendix", label: "A-1" },
];
const contentsPreview = () => Promise.resolve(CONTENTS_ENTRIES);

/** O11's second artboard: a contents page before page 1, two levels, with its bookmark. */
function ContentsPage(): ReactNode {
  return <ContentsPageDialog isOpen onOpenChange={noop} firstPage="1" preview={contentsPreview} onCreate={noop} />;
}

/** The editor's dialogs: the save's two confirmations, the bookmarks' two and Organise's three (O06). */
const EDITING_DIALOGS = new Set([
  "remove-signatures",
  "conformance",
  "named-destinations",
  "contents-page",
  "move-to-page",
  "blank-pages",
  "signed-page",
  "crop",
  "crop-mixed",
  "resize",
  "page-boxes",
  "page-labels",
  "page-labels-clash",
  "insert-from-file",
  "insert-password",
  "merge",
  "merge-claims",
  "split",
  "extract",
]);

/** The 24-page sample: US Letter pages, and pages 3 and 4 on A4 for the mixed states. */
const LETTER = { width: 612, height: 792 };
const SETUP_PAGES = Array.from({ length: 24 }, (_, i) =>
  i === 2 || i === 3 ? { width: 595.28, height: 841.89 } : LETTER,
);
const SETUP_IDS = SETUP_PAGES.map((_, i) => `p${i + 1}.0`);

/** O10's ranges: pages 1–2 roman, the body from 1 on page 3, the appendix A-1 to A-3 from page 22. */
const LABEL_RANGES = [
  { page: "p1.0", style: "lowerRoman" as const },
  { page: "p3.0", style: "decimal" as const },
  { page: "p22.0", style: "decimal" as const, prefix: "A-" },
];

/** A page's boxes: a crop box inside the media box, its own trim box, and bleed and art as the crop box. */
const BOXES = {
  media: [0, 0, 612, 792],
  crop: [18, 18, 594, 774],
  bleed: [18, 18, 594, 774],
  trim: [36, 36, 576, 756],
  art: [18, 18, 594, 774],
  own: { bleed: false, trim: true, art: false },
} as const;

/** One of Page setup's dialogs (O09, O10). */
function PageSetupDialog({ state }: { state: string }): ReactNode {
  if (state === "crop" || state === "crop-mixed") {
    const selected = state === "crop" ? [2] : [1, 2, 3];
    return <CropDialog isOpen onClose={noop} unit="mm" pages={SETUP_PAGES} selected={selected} onCrop={noop} />;
  }
  if (state === "resize") {
    return <ResizeDialog isOpen onClose={noop} unit="mm" pages={SETUP_PAGES.slice(4)} onResize={noop} />;
  }
  if (state === "page-boxes") {
    return <PageBoxesDialog isOpen onClose={noop} page="5" count={1} boxes={BOXES} onApply={noop} />;
  }
  // Two ranges on page 22, as O10's state with the clash.
  const ranges =
    state === "page-labels-clash" ? [...LABEL_RANGES, { page: "p22.0", style: "upperLetters" as const }] : LABEL_RANGES;
  return <PageLabelsDialog isOpen onClose={noop} pages={SETUP_IDS} ranges={ranges} onApply={noop} />;
}

/** Twelve pages for Move to page…, pages 5 and 6 selected. */
const MOVE_PAGES = Array.from({ length: 12 }, (_, i) => ({ id: `p${i + 1}.0`, label: String(i + 1) }));

/** One of Organise's dialogs (O06). */
function OrganiseDialog({ state }: { state: string }): ReactNode {
  if (state === "move-to-page") {
    return <MoveToPageDialog isOpen onClose={noop} pages={MOVE_PAGES} selected={[4, 5]} onMove={noop} />;
  }
  if (state === "blank-pages") return <BlankPagesDialog isOpen onClose={noop} hasSelection onInsert={noop} />;
  return <SignedPageDialog ask={{ action: "delete", page: "23" }} onAnswer={noop} />;
}

/** A file's pages for the dialogs across files: a US Letter page, then a landscape one. */
const SOURCE_PAGES = [LETTER, { width: 792, height: 612 }, LETTER];

/** Draws a source page as a white page with a grey block where its heading would be. */
async function drawSourcePage(_page: number, canvas: HTMLCanvasElement): Promise<void> {
  canvas.width = 90;
  canvas.height = 116;
  const context = canvas.getContext("2d");
  if (context === null) return;
  context.fillStyle = "#ffffff";
  context.fillRect(0, 0, 90, 116);
  context.fillStyle = "#c8c4bc";
  context.fillRect(10, 12, 60, 8);
}

/** One of the dialogs across files (O07, O08). */
function DocumentsDialog({ state }: { state: string }): ReactNode {
  if (state === "insert-from-file" || state === "insert-password") {
    const source =
      state === "insert-password"
        ? { name: "Appendix-B.pdf", size: 412_000, status: "password" as const, wrong: true }
        : { name: "Appendix-B.pdf", size: 412_000, status: "ready" as const, pages: SOURCE_PAGES };
    const labels = SETUP_IDS.map((_, i) => String(i + 1));
    return (
      <PagesFromFileDialog
        isOpen
        onClose={noop}
        mode="insert"
        source={source}
        onChoose={noop}
        onPassword={noop}
        draw={drawSourcePage}
        labels={labels}
        selected={[10]}
        onApply={noop}
      />
    );
  }
  if (state === "merge" || state === "merge-claims") {
    const claims = state === "merge-claims" ? ["PDF/A-2B"] : [];
    const files = [
      {
        key: "a",
        source: { name: "Rent-collection.pdf", size: 1_204_000, status: "ready" as const, pages: SOURCE_PAGES },
      },
      { key: "b", source: { name: "Surveyor-capacity.pdf", size: 88_000, status: "loading" as const } },
    ];
    return (
      <MergeDialog
        isOpen
        onClose={noop}
        current={{ name: "Q3-board-report", claims }}
        files={files}
        onAdd={noop}
        onMove={noop}
        onRemove={noop}
        onPassword={noop}
        onMerge={noop}
      />
    );
  }
  const chapters = [
    { title: "Chair’s summary", page: 0 },
    { title: "Repairs backlog", page: 2 },
    { title: "Rent collection", page: 6 },
    { title: "Surveyor capacity", page: 10 },
    { title: "Risk register", page: 15 },
    { title: "Appendix A. Data notes", page: 21 },
  ];
  return (
    <SplitDialog
      isOpen
      onClose={noop}
      start={state === "extract" ? "ranges" : "bookmarks"}
      labels={SETUP_IDS.map((_, i) => String(i + 1))}
      selected={state === "extract" ? [2, 3, 4, 5] : []}
      chapters={chapters}
      stem="Q3-board-report"
      output={(parts) => (parts === 1 ? "file" : "zip")}
      signed={state === "split"}
      progress={undefined}
      onSplit={noop}
    />
  );
}

/** One of {@link EDITING_DIALOGS}. */
function EditingDialogs({ state }: { state: string }): ReactNode {
  if (state.startsWith("insert-") || state.startsWith("merge") || state === "split" || state === "extract") {
    return <DocumentsDialog state={state} />;
  }
  if (state === "move-to-page" || state === "blank-pages" || state === "signed-page") {
    return <OrganiseDialog state={state} />;
  }
  if (state.startsWith("crop") || state === "resize" || state.startsWith("page-")) {
    return <PageSetupDialog state={state} />;
  }
  if (state === "remove-signatures") {
    // O02's confirmation before a save that invalidates a certification and a signature.
    const signatures = [{ kind: "certification" }, { kind: "approval" }];
    return <RemoveSignaturesDialog signatures={signatures} onAnswer={noop} />;
  }
  if (state === "conformance") {
    const warnings = ["The file claims PDF/A-2b, and the saved file may no longer meet it."];
    return <ConformanceDialog warnings={warnings} onAnswer={noop} />;
  }
  if (state === "contents-page") return <ContentsPage />;
  return <NamedDestinations />;
}

/** A crash report as apps/updates' test fixture shapes it: an engine panic, pretty-printed. */
const CRASH_REPORT = JSON.stringify(
  {
    schema: 1,
    report_id: "3f1c2a9e-8b7d-4e6f-9a5b-1c2d3e4f5a6b",
    app_version: "0.1.0",
    channel: "beta",
    os: "macos",
    os_version: "15.1",
    arch: "aarch64",
    process: "engine",
    renderer: "pdfium",
    sandbox: ["seatbelt", "memory_limit"],
    uptime: "1m_to_10m",
    kind: "panic",
    reason: "index out of bounds: the len is 3 but the index is 7 (in <path>)",
    location: { file: "crates/laminapdf-desktop-engine/src/worker.rs", line: 120, column: 9 },
    frames: [
      { module: "laminapdf-desktop-engine", offset: "0x1a2b3c", function: "laminapdf_desktop_engine::worker::render" },
      { module: "<unknown>", offset: "0x7f00", function: null },
    ],
    modules: [],
  },
  null,
  2,
);

/** The desktop's own dialogs: S02's located-file question, S08's crash report question and preview, and its tab choice. */
const DESKTOP_DIALOGS = new Set([
  "locate-different",
  "crash-main",
  "crash-engine",
  "crash-report",
  "crash-tabs",
  "crash-tabs-report",
]);

/** S08's tab choice: three tabs of the last session, the one in front when the app quit unchecked. */
const CRASH_TABS = [
  { token: "t1", name: "Q3-board-report.pdf", page: 4, front: false },
  { token: "t2", name: "site-survey-scans.pdf", page: 212, front: true },
  { token: "t3", name: "lease-agreement.pdf", page: 1, front: false },
];

/** One of {@link DESKTOP_DIALOGS}. */
function DesktopDialog({ state }: { state: string }): ReactNode {
  // S02: the desktop's question before a located file that differs from the session's opens.
  if (state === "locate-different") return <LocateDifferentDialog name="Q2-board-report.pdf" onAnswer={noop} />;
  // S08's tab choice after an unexpected quit, alone (crash-tabs) and with the report part (crash-tabs-report).
  if (state === "crash-tabs" || state === "crash-tabs-report") {
    return (
      <CrashRecoveredDialog
        tabs={CRASH_TABS}
        {...(state === "crash-tabs-report" ? { report: { json: CRASH_REPORT, onCopy: noop } } : {})}
        onChoose={noop}
      />
    );
  }
  // S08: after the app quit (crash-main) or the document view restarted (crash-engine), and the report.
  return (
    <CrashReportDialog
      process={state === "crash-main" ? "main" : "engine"}
      report={CRASH_REPORT}
      onAnswer={noop}
      onCopy={noop}
      {...(state === "crash-report" ? { initialView: "preview" as const } : {})}
    />
  );
}

/** Export (O16) on a 24-page file with pictures, links and bookmarks; its progress, and a page that failed; From images (O07). */
function ExportDialogs({ state }: { state: string }): ReactNode {
  if (state === "images-to-pdf") {
    const files = [{ name: "receipt-front.jpg" }, { name: "receipt-back.jpg" }, { name: "invoice.png" }];
    return <ImagesToPdfDialog isOpen files={files} onChooseFiles={noop} onInsert={noop} onClose={noop} />;
  }
  if (state === "export") {
    return (
      <ExportDialog
        isOpen
        pageCount={24}
        currentPage={2}
        labels={LABELS}
        pageSize={{ width: 612, height: 792 }}
        counts={{ pictures: 12, links: 8, bookmarks: 5, tagged: false }}
        onExport={noop}
        onClose={noop}
      />
    );
  }
  const choice = { kind: "pages", pages: PAGES.map((_, i) => i), format: "png", dpi: 150, annotations: true } as const;
  return (
    <ExportProgressDialog
      isOpen
      title={exportProgressTitle(choice, 24)}
      done={15}
      total={24}
      unit="pages"
      fileName="Q3-board-report-pages.zip"
      failed={state === "export-failed" ? [16] : undefined}
      onCancel={noop}
      {...(state === "export-failed" ? { onExportRest: noop } : {})}
      onClose={noop}
    />
  );
}

/** The dialogs: the three widths, a destructive confirm, the reader's password prompt, print, properties and the save confirmations. */
export function Dialogs({ state }: { state: string }): ReactNode {
  if (state === "password") return <PasswordPrompt />;
  if (state.startsWith("print") || state === "properties" || state.startsWith("initial-view")) {
    return <OutputDialog state={state} />;
  }
  if (state.startsWith("export") || state === "images-to-pdf") return <ExportDialogs state={state} />;
  if (state === "settings" || state === "shortcuts" || state === "view-settings")
    return <SettingsDialogs state={state} />;
  if (EDITING_DIALOGS.has(state)) return <EditingDialogs state={state} />;
  if (DESKTOP_DIALOGS.has(state)) return <DesktopDialog state={state} />;
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

/** The gallery's stand-in for the reader's address check: web addresses only. */
function checkAddress(address: string): AddressCheck {
  return /^https?:\/\/\S+$/i.test(address) ? { ok: true, address } : { ok: false, reason: "script" };
}

/** The goal each link editor state shows. */
const LINK_GOALS: Record<string, LinkGoal | undefined> = {
  page: { kind: "page", page: 4 },
  web: { kind: "web", address: "https://example.com/report" },
  refused: { kind: "web", address: "https://example.com/report" },
  script: { kind: "script", action: "JavaScript" },
};

/**
 * O12's link editor by a link on text: a page target, a web address, a refused `javascript:`
 * address (type it in the field and leave it to see the error), and a script link read from a file.
 */
export function LinkEditors({ state }: { state: string }): ReactNode {
  const goal = LINK_GOALS[state];
  return (
    <div className="g-selection">
      <p>
        Read the <mark className="g-selected">full report</mark> online.
      </p>
      <LinkPanel anchor={() => ({ top: 160, left: 120, width: 90, height: 22 })} onDismiss={noop}>
        <LinkEditor
          key={state}
          text="full report"
          goal={goal}
          pageCount={12}
          names={["chapter-2", "summary"]}
          contents={state === "script" ? "" : "The full report"}
          onGoal={noop}
          onContents={noop}
          onDelete={noop}
          checkAddress={checkAddress}
          {...(state === "page" || state === "web" ? { onFollow: noop } : {})}
        />
      </LinkPanel>
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
