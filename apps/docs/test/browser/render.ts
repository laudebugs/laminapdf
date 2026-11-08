// Renders every diagram in Node with Astro's container API, through the same Diagram.astro the
// pages use, and hands the HTML to the browser tests.
import { experimental_AstroContainer as AstroContainer } from "astro/container";
import type { TestProject } from "vitest/node";
import Diagram from "../../src/components/Diagram.astro";
import { DIAGRAMS } from "../../src/components/diagrams/registry.mjs";

declare module "vitest" {
  export interface ProvidedContext {
    diagrams: Record<string, string>;
  }
}

export default async function setup(project: TestProject): Promise<void> {
  const container = await AstroContainer.create();
  const diagrams: Record<string, string> = {};
  for (const name of Object.keys(DIAGRAMS)) {
    diagrams[name] = await container.renderToString(Diagram, { props: { name } });
  }
  project.provide("diagrams", diagrams);
}
