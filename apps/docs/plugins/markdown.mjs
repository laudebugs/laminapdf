// The site's pages as plain Markdown, for agents (todo L5a): every page's MDX source with its
// components replaced by what they show. `<Snippet file>` becomes a fenced block holding the
// example file, so a twin needs nothing else to be read; `<Diagram name>` becomes its caption and
// text description; `<Tabs>` become a bold label per tab;
// Starlight asides (`:::note[Title]`) become block quotes; links to the site's pages point at
// their Markdown twins. Pure functions over strings, tested by markdown.test.mjs.

import { withVersion } from "@laminapdf/site-kit/version";
import { diagramText } from "../src/components/diagrams/registry.mjs";

/** The fence language of a file the docs show, by extension. */
const LANGUAGES = { ts: "ts", rs: "rust", toml: "toml", json: "json", md: "md" };

/** A `<Snippet … file="…" … />`, whatever its attributes' order; group 1 is the file. */
const SNIPPET = /<Snippet\b[^>]*?\bfile="([^"]+)"[^>]*\/>/g;

/** A `<Transcript name="…" />` (todo Q4a); group 1 is the transcript's name. */
export const TRANSCRIPT = /<Transcript\s+name="([a-z0-9-]+)"\s*\/>/g;

/**
 * The file a `<Transcript>` shows, from the repository root.
 *
 * @param {string} name
 * @returns {string}
 */
export function transcriptFile(name) {
  return `fixtures/cli/transcripts/${name}.console`;
}

/** An `<Example … />` (todo B7d); group 1 is its attributes. */
const EXAMPLE = /<Example\b([^>]*?)\/>/g;

/**
 * What an `<Example>`'s attributes name: its code files (`ts`, `rust`), the PDFs it makes
 * (`result="hello"` or `result={["a", "b"]}`) and the text file it prints (`output`).
 *
 * @param {string} attributes
 * @returns {{ ts: string | undefined, rust: string | undefined, results: string[], output: string | undefined }}
 */
export function exampleParts(attributes) {
  /** @param {string} name */
  const value = (name) => new RegExp(`(?:^|\\s)${name}="([^"]+)"`).exec(attributes)?.[1];
  const list = /(?:^|\s)result=\{\[([^\]]*)\]\}/.exec(attributes)?.[1];
  const one = value("result");
  const results = list !== undefined ? [...list.matchAll(/"([^"]+)"/g)].map((m) => m[1] ?? "") : one ? [one] : [];
  return { ts: value("ts"), rust: value("rust"), results, output: value("output") };
}

/**
 * Every file a page's `<Snippet>`s, `<Example>`s and `<Transcript>`s show, for the twins to read before they write.
 *
 * @param {string} source
 * @returns {string[]}
 */
export function shownFiles(source) {
  const files = [...source.matchAll(SNIPPET)].map((m) => m[1] ?? "");
  for (const [, name = ""] of source.matchAll(TRANSCRIPT)) files.push(transcriptFile(name));
  for (const [, attributes = ""] of source.matchAll(EXAMPLE)) {
    const { ts, rust, output } = exampleParts(attributes);
    for (const file of [ts, rust, output]) if (file !== undefined) files.push(file);
  }
  return files;
}

/**
 * An `<Example>` as Markdown: each language's code under its name, then the PDFs it makes as
 * links (the page shows them in a Result tab) or the text it prints.
 *
 * @param {string} attributes
 * @param {(file: string) => string} snippet
 * @returns {string}
 */
function exampleMarkdown(attributes, snippet) {
  const { ts, rust, results, output } = exampleParts(attributes);
  const parts = [];
  if (ts !== undefined) parts.push(`**TypeScript**\n\n${codeBlock(ts, snippet(ts))}`);
  if (rust !== undefined) parts.push(`**Rust**\n\n${codeBlock(rust, snippet(rust))}`);
  if (results.length > 0) {
    parts.push(`**Result:** ${results.map((stem) => `[${stem}.pdf](/examples/pdf/${stem}.pdf)`).join(", ")}`);
  }
  if (output !== undefined) parts.push(`**Output**\n\n${codeBlock(output, snippet(output))}`);
  return parts.join("\n\n");
}

/**
 * A `<Diagram … name="…" … />` (or `<Diagram name="…"></Diagram>`), whatever its attributes and
 * quotes; group 2 is the name (src/components/diagrams/registry.mjs). test/diagrams.test.mjs uses
 * it too, so the pages' check and the twin see the same diagrams.
 */
