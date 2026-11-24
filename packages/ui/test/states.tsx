// Every component in each state it can be put in through props, for the axe and size tests here
// and for the gallery's screenshots later. Hover and pressed come from real pointer events in
// the tests that need them, not from props, so shipped code carries nothing for tests.

import type { ReactElement } from "react";
import { type AttachmentData, AttachmentsPanel, type AttachmentsPanelProps, ThreadsPanel } from "../src/dialogs.ts";
import {
  AnnotateBar,
  AnnotationInspector,
  type AnnotationInspectorProps,
  BookmarkEditor,
  type BookmarkItem,
  CommentsPanel,
  type CommentsPanelProps,
  CommentThreadDetail,
  type CommentThreadItem,
  CommentThreadSummary,
  CommentToolbar,
  EditArea,
  FILL_COLOURS,
  FieldConflictDialog,
  FieldsPanel,
  FormToolbar,
  HistoryPanel,
  LINE_COLOURS,
  LinkEditor,
  OptionsStrip,
  OrganiseToolbar,
  PageGrid,
  type SavePlanSlice,
  SaveStrip,
  ToolShelf,
} from "../src/editing.ts";
import {
  Badge,
  Button,
  Checkbox,
  CrashNotice,
  DEFAULT_READER_SETTINGS,
  DocumentTabs,
  EmptyState,
  FindBar,
  FindResults,
  IconButton,
  ModeSwitcher,
  NetworkActivityPanel,
  NetworkIndicator,
  Notice,
  OfflineNotice,
  OpenFailure,
  OutlineRow,
  OutlineTree,
  type OutlineTreeItem,
  PageAppearanceChips,
  PageAppearanceFields,
  PageInput,
  PageLabel,
  ProgressBar,
  Radio,
  RadioGroup,
  type ReaderState,
  ReaderTopBar,
  RecentList,
  RemoteFailure,
  RepairDetails,
  RepairedNotice,
  RestoreNotice,
  SegmentedTabs,
  Select,
  ShortcutChip,
  Skeleton,
  StatusBarItem,
  SwatchGroup,
  Switch,
  TabPanel,
  TextArea,
  TextInput,
  Thumbnail,
  Toast,
  ToggleButton,
  ToolToggle,
  WholeFileDownload,
  ZoomControl,
} from "../src/index.ts";
import { stepThreadPart } from "../src/reader/threads.ts";
import { ReaderSettingsPanel, ReaderView } from "../src/reader-mode.ts";
import { WithCommands } from "./commands.tsx";
import { FakeEdits, FakeFind, FakeViewer, fakeFile } from "./fakes.ts";
import { inspectorExamples } from "./states-inspector.tsx";

/** One rendering of one component in one state. */
export interface Example {
  /** The exported component it shows. */
  component: string;
  /** The state, in words. */
  state: string;
  /** The element. */
  element: () => ReactElement;
}

const noop = () => {};
const MODES = [
  { id: "read", label: "Read" },
  { id: "comment", label: "Comment" },
  { id: "organise", label: "Organise" },
] as const;
const TABS = [
  { id: "pages", label: "Pages" },
  { id: "outline", label: "Outline" },
  { id: "attachments", label: "Attachments" },
] as const;
const SWATCHES = [
  { id: "yellow", label: "Yellow", color: "annotation-yellow" },
  { id: "green", label: "Green", color: "annotation-green" },
  { id: "blue", label: "Blue", color: "annotation-blue" },
  { id: "pink", label: "Pink", color: "annotation-pink" },
] as const;
const APPEARANCE_LABELS = { original: "Original", sepia: "Sepia", dark: "Dark", "high-contrast": "High contrast" };
const OPTIONS = [
  { value: "a4", label: "A4" },
  { value: "letter", label: "Letter" },
] as const;

const documentTabs: Example[] = [
  {
    component: "DocumentTabs",
    state: "three tabs, the second shown",
    element: () => (
      <DocumentTabs
        tabs={[
          { id: "1", title: "report.pdf" },
          { id: "2", title: "Board papers, November 2026, final.pdf" },
          { id: "3", title: "notes.pdf" },
        ]}
        active="2"
        onActivate={() => {}}
        onClose={() => {}}
        onReorder={() => {}}
        moves={[{ id: "new", label: "Move tab to new window" }]}
        onMove={() => {}}
        platform="mac"
      />
    ),
  },
  {
    component: "DocumentTabs",
    state: "restored, one file missing (S02-tab-missing)",
    element: () => (
      <DocumentTabs
        tabs={[
          { id: "1", title: "report.pdf", detail: "Page 12" },
          { id: "2", title: "Q2-board-report.pdf", missing: true },
          { id: "3", title: "notes.pdf", detail: "Page 3" },
        ]}
        active="2"
        onActivate={() => {}}
        onClose={() => {}}
        platform="mac"
      />
    ),
  },
  {
    component: "DocumentTabs",
    state: "no tab shown (an error in the window)",
    element: () => (
      <DocumentTabs
        tabs={[
          { id: "1", title: "report.pdf" },
          { id: "2", title: "notes.pdf" },
        ]}
        active={undefined}
        onActivate={() => {}}
        onClose={() => {}}
        platform="other"
      />
    ),
  },
];

const buttons: Example[] = [
  ...(["primary", "secondary", "ghost", "danger", "danger-filled"] as const).flatMap((variant) => [
    { component: "Button", state: variant, element: () => <Button variant={variant}>Save</Button> },
    {
      component: "Button",
      state: `${variant} disabled`,
      element: () => (
        <Button variant={variant} isDisabled>
          Save
        </Button>
      ),
    },
  ]),
  ...([30, 26] as const).map((size) => ({
    component: "Button",
    state: `size ${size}`,
    element: () => <Button size={size}>Open</Button>,
  })),
  { component: "Button", state: "icon and text", element: () => <Button icon="download">Download</Button> },
  ...(["secondary", "primary", "danger-filled"] as const).map((variant) => ({
    component: "Button",
    state: variant === "secondary" ? "pending" : `${variant} pending`,
    element: () => (
      <Button variant={variant} isPending>
        Saving
      </Button>
    ),
  })),
  ...([26, 28, 32] as const).map((size) => ({
    component: "IconButton",
    state: `size ${size}`,
    element: () => <IconButton label="Find in document" icon="find" size={size} />,
  })),
  { component: "IconButton", state: "disabled", element: () => <IconButton label="Undo" icon="undo" isDisabled /> },
  { component: "IconButton", state: "pending", element: () => <IconButton label="Save" icon="download" isPending /> },
];

