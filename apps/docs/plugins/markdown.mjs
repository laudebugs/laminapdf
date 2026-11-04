// The site's pages as plain Markdown, for agents (todo L5a): every page's MDX source with its
// components replaced by what they show. `<Snippet file>` becomes a fenced block holding the
// example file, so a twin needs nothing else to be read; `<Tabs>` become a bold label per tab;
// Starlight asides (`:::note[Title]`) become block quotes; links to the site's pages point at
// their Markdown twins. Pure functions over strings, tested by markdown.test.mjs.

/** The fence language of a file the docs show, by extension. */
const LANGUAGES = { ts: "ts", rs: "rust", toml: "toml", json: "json", md: "md" };

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
  const parts = markdown.split(/(^ {0,3}(`{3,}|~{3,})[^\n]*\n[\s\S]*?\n {0,3}\2[ \t]*$)/m);
  /** @type {string[]} */
  const out = [];
  for (let i = 0; i < parts.length; i++) {
    const part = parts[i] ?? "";
    if (i % 3 === 1) out.push(part);
    else if (i % 3 === 0) {
      /** @type {string[]} */
      const spans = [];
      const hidden = part.replace(/`[^`\n]+`/g, (span) => `\u0000${spans.push(span) - 1}\u0000`);
      out.push(transform(hidden).replace(/\u0000(\d+)\u0000/g, (_, n) => spans[Number(n)] ?? ""));
    }
  }
  return out.join("");
}

/**
 * A page's MDX body as Markdown.
 *
 * @param {string} body the page without its front matter
 * @param {{ snippet: (file: string) => string, codeTable: () => string }} components
 *   `snippet` reads an example file; `codeTable` writes the code table
 * @returns {string}
 */
export function mdxToMarkdown(body, components) {
  let text = body
    .replace(/^import\s.+?;?\s*$/gm, "")
    .replace(/<Snippet\s+file="([^"]+)"[^>]*\/>/g, (_, file) => codeBlock(file, components.snippet(file)))
    .replace(/<CodeTable\s*\/>/g, () => components.codeTable())
    .replace(/^[ \t]*<Tabs[^>]*>[ \t]*$/gm, "")
    .replace(/^[ \t]*<\/Tabs>[ \t]*$/gm, "")
    .replace(/^[ \t]*<TabItem\s+label="([^"]+)"[^>]*>[ \t]*$/gm, "**$1**\n")
    .replace(/^[ \t]*<\/TabItem>[ \t]*$/gm, "");
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
 * the H1, the description as a lead paragraph, then the body.
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
  return `${head.join("\n")}\n${page.body.trim()}\n`;
}
