// Where the docs keep a reader's choice to hide "On this page" (todo Q4v): one name for the two
// scripts that use it, Head.astro's (which applies the choice before the first paint) and
// PageSidebar.astro's (the button that changes it).

/** The localStorage key, and the value that means "hidden" (anything else shows the column). */
export const TOC_STORAGE = { key: "lamina-docs-toc", hidden: "hidden", shown: "shown" } as const;
