// Renders the site header in Node with Astro's container API, once for the website and once for
// the docs site (with a tools slot), and hands both to the browser tests as HTML.
import { experimental_AstroContainer as AstroContainer } from "astro/container";
import type { TestProject } from "vitest/node";
import { siteNav } from "../../src/nav.mjs";
import SiteHeader from "../../src/SiteHeader.astro";

/** The website's page files in these tests: three of §7.39's links have their page. */
const PAGES = ["/src/pages/index.astro", "/src/pages/download.astro", "/src/pages/privacy/index.astro"];

declare module "vitest" {
  export interface ProvidedContext {
    headers: { website: string; docs: string };
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
}
