// Turns each code panel (`[data-tabs]`, CodePanel.astro) into a tab list, following the WAI-ARIA
// tabs pattern with automatic activation: click or arrow keys pick a tab, Home and End jump to the
// ends, and only the selected tab is in the tab order. Without this script every pane shows under
// its own heading.

const KEYS: Readonly<Record<string, (index: number, count: number) => number>> = {
  ArrowRight: (index, count) => (index + 1) % count,
  ArrowLeft: (index, count) => (index - 1 + count) % count,
  Home: () => 0,
  End: (_index, count) => count - 1,
};

function enhance(panel: HTMLElement): void {
  const list = panel.querySelector<HTMLElement>("[data-tablist]");
  const tabs = [...panel.querySelectorAll<HTMLButtonElement>("[data-tab]")];
  const panes = [...panel.querySelectorAll<HTMLElement>("[data-pane]")];
  if (list === null || tabs.length === 0 || tabs.length !== panes.length) return;

  const select = (index: number, focus: boolean): void => {
    for (const [i, tab] of tabs.entries()) {
      const selected = i === index;
      tab.setAttribute("aria-selected", String(selected));
      tab.tabIndex = selected ? 0 : -1;
      const pane = panes[i];
      if (pane !== undefined) pane.hidden = !selected;
    }
    if (focus) tabs[index]?.focus();
  };

  list.setAttribute("role", "tablist");
  list.hidden = false;
  for (const [i, tab] of tabs.entries()) {
    const pane = panes[i];
    if (pane === undefined) continue;
    tab.setAttribute("role", "tab");
    tab.setAttribute("aria-controls", pane.id);
    pane.setAttribute("role", "tabpanel");
    pane.setAttribute("aria-labelledby", tab.id);
    pane.querySelector(".code-label")?.setAttribute("hidden", "");
    tab.addEventListener("click", () => select(i, false));
    tab.addEventListener("keydown", (event) => {
      const next = KEYS[event.key];
      if (next === undefined) return;
      event.preventDefault();
      select(next(i, tabs.length), true);
    });
  }
  select(0, false);
}

for (const panel of document.querySelectorAll<HTMLElement>("[data-tabs]")) enhance(panel);