const toggles: Example[] = [
  { component: "ToggleButton", state: "off", element: () => <ToggleButton icon="sidebar">Sidebar</ToggleButton> },
  {
    component: "ToggleButton",
    state: "on",
    element: () => (
      <ToggleButton icon="sidebar" defaultSelected>
        Sidebar
      </ToggleButton>
    ),
  },
  {
    component: "ToggleButton",
    state: "icon only",
    element: () => <ToggleButton icon="reader" aria-label="Reader" />,
  },
  { component: "ToolToggle", state: "off", element: () => <ToolToggle icon="highlight">Highlight</ToolToggle> },
  {
    component: "ToolToggle",
    state: "on",
    element: () => (
      <ToolToggle icon="highlight" defaultSelected>
        Highlight
      </ToolToggle>
    ),
  },
  {
    component: "ToolToggle",
    state: "disabled",
    element: () => (
      <ToolToggle icon="comment" isDisabled>
        Note
      </ToolToggle>
    ),
  },
  {
    component: "ToolToggle",
    state: "on, disabled",
    element: () => (
      <ToolToggle icon="highlight" defaultSelected isDisabled>
        Highlight
      </ToolToggle>
    ),
  },
  {
    component: "ToggleButton",
    state: "disabled",
    element: () => (
      <ToggleButton icon="sidebar" isDisabled>
        Sidebar
      </ToggleButton>
    ),
  },
  {
    component: "ModeSwitcher",
    state: "Read",
    element: () => <ModeSwitcher aria-label="Mode" modes={MODES} value="read" onChange={noop} />,
  },
  {
    component: "SegmentedTabs",
    state: "Pages",
    element: () => (
      <SegmentedTabs aria-label="Sidebar" tabs={TABS} selectedKey="pages" onSelectionChange={noop}>
        {TABS.map((tab) => (
          <TabPanel key={tab.id} id={tab.id}>
            {tab.label} panel
          </TabPanel>
        ))}
      </SegmentedTabs>
    ),
  },
];

const fields: Example[] = [
  { component: "TextInput", state: "empty", element: () => <TextInput label="Title" placeholder="Untitled" /> },
  {
    component: "TextInput",
    state: "with help",
    element: () => <TextInput label="Author" description="Shown in the document's properties" defaultValue="A. Lee" />,
  },
  {
    component: "TextInput",
    state: "invalid",
    element: () => <TextInput label="Page" defaultValue="40" errorMessage="Enter a page from 1 to 24" />,
  },
  {
    component: "TextInput",
    state: "disabled",
    element: () => <TextInput label="Producer" defaultValue="Lamina PDF" isDisabled />,
  },
  {
    component: "TextInput",
    state: "read-only",
    element: () => <TextInput label="Created" defaultValue="14 November 2026" isReadOnly />,
  },
  {
    component: "TextArea",
    state: "with text",
    element: () => <TextArea label="Note" defaultValue="Check the totals." />,
  },
  {
    component: "TextArea",
    state: "invalid",
    element: () => <TextArea label="Note" defaultValue="" errorMessage="Write a note to add it" />,
  },
  {
    component: "TextArea",
    state: "read-only",
    element: () => <TextArea label="Note" defaultValue="Checked by the auditor." isReadOnly />,
  },
  {
    component: "TextArea",
    state: "disabled",
    element: () => <TextArea label="Note" defaultValue="Check the totals." isDisabled />,
  },
  {
    component: "Select",
    state: "chosen",
    element: () => <Select label="Paper size" options={OPTIONS} value="a4" onChange={noop} />,
  },
  {
    component: "Select",
    state: "disabled",
    element: () => <Select label="Paper size" options={OPTIONS} value="a4" onChange={noop} isDisabled />,
  },
  {
    component: "Select",
    state: "invalid",
    element: () => (
      <Select
        label="Paper size"
        options={OPTIONS}
        value="a4"
        onChange={noop}
        errorMessage="Choose a size the printer supports"
      />
    ),
  },
];

const choices: Example[] = [
  { component: "Checkbox", state: "off", element: () => <Checkbox>Highlight fields</Checkbox> },
  { component: "Checkbox", state: "on", element: () => <Checkbox defaultSelected>Highlight fields</Checkbox> },
  { component: "Checkbox", state: "mixed", element: () => <Checkbox isIndeterminate>All pages</Checkbox> },
  { component: "Checkbox", state: "disabled", element: () => <Checkbox isDisabled>Locked</Checkbox> },
  {
    component: "Checkbox",
    state: "on, disabled",
    element: () => (
      <Checkbox defaultSelected isDisabled>
        Locked
      </Checkbox>
    ),
  },
  {
    component: "RadioGroup",
    state: "one chosen",
    element: () => (
      <RadioGroup label="Pages" defaultValue="all">
        <Radio value="all">All pages</Radio>
        <Radio value="current">Current page</Radio>
      </RadioGroup>
    ),
  },
  {
    component: "RadioGroup",
    state: "disabled",
    element: () => (
      <RadioGroup label="Pages" defaultValue="all" isDisabled>
        <Radio value="all">All pages</Radio>
        <Radio value="current">Current page</Radio>
      </RadioGroup>
    ),
  },
  {
    component: "RadioGroup",
    state: "invalid",
    element: () => (
      <RadioGroup label="Format" errorMessage="Choose a format">
        <Radio value="pdf">PDF</Radio>
        <Radio value="png">PNG</Radio>
      </RadioGroup>
    ),
  },
  { component: "Switch", state: "off", element: () => <Switch>Show thumbnails</Switch> },
  { component: "Switch", state: "on", element: () => <Switch defaultSelected>Show thumbnails</Switch> },
  { component: "Switch", state: "disabled", element: () => <Switch isDisabled>Show thumbnails</Switch> },
  {
    component: "Switch",
    state: "on, disabled",
    element: () => (
      <Switch defaultSelected isDisabled>
        Show thumbnails
      </Switch>
    ),
  },
];

const pickers: Example[] = [
  {
    component: "SwatchGroup",
    state: "24 px",
    element: () => <SwatchGroup label="Colour" swatches={SWATCHES} value="yellow" onChange={noop} />,
  },
  {
    component: "SwatchGroup",
    state: "28 px",
    element: () => <SwatchGroup label="Colour" swatches={SWATCHES} value="blue" onChange={noop} size={28} />,
  },
  {
    component: "PageAppearanceChips",
    state: "Sepia",
    element: () => (
      <PageAppearanceChips aria-label="Page appearance" labels={APPEARANCE_LABELS} value="sepia" onChange={noop} />
    ),
  },
  {
    component: "PageAppearanceFields",
    state: "Dark, this document",
    element: () => <PageAppearanceFields value="dark" onChange={noop} scope="document" onScopeChange={noop} />,
  },
  {
    component: "PageAppearanceFields",
    state: "No document open",
    element: () => <PageAppearanceFields value="original" onChange={noop} />,
  },
];

