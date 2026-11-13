// Every component in each state it can be put in through props, for the axe and size tests here
// and for the gallery's screenshots later. Hover and pressed come from real pointer events in
// the tests that need them, not from props, so shipped code carries nothing for tests.

import type { ReactElement } from "react";
import { ThreadsPanel } from "../src/dialogs.ts";
import {
  BookmarkEditor,
  type BookmarkItem,
  EditArea,
  HistoryPanel,
  LinkEditor,
  type SavePlanSlice,
  SaveStrip,
} from "../src/editing.ts";
import {
  Badge,
  Button,
  Checkbox,
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
  { component: "Button", state: "pending", element: () => <Button isPending>Saving</Button> },
  ...([26, 28, 32] as const).map((size) => ({
    component: "IconButton",
    state: `size ${size}`,
    element: () => <IconButton label="Find in document" icon="find" size={size} />,
  })),
  { component: "IconButton", state: "disabled", element: () => <IconButton label="Undo" icon="undo" isDisabled /> },
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
    component: "TextArea",
    state: "with text",
    element: () => <TextArea label="Note" defaultValue="Check the totals." />,
  },
  {
    component: "Select",
    state: "chosen",
    element: () => <Select label="Paper size" options={OPTIONS} value="a4" onChange={noop} />,
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

export const EXAMPLES: readonly Example[] = [
  ...topBar,
  ...findBar,
  ...findResults,
  ...history,
  ...threads,
  ...bookmarks,
  ...links,
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
];
