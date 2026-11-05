// Segmented tabs (§7.5): the rail's tabs, over React Aria's Tabs. Arrow keys move between tabs and
// select as they go (automatic activation, since a rail panel shows at once); Home and End jump to
// the ends. `keyboardActivation="manual"` keeps selection on Enter and Space for slow panels.

import type { ReactNode } from "react";
import {
  TabPanel as AriaTabPanel,
  type TabPanelProps as AriaTabPanelProps,
  Tab,
  TabList,
  Tabs,
} from "react-aria-components";

/** One tab: its id, shared with its `TabPanel`, and its visible name. */
export interface TabItem<Id extends string> {
  /** The tab's id. */
  id: Id;
  /** The visible name ("Pages", "Outline"). */
  label: string;
}

/** Props of `SegmentedTabs`. */
export interface SegmentedTabsProps<Id extends string> {
  /** The tabs, in order. */
  tabs: readonly TabItem<Id>[];
  /** The selected tab. */
  selectedKey: Id;
  /** Called with a tab when the user selects it. */
  onSelectionChange: (tab: Id) => void;
  /** The tab list's accessible name ("Sidebar"). */
  "aria-label": string;
  /** Whether arrow keys select (`automatic`, the default) or only move focus (`manual`). */
  keyboardActivation?: "automatic" | "manual";
  /** The panels: one `TabPanel` per tab. */
  children: ReactNode;
}

/** A row of tabs with an accent underline on the selected one, and its panels below. */
export function SegmentedTabs<Id extends string>({
  tabs,
  selectedKey,
  onSelectionChange,
  "aria-label": label,
  keyboardActivation = "automatic",
  children,
}: SegmentedTabsProps<Id>): ReactNode {
  return (
    <Tabs
      className="paper-tabs"
      selectedKey={selectedKey}
      onSelectionChange={(key) => onSelectionChange(key as Id)}
      keyboardActivation={keyboardActivation}
    >
      <TabList aria-label={label} className="paper-tab-list">
        {tabs.map((tab) => (
          <Tab key={tab.id} id={tab.id} className="paper-tab">
            {tab.label}
          </Tab>
        ))}
      </TabList>
      {children}
    </Tabs>
  );
}

/** Props of `TabPanel`. */
export interface TabPanelProps extends Omit<AriaTabPanelProps, "className" | "style"> {
  /** The id of the tab that shows this panel. */
  id: string;
}

/** The panel a tab shows. */
export function TabPanel(props: TabPanelProps): ReactNode {
  return <AriaTabPanel {...props} className="paper-tab-panel" />;
}