const page = () => <span style={{ display: "block", blockSize: "100%", background: "var(--paper-page-paper)" }} />;
const rail: Example[] = [
  {
    component: "Thumbnail",
    state: "current",
    element: () => (
      <Thumbnail label="Page 3, 3 of 24" number="3" isCurrent>
        {page()}
      </Thumbnail>
    ),
  },
  { component: "Thumbnail", state: "other page", element: () => <Thumbnail label="Page A-1, 22 of 24" number="A-1" /> },
  {
    component: "Thumbnail",
    state: "phone",
    element: () => <Thumbnail label="Page 4, 4 of 24" number="4" size="phone" />,
  },
  {
    component: "OutlineRow",
    state: "current, expanded",
    element: () => (
      <OutlineRow
        title="2. Repairs backlog"
        page="3"
        href="#repairs"
        isCurrent
        toggle={{ isExpanded: true, onExpandedChange: noop, label: "Sections in 2. Repairs backlog" }}
      />
    ),
  },
  {
    component: "OutlineRow",
    state: "level 2",
    element: () => <OutlineRow title="Open repairs by region" page="3" level={2} onPress={noop} toggle="space" />,
  },
  {
    component: "OutlineTree",
    state: "nested, current section open",
    element: () => <OutlineTree items={OUTLINE} current="1.1" onNavigate={noop} />,
  },
  {
    component: "OutlineTree",
    state: "closed",
    element: () => <OutlineTree items={OUTLINE.map((i) => ({ ...i, open: false }))} onNavigate={noop} />,
  },
  {
    component: "OutlineRow",
    state: "collapsed",
    element: () => (
      <OutlineRow
        title="3. Budget"
        page="7"
        href="#budget"
        toggle={{ isExpanded: false, onExpandedChange: noop, label: "Sections in 3. Budget" }}
      />
    ),
  },
];

const status: Example[] = [
  {
    component: "StatusBarItem",
    state: "with an icon",
    element: () => (
      <StatusBarItem icon="sync" onPress={noop}>
        Comments synced 10:32
      </StatusBarItem>
    ),
  },
  {
    component: "StatusBarItem",
    state: "disabled",
    element: () => (
      <StatusBarItem icon="sync" isDisabled>
        Comments synced 10:32
      </StatusBarItem>
    ),
  },
  {
    component: "NetworkIndicator",
    state: "nothing sent",
    element: () => (
      <NetworkIndicator
        count="0 network requests for this document"
        explanation="· Offline-safe: nothing uploaded"
        onPress={noop}
      />
    ),
  },
  {
    component: "NetworkIndicator",
    state: "warning",
    element: () => (
      <NetworkIndicator count="2 network requests" explanation="· to files.example.org" warning onPress={noop} />
    ),
  },
  {
    component: "RepairedNotice",
    state: "default",
    element: () => <RepairedNotice label="Repaired on open" details="Details" onPress={noop} />,
  },
  { component: "PageLabel", state: "default", element: () => <PageLabel>Page 3 · Repairs backlog</PageLabel> },
  {
    component: "ShortcutChip",
    state: "macOS",
    element: () => <ShortcutChip keys={{ mac: "Mod+Alt+E", other: "Mod+Shift+E" }} platform="mac" />,
  },
  {
    component: "ShortcutChip",
    state: "Windows and Linux",
    element: () => <ShortcutChip keys={{ mac: "Mod+Alt+E", other: "Mod+Shift+E" }} platform="other" />,
  },
];

const feedback: Example[] = [
  ...(["neutral", "positive", "warning", "danger"] as const).map((tone) => ({
    component: "Notice",
    state: tone,
    element: () => (
      <Notice tone={tone} title="Approximate layout.">
        This file has no tags, so the reading order is a guess.
      </Notice>
    ),
  })),
  { component: "Toast", state: "plain", element: () => <Toast>Copied</Toast> },
  {
    component: "Toast",
    state: "with an action",
    element: () => <Toast action={{ label: "Undo", onPress: noop }}>Highlight deleted</Toast>,
  },
  {
    component: "EmptyState",
    state: "with an action",
    element: () => (
      <EmptyState icon="list" title="This file has no outline" actions={<Button variant="ghost">Show pages</Button>}>
        Use Pages to move through it.
      </EmptyState>
    ),
  },
  { component: "EmptyState", state: "title only", element: () => <EmptyState title="No attachments" /> },
  { component: "ProgressBar", state: "62%", element: () => <ProgressBar label="Saving…" value={62} /> },
  { component: "ProgressBar", state: "indeterminate", element: () => <ProgressBar label="Opening…" /> },
  {
    component: "Skeleton",
    state: "lines",
    element: () => (
      <div aria-busy="true">
        <Skeleton inlineSize="60%" />
        <Skeleton />
      </div>
    ),
  },
  ...(["neutral", "accent", "warning", "danger"] as const).map((tone) => ({
    component: "Badge",
    state: tone,
    element: () => <Badge tone={tone}>Approximate</Badge>,
  })),
  {
    component: "Badge",
    state: "positive with an icon",
    element: () => (
      <Badge tone="positive" icon="check">
        Signature valid
      </Badge>
    ),
  },
];

/** A store that always holds `state`. */
const holding = (state: ReaderState) => ({ getState: () => state, subscribe: () => () => undefined });
const file = fakeFile("Q3-board-report.pdf");
const LABELS = [...Array.from({ length: 21 }, (_, n) => String(n + 1)), "A-1", "A-2", "A-3"];
const MORE = { items: [{ id: "open", label: "Open another file…" }], onAction: noop };

/** The shell's handlers the top bar's toggle and Commands button run. */
const SHELL = { "view.toggle-sidebar": noop, "palette.open": noop };

const FIND_ACTIONS = {
  "find.next": noop,
  "find.previous": noop,
  "find.match-case": noop,
  "find.whole-words": noop,
  "find.close": noop,
};

const findBar: Example[] = [
  {
    component: "FindBar",
    state: "done: 3 of 9",
    element: () => {
      const reader = holding({ phase: "open", file, pageCount: 24, mode: "select" });
      const find = new FakeFind({
        query: "repairs",
        total: 9,
        current: 2,
        searched: 24,
        options: { wholeWords: true },
      });
      return (
        <WithCommands reader={reader} viewer={new FakeViewer()} actions={FIND_ACTIONS}>
          <FindBar find={find} onClose={noop} />
        </WithCommands>
      );
    },
  },
  {
    component: "FindBar",
    state: "counting, with pages without text",
    element: () => {
      const reader = holding({ phase: "open", file, pageCount: 24, mode: "select" });
      const find = new FakeFind({ query: "repairs", total: 40, searched: 14, done: false, pagesWithoutText: 4 });
      return (
        <WithCommands reader={reader} viewer={new FakeViewer()} actions={FIND_ACTIONS}>
          <FindBar find={find} onClose={noop} />
        </WithCommands>
      );
    },
  },
  {
    component: "FindBar",
    state: "no matches",
    element: () => {
      const reader = holding({ phase: "open", file, pageCount: 24, mode: "select" });
      const find = new FakeFind({ query: "backlogs", searched: 24 });
      return (
        <WithCommands reader={reader} viewer={new FakeViewer()} actions={FIND_ACTIONS}>
          <FindBar find={find} onClose={noop} />
        </WithCommands>
      );
    },
  },
];

