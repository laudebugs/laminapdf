// Every component in each state it can be put in through props, for the axe and size tests here
// and for the gallery's screenshots later. Hover and pressed come from real pointer events in
// the tests that need them, not from props, so shipped code carries nothing for tests.

import type { ReactElement } from "react";
import {
  Badge,
  Button,
  Checkbox,
  EmptyState,
  IconButton,
  ModeSwitcher,
  NetworkIndicator,
  Notice,
  OutlineRow,
  PageAppearanceChips,
  PageInput,
  PageLabel,
  ProgressBar,
  Radio,
  RadioGroup,
  type ReaderState,
  ReaderTopBar,
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
  ZoomControl,
} from "../src/index.ts";
import { FakeViewer, fakeFile } from "./fakes.ts";

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

const topBar: Example[] = [
  {
    component: "ReaderTopBar",
    state: "a document open",
    element: () => (
      <ReaderTopBar
        reader={holding({ phase: "open", file, pageCount: 24, mode: "select" })}
        viewer={new FakeViewer()}
        keys="other"
        sidebar={{ isOpen: true, onChange: noop }}
        commands={{ onPress: noop, shortcut: { mac: "Mod+K", other: "Mod+K" } }}
        more={MORE}
      />
    ),
  },
  {
    component: "ReaderTopBar",
    state: "locked",
    element: () => (
      <ReaderTopBar reader={holding({ phase: "needs-password", file, wrong: false })} keys="other" more={MORE} />
    ),
  },
  {
    component: "ReaderTopBar",
    state: "no document",
    element: () => <ReaderTopBar reader={holding({ phase: "empty" })} keys="other" more={MORE} />,
  },
  {
    component: "PageInput",
    state: "a page label and its position",
    element: () => <PageInput viewer={new FakeViewer({ currentPage: 21 })} labels={LABELS} />,
  },
  {
    component: "PageInput",
    state: "disabled",
    element: () => <PageInput viewer={new FakeViewer()} isDisabled />,
  },
  { component: "ZoomControl", state: "at 110%", element: () => <ZoomControl viewer={new FakeViewer()} keys="other" /> },
  {
    component: "ZoomControl",
    state: "at the largest zoom",
    element: () => <ZoomControl viewer={new FakeViewer({ percent: 800 })} keys="other" />,
  },
];

/** Every example, grouped as the design language's §7 orders the components. */
export const EXAMPLES: readonly Example[] = [
  ...topBar,
  ...buttons,
  ...toggles,
  ...rail,
  ...status,
  ...fields,
  ...choices,
  ...pickers,
  ...feedback,
];
