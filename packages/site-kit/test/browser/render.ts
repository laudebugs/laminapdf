// Renders the site header in Node with Astro's container API, once for the website and once for
// the docs site (with a tools slot), and the select with its label shown and hidden, and hands them
// to the browser tests as HTML.
import { readFileSync } from "node:fs";
import { experimental_AstroContainer as AstroContainer } from "astro/container";
import type { TestProject } from "vitest/node";
import MenuSelect from "../../src/MenuSelect.astro";
import { siteNav } from "../../src/nav.mjs";
import SiteHeader from "../../src/SiteHeader.astro";

/** The website's page files in these tests: three of §7.39's links have their page. */
const PAGES = [
  "/src/pages/index.astro",
  "/src/pages/download.astro",
  "/src/pages/how-we-test.astro",
  "/src/pages/privacy/index.astro",
];

declare module "vitest" {
  export interface ProvidedContext {
    headers: { website: string; docs: string };
    selects: { shown: string; hidden: string };
  }
}

export default async function setup(project: TestProject): Promise<void> {
  const container = await AstroContainer.create();
  const website = await container.renderToString(SiteHeader, {
    props: { links: siteNav(PAGES, { site: "website", path: "/privacy/" }), home: "/" },
  });
  const docs = await container.renderToString(SiteHeader, {
    // As the docs site renders it: Starlight's phone menu, not the header's.
    props: { links: siteNav(PAGES, { site: "docs", path: "/guides/" }), home: "https://laminapdf.com/", menu: false },
    slots: { tools: '<button type="button">Search</button>' },
  });
  project.provide("headers", { website, docs });
  const icon = (name: string) => readFileSync(new URL(`../../../icons/svg/${name}.svg`, import.meta.url), "utf8");
  const options = [
    { value: "light", label: "Light", icon: icon("sun") },
    { value: "dark", label: "Dark", icon: icon("moon") },
    { value: "auto", label: "Auto", icon: icon("laptop") },
  ];
  const shown = await container.renderToString(MenuSelect, { props: { label: "Theme", options, value: "auto" } });
  const hidden = await container.renderToString(MenuSelect, {
    props: { label: "Select theme", hideLabel: true, options, value: "auto" },
  });
  project.provide("selects", { shown, hidden });
}