/** A page's results heading, as the reader makes them. */
const resultHeading = (page: number) => (page === 2 ? "Page 3 · Repairs backlog" : `Page ${page + 1}`);
const RESULTS = [
  { ordinal: 0, page: 2, snippet: { before: "…the ", match: "repairs", after: " backlog for the third quarter…" } },
  { ordinal: 1, page: 2, snippet: { before: "Open ", match: "repairs", after: " are listed by site." } },
  { ordinal: 2, page: 6, snippet: { before: "…when ", match: "repairs", after: " ran over budget." } },
];

/** The History tab's steps, as O01 draws them. */
const STEPS = ["Rotate page 5", "Rotate page 2", "Rotate page 5", "Change the title"];
const AT = {
  formatTime: (at: number) => new Date(at).toISOString().slice(11, 16),
  openedAt: Date.UTC(2026, 10, 2, 16, 0),
};

const history: Example[] = [
  {
    component: "HistoryPanel",
    state: "three of four steps done",
    element: () => <HistoryPanel edits={new FakeEdits(STEPS, 3)} {...AT} />,
  },
  {
    component: "HistoryPanel",
    state: "an undo restored only part of a step",
    element: () => {
      const edits = new FakeEdits(STEPS, 2);
      const step = { id: 3, label: "Rotate page 5", pages: [4], at: 0, partial: "undo" as const };
      edits.set({ notice: { step, way: "undo", outcome: "partial" } });
      return <HistoryPanel edits={edits} {...AT} />;
    },
  },
  {
    component: "HistoryPanel",
    state: "no changes yet",
    element: () => <HistoryPanel edits={new FakeEdits()} {...AT} />,
  },
];

/** Two article threads, one with a part on a page the file lacks. */
const THREADS = [
  { title: "Repairs backlog in depth", parts: [{ page: 2 }, { page: 4 }, {}] },
  { parts: [{ page: 7 }] },
];

/** O14's files: a ZUGFeRD invoice's data, a scan a page holds, and a program. */
const FILES: AttachmentData[] = [
  {
    id: "e12.0",
    name: "factur-x.xml",
    description: "Invoice data",
    mime: "text/xml",
    size: 18_432,
    modified: "D:20261027091200Z",
    hasChecksum: true,
    relationship: "data",
    embedded: true,
    holders: [{ kind: "embeddedFiles" }, { kind: "document" }],
  },
  {
    id: "e14.0",
    name: "site-photo.jpg",
    mime: "image/jpeg",
    size: 2_411_520,
    hasChecksum: false,
    relationship: "source",
    embedded: true,
    holders: [{ kind: "page", page: 2 }],
  },
  {
    id: "e16.0",
    name: "setup.exe",
    mime: "application/octet-stream",
    size: 734_003,
    hasChecksum: true,
    relationship: "unspecified",
    embedded: true,
    holders: [{ kind: "embeddedFiles" }],
  },
];

/** The panel's callbacks, none of which the gallery acts on. */
const FILE_ACTIONS: Omit<AttachmentsPanelProps, "files" | "organise" | "selected"> = {
  onSelect: () => undefined,
  pageName: (index) => String(index + 1),
  saveLabel: "Download",
  onSave: () => undefined,
  onEditInOrganise: () => undefined,
  onAdd: () => undefined,
  onChange: () => undefined,
  onDelete: () => undefined,
  onGoToPage: () => undefined,
};

const attachments: Example[] = [
  {
    component: "AttachmentsPanel",
    state: "Read, the invoice data selected",
    element: () => (
      <AttachmentsPanel files={FILES} organise={false} selected="e12.0" checksum="matches" {...FILE_ACTIONS} />
    ),
  },
  {
    component: "AttachmentsPanel",
    state: "Organise, the invoice data selected",
    element: () => <AttachmentsPanel files={FILES} organise selected="e12.0" checksum="matches" {...FILE_ACTIONS} />,
  },
  {
    component: "AttachmentsPanel",
    state: "Organise, no files",
    element: () => <AttachmentsPanel files={[]} organise selected={undefined} {...FILE_ACTIONS} />,
  },
  {
    component: "AttachmentsPanel",
    state: "a checksum that doesn't match",
    element: () => (
      <AttachmentsPanel files={FILES} organise={false} selected="e12.0" checksum="mismatch" {...FILE_ACTIONS} />
    ),
  },
  {
    component: "AttachmentsPanel",
    state: "a program selected, with its warning badge",
    element: () => (
      <AttachmentsPanel files={FILES} organise={false} selected="e16.0" checksum="absent" {...FILE_ACTIONS} />
    ),
  },
  {
    component: "AttachmentsPanel",
    state: "a file too large to read here",
    element: () => (
      <AttachmentsPanel files={FILES} organise={false} selected="e14.0" tooLarge="64 MiB" {...FILE_ACTIONS} />
    ),
  },
];

const threads: Example[] = [
  {
    component: "ThreadsPanel",
    state: "following the first thread, its second part current",
    element: () => (
      <ThreadsPanel
        threads={THREADS}
        current={{ thread: 0, part: 1 }}
        pageName={(index) => String(index + 1)}
        step={(direction) => stepThreadPart(THREADS, { thread: 0, part: 1 }, direction)}
        onGo={() => undefined}
      />
    ),
  },
];

/** O11's bookmarks: one styled red and bold, one nested, one that goes to a name. */
const BOOKMARKS: BookmarkItem[] = [
  { id: "o4.0", title: "Chair's summary", page: "2", bold: true, color: "#c00000", children: [] },
  {
    id: "o5.0",
    title: "Repairs backlog",
    page: "5",
    open: true,
    children: [{ id: "o6.0", title: "By region", page: "6", children: [] }],
  },
  { id: "o7.0", title: "Surveyor capacity", destinationName: "surveyor-capacity", children: [] },
];

/** The editor's callbacks, none of which the gallery acts on. */
const BOOKMARK_ACTIONS = {
  apple: false,
  onNavigate: () => undefined,
  onAdd: () => undefined,
  onRename: () => undefined,
  onMove: () => undefined,
  onDelete: () => undefined,
  onStyle: () => undefined,
  onShowChildren: () => undefined,
  onUseCurrentView: () => undefined,
  onNamedDestinations: () => undefined,
};

const bookmarks: Example[] = [
  {
    component: "BookmarkEditor",
    state: "three bookmarks, one nested",
    element: () => <BookmarkEditor items={BOOKMARKS} current="o5.0" {...BOOKMARK_ACTIONS} />,
  },
  {
    component: "BookmarkEditor",
    state: "an outline the first edit rebuilt",
    element: () => <BookmarkEditor items={BOOKMARKS} repaired {...BOOKMARK_ACTIONS} />,
  },
  {
    component: "BookmarkEditor",
    state: "no bookmarks yet",
    element: () => <BookmarkEditor items={[]} {...BOOKMARK_ACTIONS} />,
  },
];

