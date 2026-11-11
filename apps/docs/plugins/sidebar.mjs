// The docs site's sidebar (astro.config.mjs): the order is a decision (the docs as a story,
// quality plan §3), so every page is listed. llms.txt (agent-docs.mjs) follows the same order.

/**
 * The sidebar's groups, with the TypeDoc reference group (`typeDoc`) when the build has one.
 *
 * @param {unknown} [typeDoc]
 * @returns {any[]}
 */
export function sidebar(typeDoc) {
  return [
    {
      label: "Start here",
      items: [
        { label: "What is Lamina PDF", link: "/" },
        { label: "Choosing a PDF library", slug: "start/choosing-a-pdf-library" },
        { label: "Install", slug: "install" },
        { label: "Your first PDF", slug: "start/first-pdf" },
        { label: "Use with coding agents", slug: "start/coding-agents" },
      ],
    },
    {
      label: "Your runtime",
      items: [
        { label: "Browser and bundlers", slug: "quick-start/browser" },
        { label: "Node", slug: "quick-start/node" },
        { label: "Deno", slug: "quick-start/deno" },
        { label: "Bun", slug: "quick-start/bun" },
        { label: "Cloudflare Workers", slug: "quick-start/workers" },
        { label: "Rust", slug: "quick-start/rust" },
      ],
    },
    {
      label: "Concepts",
      items: [
        { label: "How Lamina PDF thinks about a PDF", slug: "concepts/how-lamina-thinks" },
        { label: "Pages and coordinates", slug: "concepts/pages-and-coordinates" },
        { label: "Errors and warnings", slug: "concepts/errors-and-warnings" },
        { label: "What Lamina PDF writes and what it borrows", slug: "concepts/dependencies" },
        { label: "How a page is drawn", slug: "concepts/drawing-pages" },
      ],
    },
    {
      label: "Create PDFs",
      items: [
        { label: "Text and fonts", slug: "guides/fonts-and-text" },
        { label: "Shapes, colour and images", slug: "guides/images-and-vectors" },
        { label: "Links, outlines and metadata", slug: "guides/links-outlines-and-metadata" },
        { label: "Pages that flow", slug: "guides/flowing-pages" },
        { label: "Accessible PDFs", slug: "guides/accessible-pdfs" },
        { label: "Archival PDFs", slug: "guides/archival-pdfs" },
      ],
    },
    {
      label: "Read and edit PDFs",
      items: [
        { label: "Edit a PDF", slug: "guides/edit-a-pdf" },
        { label: "Save a PDF", slug: "guides/save-a-pdf" },
        { label: "Open password-protected files", slug: "guides/password-protected-files" },
        { label: "Open files you don't trust", slug: "guides/untrusted-files" },
        { label: "Open large files by ranges", slug: "guides/open-large-files" },
      ],
    },
    {
      label: "Command line",
      items: [
        { label: "The command line", slug: "cli" },
        { label: "Install the tool", slug: "cli/install" },
        { label: "Inspect and check PDFs", slug: "cli/inspect" },
        { label: "Extract text and render pages", slug: "cli/extract-and-render" },
        { label: "Make and change PDFs", slug: "cli/build-and-edit" },
        { label: "Scripts and batch jobs", slug: "cli/scripts" },
        { label: "Connect an AI agent (MCP)", slug: "cli/mcp" },
      ],
    },
    {
      label: "Production",
      items: [
        { label: "Keep the page responsive", slug: "guides/keep-the-page-responsive" },
        { label: "Test the PDFs you generate", slug: "guides/test-generated-pdfs" },
      ],
    },
    {
      label: "Recipes",
      items: [
        { label: "All recipes", slug: "recipes" },
        { label: "Hello, world", slug: "recipes/hello" },
        { label: "An invoice", slug: "recipes/invoice" },
        { label: "Multilingual text", slug: "recipes/multilingual" },
        { label: "Images and vectors", slug: "recipes/images-and-vectors" },
        { label: "Element flow", slug: "recipes/element-flow" },
        { label: "Links and outlines", slug: "recipes/links-and-outlines" },
        { label: "Generate, then edit", slug: "recipes/generate-then-edit" },
        { label: "Extract text for an LLM", slug: "recipes/extract-markdown" },
        { label: "A PDF/UA report", slug: "recipes/pdf-ua-report" },
        { label: "A PDF/A archive", slug: "recipes/pdf-a-archive" },
        { label: "A PDF/A-3 invoice with XML", slug: "recipes/pdf-a3-invoice-with-xml" },
        { label: "A 100-page report", slug: "recipes/report" },
        { label: "Open someone else's file", slug: "recipes/open-untrusted" },
        { label: "Save in a Web Worker", slug: "recipes/web-worker-client" },
        { label: "A Cloudflare Worker", slug: "recipes/cloudflare-worker" },
      ],
    },
    { label: "Troubleshooting", slug: "troubleshooting" },
    {
      label: "Reference",
      items: [
        ...(typeDoc ? [typeDoc] : []),
        { label: "Rust API (rustdoc)", slug: "reference/rust" },
        { label: "Error and warning codes", slug: "reference/codes" },
        { label: "Stability policy", slug: "reference/stability" },
        { label: "Benchmarks", slug: "reference/benchmarks" },
        { label: "Changelog", slug: "reference/changelog" },
        { label: "Glossary", slug: "reference/glossary" },
      ],
    },
  ];
}

/**
 * The sidebar's groups as routes (`guides/save-a-pdf/`, `` for the home page), the generated
 * reference left out.
 *
 * @returns {{ label: string, slugs: string[] }[]}
 */
export function sidebarRoutes() {
  /** @param {any} item */
  const route = (item) => (item.link ? item.link.replace(/^\//, "") : `${item.slug}/`);
  return sidebar().flatMap((entry) =>
    entry.items
      ? [{ label: entry.label, slugs: entry.items.map(route) }]
      : [{ label: entry.label, slugs: [route(entry)] }],
  );
}
