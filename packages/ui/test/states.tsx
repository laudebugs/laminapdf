// Every component in each state it can be put in through props, for the axe and size tests here
// and for the gallery's screenshots later. Hover and pressed come from real pointer events in
// the tests that need them, not from props, so shipped code carries nothing for tests.

import type { ReactElement } from "react";
import {
  Button,
  Checkbox,
  IconButton,
  ModeSwitcher,
  PageAppearanceChips,
  Radio,
  RadioGroup,
  SegmentedTabs,
  Select,
  SwatchGroup,
  Switch,
  TabPanel,
  TextArea,
  TextInput,
  ToggleButton,
  ToolToggle,
} from "../src/index.ts";

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

/** Every example, grouped as the design language's §7 orders the components. */
export const EXAMPLES: readonly Example[] = [...buttons, ...toggles, ...fields, ...choices, ...pickers];