/** The link editor's handlers, which the examples never call. */
const LINK_ACTIONS = {
  pageCount: 12,
  names: ["chapter-2", "summary"],
  onGoal: () => undefined,
  onContents: () => undefined,
  onDelete: () => undefined,
  checkAddress: (address: string) => ({ ok: true, address }) as const,
};

const links: Example[] = [
  {
    component: "LinkEditor",
    state: "a link on text to a page",
    element: () => (
      <LinkEditor
        text="full report"
        goal={{ kind: "page", page: 4 }}
        contents="The full report"
        onFollow={() => undefined}
        {...LINK_ACTIONS}
      />
    ),
  },
  {
    component: "LinkEditor",
    state: "a new drawn link with nowhere to go yet",
    element: () => <LinkEditor drawn goal={undefined} contents="" {...LINK_ACTIONS} />,
  },
  {
    component: "LinkEditor",
    state: "a link read from a file that runs a script",
    element: () => <LinkEditor goal={{ kind: "script", action: "JavaScript" }} contents="" {...LINK_ACTIONS} />,
  },
];

const CERTIFICATION = { kind: "certification", field: "f7.0" };
const APPROVAL = { kind: "approval", field: "f9.0" };

function savePlan(constraint: SavePlanSlice["constraint"]): SavePlanSlice {
  const signed = constraint === "appendOnly" || constraint === "conflict";
  return {
    constraint,
    signatures: signed ? [CERTIFICATION, APPROVAL] : [],
    wouldInvalidate: constraint === "conflict" ? [CERTIFICATION, APPROVAL] : [],
  };
}

/** The strip under the top bar (O02) in each way a file saves that isn't plain. */
const saveStrip: Example[] = [
  {
    component: "SaveStrip",
    state: "signed: changes go after the signature",
    element: () => <SaveStrip plan={savePlan("appendOnly")} />,
  },
  {
    component: "SaveStrip",
    state: "the save rewrites the file",
    element: () => <SaveStrip plan={savePlan("requiresRewrite")} />,
  },
  {
    component: "SaveStrip",
    state: "the save would invalidate the certification",
    element: () => <SaveStrip plan={savePlan("conflict")} />,
  },
];

const findResults: Example[] = [
  {
    component: "FindResults",
    state: "results, the second current",
    element: () => {
      const find = new FakeFind({ query: "repairs", total: 3, current: 1, searched: 24 });
      find.list = RESULTS;
      return <FindResults find={find} heading={resultHeading} />;
    },
  },
  {
    component: "FindResults",
    state: "no matches, Whole words on",
    element: () => (
      <FindResults
        find={new FakeFind({ query: "backlogs", searched: 24, options: { wholeWords: true } })}
        heading={resultHeading}
      />
    ),
  },
];

const topBar: Example[] = [
  {
    component: "ReaderTopBar",
    state: "a document open",
    element: () => {
      const reader = holding({ phase: "open", file, pageCount: 24, mode: "select" });
      const viewer = new FakeViewer();
      return (
        <WithCommands reader={reader} viewer={viewer} actions={SHELL}>
          <ReaderTopBar reader={reader} viewer={viewer} keys="other" sidebarOpen more={MORE} />
        </WithCommands>
      );
    },
  },
  {
    component: "ReaderTopBar",
    state: "a document open, with the × that closes it (the web, R19)",
    element: () => {
      const reader = holding({ phase: "open", file, pageCount: 24, mode: "select" });
      const viewer = new FakeViewer();
      return (
        <WithCommands reader={reader} viewer={viewer} actions={{ ...SHELL, "file.close": noop }}>
          <ReaderTopBar reader={reader} viewer={viewer} keys="other" sidebarOpen more={MORE} />
        </WithCommands>
      );
    },
  },
  {
    component: "EditArea",
    state: "the editor's top bar: two unsaved changes, one to redo",
    element: () => {
      const reader = holding({ phase: "open", file, pageCount: 24, mode: "select" });
      const viewer = new FakeViewer();
      const edits = new FakeEdits(["Rotate page 5", "Rotate page 2", "Change the title"], 2);
      return (
        <WithCommands reader={reader} viewer={viewer} actions={SHELL} edits={edits}>
          <ReaderTopBar
            reader={reader}
            viewer={viewer}
            keys="other"
            sidebarOpen
            editArea={<EditArea edits={edits} keys="other" />}
            more={MORE}
          />
        </WithCommands>
      );
    },
  },
  {
    component: "ReaderTopBar",
    state: "after a jump: Back to page 2",
    element: () => {
      const reader = holding({ phase: "open", file, pageCount: 24, mode: "select" });
      const viewer = new FakeViewer({ currentPage: 10 });
      return (
        <WithCommands reader={reader} viewer={viewer} actions={{ ...SHELL, "nav.back": noop, "nav.forward": noop }}>
          <ReaderTopBar
            reader={reader}
            viewer={viewer}
            keys="other"
            sidebarOpen
            history={{ back: "2", forward: undefined }}
            more={MORE}
          />
        </WithCommands>
      );
    },
  },
  {
    component: "ReaderTopBar",
    state: "locked",
    element: () => {
      const reader = holding({ phase: "needs-password", file, wrong: false });
      return (
        <WithCommands reader={reader}>
          <ReaderTopBar reader={reader} keys="other" more={MORE} />
        </WithCommands>
      );
    },
  },
  {
    component: "ReaderTopBar",
    state: "no document",
    element: () => {
      const reader = holding({ phase: "empty" });
      return (
        <WithCommands reader={reader}>
          <ReaderTopBar reader={reader} keys="other" more={MORE} />
        </WithCommands>
      );
    },
  },
  {
    component: "PageInput",
    state: "a page label and its position",
    element: () => {
      const viewer = new FakeViewer({ currentPage: 21 });
      return (
        <WithCommands viewer={viewer} labels={LABELS}>
          <PageInput viewer={viewer} labels={LABELS} />
        </WithCommands>
      );
    },
  },
  {
    component: "ZoomControl",
    state: "at 110%",
    element: () => {
      const viewer = new FakeViewer();
      return (
        <WithCommands viewer={viewer}>
          <ZoomControl viewer={viewer} keys="other" />
        </WithCommands>
      );
    },
  },
  {
    component: "ZoomControl",
    state: "at the largest zoom",
    element: () => {
      const viewer = new FakeViewer({ percent: 800 });
      return (
        <WithCommands viewer={viewer}>
          <ZoomControl viewer={viewer} keys="other" />
        </WithCommands>
      );
    },
  },
];