export const DIAGRAM = /<Diagram\b[^>]*?\sname=(["'])([^"']+)\1[^>]*?(?:\/>|>\s*<\/Diagram>)/g;

/**
 * A diagram as its twin shows it: the caption in italics, then the text description, which says
 * all the picture says. An unknown name throws, as the page's build does.
 *
 * @param {string} name
 * @returns {string}
 */
function diagramMarkdown(name) {
  const { caption, text } = diagramText(name);
  return `*${caption}*\n\n${text}`;
}

/**
 * Splits a page into its front matter (simple `key: value` lines, quotes removed) and body.
 *
 * @param {string} source
 * @returns {{ data: Record<string, string>, body: string }}
 */
export function frontMatter(source) {
  const match = /^---\r?\n([\s\S]*?)\r?\n---\r?\n?/.exec(source);
  if (!match) return { data: {}, body: source };
  /** @type {Record<string, string>} */
  const data = {};
  for (const line of (match[1] ?? "").split(/\r?\n/)) {
    const pair = /^([A-Za-z][\w-]*):\s*(.*)$/.exec(line);
    if (!pair) continue;
    const [, key = "", raw = ""] = pair;
    data[key] = raw.replace(/^(["'])(.*)\1$/, "$2");
  }
  return { data, body: source.slice(match[0].length) };
}

/**
 * The longest run of backticks in `text`, plus one (at least three), so a fence around it can't
 * be closed by a line inside it.
 *
 * @param {string} text
 * @returns {string}
 */
function fenceFor(text) {
  const longest = Math.max(2, ...[...text.matchAll(/`+/g)].map((m) => m[0].length));
  return "`".repeat(longest + 1);
}

/**
 * A terminal session as a `console` block, without a caption: the page shows it the same way.
 *
 * @param {string} text
 * @returns {string}
 */
function consoleBlock(text) {
  const fence = fenceFor(text);
  return `${fence}console\n${text.replace(/\n$/, "")}\n${fence}`;
}

/**
 * A fenced block of `file`'s contents, captioned with its path.
 *
 * @param {string} file the path from the repository root
 * @param {string} code
 * @returns {string}
 */
export function codeBlock(file, code) {
  const extension = file.slice(file.lastIndexOf(".") + 1);
  const lang = LANGUAGES[/** @type {keyof typeof LANGUAGES} */ (extension)] ?? "text";
  const fence = fenceFor(code);
  return `\`${file}\`:\n\n${fence}${lang}\n${code.replace(/\n$/, "")}\n${fence}`;
}

/**
 * Where the page at `pathname` has its Markdown twin (`/guides/x/` → `/guides/x.md`, `/` →
 * `/index.md`), or `undefined` for the 404 page, which has none. The head's alternate link, "Copy as
 * Markdown" and the build's check of that link all ask this.
 *
 * @param {string} pathname a page's path on the site
 * @returns {string | undefined}
 */
export function pageTwin(pathname) {
  if (/^\/404(\.html|\/)?$/.test(pathname)) return undefined;
  return twinHref(pathname.endsWith("/") ? pathname : `${pathname}/`);
}

/**
 * A site link as a link to its Markdown twin: `/guides/x/#a` → `/guides/x.md#a`, `/` →
 * `/index.md`. Other links are returned as they are.
 *
 * @param {string} href
 * @returns {string}
 */
export function twinHref(href) {
  const match = /^(\/[^#?]*?)\/?(#.*)?$/.exec(href);
  if (!match || href.startsWith("//") || /\.[a-z0-9]+(#.*)?$/i.test(match[1] ?? "")) return href;
  const path = match[1] === "" || match[1] === "/" ? "/index" : match[1];
  return `${path}.md${match[2] ?? ""}`;
}

/**
 * Rewrites every Markdown link target in `markdown` with `rewrite`, outside code.
 *
 * @param {string} markdown
 * @param {(href: string) => string | undefined} rewrite returns `undefined` to keep the text alone
 * @returns {string}
 */
export function rewriteLinks(markdown, rewrite) {
  return outsideCode(markdown, (text) =>
    text.replace(/(!?)\[([^\]]*)\]\(([^)\s]+)\)/g, (whole, bang, label, href) => {
      if (bang) return whole;
      const target = rewrite(href);
      return target === undefined ? label : `[${label}](${target})`;
    }),
  );
}

/**
 * Applies `transform` to `markdown` with its fenced code blocks and inline code hidden: inline
 * code becomes a placeholder while `transform` runs, so a link whose text is code (`` [`x`](/y/) ``)
 * is still seen whole, and a link written inside code is never touched.
 *
 * @param {string} markdown
 * @param {(text: string) => string} transform
 * @returns {string}
 */
function outsideCode(markdown, transform) {
  return outsideFences(markdown, (part) => {
    /** @type {string[]} */
    const spans = [];
    const hidden = part.replace(/`[^`\n]+`/g, (span) => `\uE000${spans.push(span) - 1}\uE001`);
    return transform(hidden).replace(/\uE000(\d+)\uE001/g, (_, n) => spans[Number(n)] ?? "");
  });
}

/**
 * Applies `transform` to the parts of `markdown` outside fenced code blocks.
 *
 * @param {string} markdown
 * @param {(text: string) => string} transform
 * @returns {string}
 */
function outsideFences(markdown, transform) {
  const parts = markdown.split(/(^ {0,3}(`{3,}|~{3,})[^\n]*\n[\s\S]*?\n {0,3}\2[ \t]*$)/m);
  /** @type {string[]} */
  const out = [];
  for (let i = 0; i < parts.length; i++) {
    const part = parts[i] ?? "";
    if (i % 3 === 1) out.push(part);
    else if (i % 3 === 0) out.push(transform(part));
  }
  return out.join("");
}

/**
 * A page's MDX body as Markdown.
 *
 * @param {string} body the page without its front matter
 * @param {{ snippet: (file: string) => string, codeTable: () => string, changelog: () => string, benchmarks?: (section: string) => string }} components
 *   `snippet` reads an example file; `codeTable` writes the code table; `changelog` the releases;
 *   `benchmarks` one section of the benchmarks page
 * @returns {string}
 */
export function mdxToMarkdown(body, components) {
  // The page's own code blocks keep their lines (an example's `import`, a `<Tabs>` in JSX).
  let text = outsideFences(body, (part) =>
    part
      .replace(/^import\s.+?;?\s*$/gm, "")
      .replace(SNIPPET, (_, file) => codeBlock(file, components.snippet(file)))
      .replace(EXAMPLE, (_, attributes) => exampleMarkdown(attributes, components.snippet))
      .replace(TRANSCRIPT, (_, name) => consoleBlock(components.snippet(transcriptFile(name))))
      .replace(DIAGRAM, (_, _quote, name) => diagramMarkdown(name))
      .replace(/<CodeTable\s*\/>/g, () => components.codeTable())
      .replace(/<Changelog\s*\/>/g, () => components.changelog())
      .replace(/<Benchmarks\s+section="([a-z-]+)"\s*\/>/g, (_, section) => {
        if (components.benchmarks === undefined) throw new Error("a page shows benchmarks, but none were read");
        return components.benchmarks(section);
      })
      .replace(/^[ \t]*<Tabs[^>]*>[ \t]*$/gm, "")
      .replace(/^[ \t]*<\/Tabs>[ \t]*$/gm, "")
      .replace(/^[ \t]*<TabItem\s+label="([^"]+)"[^>]*>[ \t]*$/gm, "**$1**\n")
      .replace(/^[ \t]*<\/TabItem>[ \t]*$/gm, ""),
  );
  text = asides(dedentComponents(text));
  text = rewriteLinks(text, (href) => twinHref(href));
  return `${text.replace(/\n{3,}/g, "\n\n").trim()}\n`;
}

/**
 * Removes the indentation MDX allows inside components (lines in tabs are indented by two or
 * four spaces), outside fenced code, which keeps its own.
 *
 * @param {string} text
 * @returns {string}
 */
function dedentComponents(text) {
  /** @type {string | undefined} */
  let fence;
  return text
    .split("\n")
    .map((line) => {
      const marker = /^\s*(`{3,}|~{3,})/.exec(line);
      if (fence !== undefined) {
        if (marker && marker[1]?.[0] === fence[0] && (marker[1]?.length ?? 0) >= fence.length) fence = undefined;
        return line;
      }
      if (marker) {
        fence = marker[1];
        return line.replace(/^ +/, "");
      }
      return line.replace(/^ {2,4}(?=\S)/, "");
    })
    .join("\n");
}

/**
 * Starlight asides (`:::note[Title]` … `:::`) as block quotes: `> **Title:** …`.
 *
 * @param {string} text
 * @returns {string}
 */
function asides(text) {
  return text.replace(/^:::(\w+)(?:\[([^\]]*)\])?[ \t]*\n([\s\S]*?)\n:::[ \t]*$/gm, (_, kind, title, inner) => {
    const label = title || kind.charAt(0).toUpperCase() + kind.slice(1);
    const lines = inner.trim().split("\n");
    return [
      `> **${label}:** ${lines[0] ?? ""}`,
      ...lines.slice(1).map((/** @type {string} */ l) => (l ? `> ${l}` : ">")),
    ].join("\n");
  });
}

/**
 * One Markdown twin: front matter (title, description, version, the page's URL), the title as
 * the H1, the description as a lead paragraph, then the body, with `{{version}}` filled in.
 *
 * @param {{ title: string, description?: string | undefined, version: string, url: string, body: string }} page
 * @returns {string}
 */
export function twin(page) {
  /** @param {string} value */
  const yaml = (value) => JSON.stringify(value);
  const head = [
    "---",
    `title: ${yaml(page.title)}`,
    ...(page.description ? [`description: ${yaml(page.description)}`] : []),
    `version: ${yaml(page.version)}`,
    `url: ${yaml(page.url)}`,
    "---",
    "",
    `# ${page.title}`,
    "",
    ...(page.description ? [page.description, ""] : []),
  ];
  return `${head.join("\n")}\n${withVersion(page.body.trim(), page.version)}\n`;
}
