// Renders the site header in Node with Astro's container API, once for the website and once for
// the docs site (with a tools slot), and the select with its label shown and hidden, and hands them
// to the browser tests as HTML.
import { experimental_AstroContainer as AstroContainer } from "astro/container";
import type { TestProject } from "vitest/node";
import { siteNav } from "../../src/nav.mjs";
import Select from "../../src/Select.astro";
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
    props: { links: siteNav(PAGES, { site: "docs", path: "/guides/" }), home: "https://laminapdf.com/" },
    slots: { tools: '<button type="button">Search</button>' },
  });
  project.provide("headers", { website, docs });
  const options = [
    { value: "dark", label: "Dark" },
    { value: "light", label: "Light" },
    { value: "auto", label: "Auto", selected: true },
  ];
  const shown = await container.renderToString(Select, { props: { label: "Theme", options } });
  const hidden = await container.renderToString(Select, { props: { label: "Select theme", hideLabel: true, options } });
  project.provide("selects", { shown, hidden });
}