const repairs: Example[] = [
  {
    component: "WholeFileDownload",
    state: "62% of a known size",
    element: () => <WholeFileDownload loaded={29.8 * 1024 * 1024} total={48 * 1024 * 1024} onCancel={noop} />,
  },
  {
    component: "RepairDetails",
    state: "repairs and a feature not run",
    element: () => (
      <RepairDetails
        fileName="Q3-board-report.pdf"
        repairs={[{ code: "xref-repaired" }, { code: "stream-length-recovered", pages: [3, 4, 9] }]}
        notRun={["This file contains JavaScript. Lamina PDF never runs it."]}
        onClose={noop}
        onCopy={noop}
      />
    ),
  },
  {
    component: "OpenFailure",
    state: "missing root",
    element: () => (
      <OpenFailure
        error={{ code: "missing-root", message: "" }}
        detail="1.8 MB read"
        onTryAnother={noop}
        onCopy={noop}
      />
    ),
  },
  {
    component: "RemoteFailure",
    state: "site blocks reading",
    element: () => (
      <RemoteFailure
        reason="cors"
        url="https://files.example.org/reports/Q3-board-report.pdf"
        onRetry={noop}
        onChooseFile={noop}
      />
    ),
  },
  {
    component: "RemoteFailure",
    state: "unencoded link",
    element: () => <RemoteFailure reason="not-encoded" onChooseFile={noop} onHome={noop} />,
  },
  {
    component: "OfflineNotice",
    state: "offline",
    element: () => <OfflineNotice onDismiss={noop} />,
  },
  {
    component: "RestoreNotice",
    state: "eight tabs, one file not found",
    element: () => <RestoreNotice tabs={8} missing={1} onShowMissing={noop} onDismiss={noop} />,
  },
  {
    component: "RestoreNotice",
    state: "every file found",
    element: () => <RestoreNotice tabs={3} missing={0} onShowMissing={noop} onDismiss={noop} />,
  },
  {
    component: "CrashNotice",
    state: "the document view restarted (S08), the report question not yet asked",
    element: () => <CrashNotice onSend={noop} onViewReport={noop} onDismiss={noop} />,
  },
  {
    component: "RecentList",
    state: "three files",
    element: () => (
      <RecentList
        entries={[
          {
            id: "a",
            name: "Q3-board-report.pdf",
            size: 1_262_480,
            lastOpened: 1_793_000_000_000,
            pages: 24,
            hasHandle: true,
          },
          {
            id: "b",
            name: "Tenant-survey-2026.pdf",
            size: 412_000,
            lastOpened: 1_792_900_000_000,
            pages: 6,
            hasHandle: true,
          },
          { id: "c", name: "Asset-register-summary.pdf", size: 88_000, lastOpened: 1_792_800_000_000, pages: 3 },
        ]}
        stoppedAt={new Map([["b", 3]])}
        onOpen={noop}
        onClear={noop}
        locale="en-GB"
      />
    ),
  },
  {
    component: "RecentList",
    state: "a file the file picker must find again, with Remove",
    element: () => (
      <RecentList
        entries={[
          {
            id: "a",
            name: "Q3-board-report.pdf",
            size: 1_262_480,
            lastOpened: 1_793_000_000_000,
            pages: 24,
            hasHandle: true,
          },
          {
            id: "c",
            name: "Asset-register-summary.pdf",
            size: 88_000,
            lastOpened: 1_792_800_000_000,
            pages: 3,
            hasHandle: false,
          },
        ]}
        onOpen={noop}
        onRemove={noop}
        onClear={noop}
        locale="en-GB"
      />
    ),
  },
  {
    component: "RecentList",
    // Only the document to continue, so the page shows one Recent list (the state above).
    state: "continue reading",
    element: () => {
      const report = {
        id: "a",
        name: "Q3-board-report.pdf",
        size: 1_262_480,
        lastOpened: 1_793_000_000_000,
        pages: 24,
      };
      return (
        <RecentList
          entries={[report]}
          continueReading={{ entry: report, page: 11, pageCount: 24 }}
          onOpen={noop}
          onClear={noop}
          locale="en-GB"
        />
      );
    },
  },
  {
    component: "NetworkActivityPanel",
    state: "file from a link",
    element: () => (
      <NetworkActivityPanel
        since="09:12"
        documentRequests={5}
        documentRows={[{ origin: "https://files.example.org", requests: 5, bytes: 327_680, unexpected: false }]}
        appRequests={11}
        appRows={[{ purpose: "webassembly", requests: 3, bytes: 4_812_000 }]}
        blocked={[]}
        fileHost="files.example.org"
        feeds={{ page: true, worker: true, serviceWorker: false }}
        onClose={noop}
        onCopy={noop}
      />
    ),
  },
];

/** A reflowed report: a heading, paragraphs, a figure kept as a crop, a list and a quote. */
const READER_BLOCKS = [
  { kind: "heading", level: 1, text: "Repairs backlog, third quarter", page: 2 },
  {
    kind: "paragraph",
    text: "Open repairs fell by a fifth over the quarter, most of them in the north and east, where two new crews started in July.",
    page: 2,
  },
  { kind: "figure", text: "", alt: "A bar chart of open repairs by region, falling in every region", page: 2 },
  { kind: "caption", text: "Figure 2. Open repairs by region at the end of each month.", page: 2 },
  { kind: "list-item", marker: "•", depth: 0, text: "North: 112 open, down from 150.", page: 2 },
  { kind: "list-item", marker: "•", depth: 0, text: "East: 87 open, down from 109.", page: 2 },
  { kind: "quote", text: "The new crews cleared the oldest jobs first, as the board asked.", page: 3 },
] as const;

/** The same report read from its layout: a page shown as its image after the reflowed text. */
const APPROXIMATE_BLOCKS = [
  ...READER_BLOCKS.slice(0, 4),
  { kind: "page-image", why: "uncertain", text: "", page: 3 },
] as const;

/** A crop as the app draws it: a block of the page's paper. */
const crop = () => <span className="paper-skeleton" aria-hidden="true" />;

const readerMode: Example[] = [
  {
    component: "ReaderView",
    state: "from the document's tags",
    element: () => (
      <ReaderView
        content={{ status: "ready", blocks: READER_BLOCKS, lang: "en" }}
        settings={DEFAULT_READER_SETTINGS}
        pageName="3"
        onBack={noop}
        renderCrop={crop}
      />
    ),
  },
  {
    component: "ReaderView",
    state: "approximate",
    element: () => (
      <ReaderView
        content={{ status: "ready", blocks: APPROXIMATE_BLOCKS, lang: "en", approximate: "untagged" }}
        settings={DEFAULT_READER_SETTINGS}
        pageName="3"
        onBack={noop}
        renderCrop={crop}
        onViewOriginal={noop}
        onOpenPage={noop}
      />
    ),
  },
  {
    component: "ReaderView",
    state: "not available",
    element: () => (
      <ReaderView
        content={{ status: "unavailable" }}
        settings={DEFAULT_READER_SETTINGS}
        pageName="3"
        onBack={noop}
        renderCrop={crop}
      />
    ),
  },
  {
    component: "ReaderSettingsPanel",
    state: "larger text, loose",
    element: () => (
      <ReaderSettingsPanel
        settings={{ textSize: 24, lineSpacing: "loose", lineLength: "narrow" }}
        onChange={noop}
        onClose={noop}
      />
    ),
  },
];

/** Every example, grouped as the design language's §7 orders the components. */
/** A two-level outline with labelled pages (the outline tree's states). */
const OUTLINE: readonly OutlineTreeItem[] = [
  { id: "0", title: "Summary", page: "i", children: [] },
  {
    id: "1",
    title: "2. Repairs backlog",
    page: "3",
    open: true,
    children: [
      { id: "1.0", title: "Open repairs by region", page: "3", children: [] },
      { id: "1.1", title: "Closed this quarter", page: "4", children: [] },
    ],
  },
  { id: "2", title: "3. Budget", page: "7", children: [{ id: "2.0", title: "Capital", page: "8", children: [] }] },
];

/** Six pages of the Organise grid: letter pages, one landscape, labelled by number. */
const GRID_PAGES = [1, 2, 3, 4, 5, 6].map((n) => ({
  id: `p${n}.0`,
  label: String(n),
  width: n === 4 ? 792 : 612,
  height: n === 4 ? 612 : 792,
  drawn: "0",
}));

/** The grid's handlers, which the examples never call (its pictures stay white). */
const GRID_ACTIONS = {
  "aria-label": "Pages",
  draw: async () => undefined,
  onSelectionChange: noop,
  onMove: noop,
  onCommand: noop,
  onOpen: noop,
  menu: [],
  onMenuAction: noop,
  platform: "other" as const,
};

/** The Organise mode's handlers for its page commands, so its toolbar's buttons are enabled. */
const ORGANISE_ACTIONS = Object.fromEntries(
  [
    "pages.insert-blank",
    "pages.delete",
    "pages.duplicate",
    "pages.reverse",
    "pages.move-to",
    "pages.select-all",
    "pages.rotate-left",
    "pages.rotate-right",
  ].map((id) => [id, noop]),
);

const organise: Example[] = [
  {
    component: "PageGrid",
    state: "two pages selected, one landscape",
    element: () => (
      <div style={{ blockSize: 480 }}>
        <PageGrid pages={GRID_PAGES} selected={new Set(["p2.0", "p3.0"])} {...GRID_ACTIONS} />
      </div>
    ),
  },
  {
    component: "PageGrid",
    state: "nothing selected, larger thumbnails",
    element: () => (
      <div style={{ blockSize: 480 }}>
        <PageGrid pages={GRID_PAGES} selected={new Set()} scale={1.25} {...GRID_ACTIONS} />
      </div>
    ),
  },
  {
    component: "OrganiseToolbar",
    state: "two pages selected",
    element: () => (
      <WithCommands edits={new FakeEdits()} organise actions={ORGANISE_ACTIONS}>
        <OrganiseToolbar note="2 pages selected" scale={1} onScale={noop} platform="other" />
      </WithCommands>
    ),
  },
];

/** The Comment mode's handlers for its tools, so its toolbar's tools are enabled. */
const COMMENT_ACTIONS = Object.fromEntries(
  [
    "select",
    "highlight",
    "underline",
    "strike",
    "caret",
    "note",
    "text-box",
    "callout",
    "typewriter",
    "ink",
    "shape",
    "highlighter",
    "eraser",
    "lasso",
    "draw-with-finger",
    "done",
  ].map((t) => [`comment.${t}`, noop]),
);

/** The swatches of a colour family, named and coloured as Comment mode gives them. */
const FILLS = FILL_COLOURS.map((id) => ({ id, label: id, color: `annotation-${id}` as const }));
const LINES = LINE_COLOURS.map((id) => ({ id, label: id, color: `content-line-${id}` as const }));

/** The touch bars' options for a pen: four colours, three widths, Medium chosen. */
const PEN_OPTIONS = {
  colours: LINES.slice(0, 4),
  colour: "blue" as const,
  onColour: noop,
  widths: [1.5, 3, 5] as const,
  width: 3,
  onWidth: noop,
};

/** A choice in the inspector, set to `value`. */
const choice = <T,>(value: T | undefined) => ({ value, onChange: noop });

/** What every inspector shows. */
const INSPECTED: Pick<AnnotationInspectorProps, "onDelete" | "opacity"> = { onDelete: noop, opacity: choice(100) };

/** A thread as the Comments panel shows it (C04). */
const THREAD: CommentThreadItem = {
  id: "a10.0",
  kindLabel: "Highlight",
  colour: "rgb(100% 85% 30%)",
  author: "R. Osei",
  when: "29 Oct 2026, 09:12",
  status: "accepted",
  statusBy: "A. Mensah",
  checked: false,
  quoted: "West is lowest because the July teams started there.",
  note: "Check the total",
  replies: [{ id: "a12.0", author: "A. Mensah", when: "29 Oct 2026, 10:01", text: "Moved the west team" }],
};

/** What every Comments panel example shares. */
const PANEL: CommentsPanelProps = {
  total: 3,
  open: 2,
  groups: [
    { title: "Page 3", threads: [THREAD, { ...THREAD, id: "a14.0", status: "open", replies: [], note: "Sign here" }] },
  ],
  query: "",
  onQuery: noop,
  sort: "page",
  onSort: noop,
  filters: [
    {
      id: "status",
      label: "Status",
      options: [
        { id: "open", label: "Open", count: 2, on: false },
        { id: "accepted", label: "Accepted", count: 1, on: false },
      ],
    },
  ],
  onFilter: noop,
  onClearFilters: noop,
  expanded: undefined,
  focus: undefined,
  onGo: noop,
  actions: { onReply: noop, onStatus: noop, onChecked: noop },
  selected: new Set(),
  onSelect: noop,
  onSetStatus: noop,
  onDelete: noop,
  broken: false,
  onClose: noop,
};

const comment: Example[] = [
  {
    component: "CommentsPanel",
    state: "threads by page, one open",
    element: () => <CommentsPanel {...PANEL} expanded="a10.0" />,
  },
  {
    component: "CommentsPanel",
    state: "two ticked, with the bulk footer and the broken-thread notice",
    element: () => <CommentsPanel {...PANEL} selected={new Set(["a10.0", "a14.0"])} broken />,
  },
  {
    component: "CommentsPanel",
    state: "a file without comments",
    element: () => <CommentsPanel {...PANEL} total={0} open={0} groups={[]} />,
  },
  {
    component: "CommentThreadDetail",
    state: "the phone sheet's thread: replies, Reply, Mark as checked and Status",
    element: () => <CommentThreadDetail thread={THREAD} actions={PANEL.actions} />,
  },
  {
    component: "CommentThreadSummary",
    state: "the phone sheet's thread heading: type, author, time, status and quoted text",
    element: () => <CommentThreadSummary thread={THREAD} />,
  },
  {
    component: "AnnotateBar",
    state: "the phone's Pen on, with its colours and widths above",
    element: () => (
      <WithCommands edits={new FakeEdits()} comment actions={COMMENT_ACTIONS}>
        <AnnotateBar tool="ink" options={PEN_OPTIONS} />
      </WithCommands>
    ),
  },
  {
    component: "AnnotateBar",
    state: "the Eraser on, which takes no options",
    element: () => (
      <WithCommands edits={new FakeEdits()} comment actions={COMMENT_ACTIONS}>
        <AnnotateBar tool="eraser" options={{ ...PEN_OPTIONS, colours: [], widths: undefined }} />
      </WithCommands>
    ),
  },
  {
    component: "OptionsStrip",
    state: "a pen's colours and widths",
    element: () => <OptionsStrip {...PEN_OPTIONS} />,
  },
  {
    component: "ToolShelf",
    state: "the tablet's Pen on, Draw with finger off",
    element: () => (
      <WithCommands edits={new FakeEdits()} comment actions={COMMENT_ACTIONS}>
        <ToolShelf tool="ink" options={PEN_OPTIONS} drawWithFinger={false} onDrawWithFinger={noop} />
      </WithCommands>
    ),
  },
  {
    component: "ToolShelf",
    state: "the Lasso on, Draw with finger on",
    element: () => (
      <WithCommands edits={new FakeEdits()} comment actions={COMMENT_ACTIONS}>
        <ToolShelf
          tool="lasso"
          options={{ ...PEN_OPTIONS, colours: [], widths: undefined }}
          drawWithFinger
          onDrawWithFinger={noop}
        />
      </WithCommands>
    ),
  },
  {
    component: "CommentToolbar",
    state: "Ink on, with its colour and width",
    element: () => (
      <WithCommands edits={new FakeEdits()} comment actions={COMMENT_ACTIONS}>
        <CommentToolbar
          tool="ink"
          shape="rectangle"
          text="textBox"
          colours={LINES}
          colour="blue"
          onColour={noop}
          width={2}
          onWidth={noop}
          count={2}
          hint="Drag to draw"
        />
      </WithCommands>
    ),
  },
  {
    component: "CommentToolbar",
    state: "Select on, with the page's count",
    element: () => (
      <WithCommands edits={new FakeEdits()} comment actions={COMMENT_ACTIONS}>
        <CommentToolbar
          tool="select"
          shape="ellipse"
          text="textBox"
          colours={[]}
          colour={undefined}
          onColour={noop}
          width={undefined}
          onWidth={noop}
          count={2}
          hint="Click a comment to select it"
        />
      </WithCommands>
    ),
  },
  {
    component: "AnnotationInspector",
    state: "a rectangle",
    element: () => (
      <AnnotationInspector
        {...INSPECTED}
        title="Rectangle"
        context="Page 3"
        colourLabel="Line colour"
        colours={LINES}
        colour={choice("blue" as const)}
        fill={{ colours: FILLS, ...choice("none" as const) }}
        width={choice(1.5)}
        border={choice("cloudy" as const)}
        cloudy
        note={{ value: "West is lowest because the July teams started there.", onCommit: noop }}
        author="R. Osei · 29 Oct 2026, 09:12"
        onChangeAuthor={noop}
        deleteLabel="Delete rectangle"
      />
    ),
  },
  {
    component: "AnnotationInspector",
    state: "a note",
    element: () => (
      <AnnotationInspector
        {...INSPECTED}
        title="Note"
        context="Page 1"
        colourLabel="Colour"
        colours={FILLS}
        colour={choice("yellow" as const)}
        note={{ value: "Check this figure", onCommit: noop }}
        author="Ada · 16 Nov 2026, 22:52"
        onChangeAuthor={noop}
        deleteLabel="Delete note"
      />
    ),
  },
  {
    component: "AnnotationInspector",
    state: "three comments selected",
    element: () => (
      <AnnotationInspector
        onDelete={noop}
        opacity={choice<number>(undefined)}
        title="3 comments"
        colourLabel="Colour"
        colours={LINES}
        colour={choice<"blue">(undefined)}
        deleteLabel="Delete 3 comments"
      />
    ),
  },
];

/** Fill & Sign's handlers for its toolbar's commands, so its buttons are enabled. */
const FORM_ACTIONS = Object.fromEntries(
  ["form.focus-next-field", "form.focus-previous-field", "form.toggle-field-highlight"].map((id) => [id, noop]),
);

/** A form's rows in the Fields tab: filled, required and locked. */
const FIELD_ROWS = [
  { widget: "w1", field: "f1", page: 0, label: "Your name", filled: true, required: true },
  { widget: "w2", field: "f2", page: 0, label: "Address", filled: false, required: true },
  {
    widget: "w3",
    field: "f3",
    page: 0,
    label: "Agree",
    filled: false,
    required: false,
    locked: "Locked by the signature in “Sig1”",
  },
  { widget: "w4", field: "f4", page: 1, label: "Notes", filled: false, required: false },
];

/** Fill & Sign (P01): its toolbar, the Fields tab and the question before a locked field changes. */
const forms: Example[] = [
  {
    component: "FormToolbar",
    state: "fields highlighted, one of two required fields filled",
    element: () => (
      <WithCommands edits={new FakeEdits()} form actions={FORM_ACTIONS}>
        <FormToolbar highlight required={{ filled: 1, total: 2 }} />
      </WithCommands>
    ),
  },
  {
    component: "FieldsPanel",
    state: "page 1 open, a field focused and one locked",
    element: () => (
      <FieldsPanel rows={FIELD_ROWS} current="f1" page={0} pageName={(i) => String(i + 1)} onFocus={noop} />
    ),
  },
  {
    component: "FieldsPanel",
    state: "no fields",
    element: () => (
      <FieldsPanel rows={[]} current={undefined} page={0} pageName={(i) => String(i + 1)} onFocus={noop} />
    ),
  },
  {
    component: "FieldConflictDialog",
    state: "a locked field",
    element: () => <FieldConflictDialog question={{ field: "Your name", kind: "locked" }} onAnswer={noop} />,
  },
];

export const EXAMPLES: readonly Example[] = [
  ...topBar,
  ...findBar,
  ...findResults,
  ...history,
  ...threads,
  ...attachments,
  ...bookmarks,
  ...links,
  ...organise,
  ...comment,
  ...forms,
  ...saveStrip,
  ...buttons,
  ...documentTabs,
  ...toggles,
  ...rail,
  ...status,
  ...fields,
  ...choices,
  ...pickers,
  ...feedback,
  ...repairs,
  ...readerMode,
  ...inspectorExamples,
];
