// The schema.org description of Lamina PDF that the website's and the docs' home pages carry as
// JSON-LD: who publishes it (`Organization`), the site itself (`WebSite`) and where the code lives
// (`SoftwareSourceCode`). One set of facts for both sites; the licence, the repository and the
// home page come from the workspace's Cargo.toml, so they can't drift from what the crates
// publish. No rating, review or offer: Lamina PDF has none, and Google treats invented ones as
// spam (discoverability research §6). The JSON-LD is a data block, which the browser never runs,
// so the CSP's inline-script hashes leave it out (`integrations.mjs`).

/** The project's name, as both sites write it. */
const NAME = "Lamina PDF";

/** The docs site: the released docs, the only version search engines index. */
const DOCS = "https://docs.laminapdf.com/";

/** The packages Lamina PDF publishes, by registry page: the same project under other names. */
const REGISTRIES = ["https://www.npmjs.com/package/@laminapdf/sdk", "https://crates.io/crates/laminapdf"];

/** What the code is written in. */
const LANGUAGES = ["Rust", "TypeScript"];

/** Where the published packages run. */
const PLATFORMS = ["Node.js", "Deno", "Bun", "Web browsers", "Cloudflare Workers", "Rust"];

/** What the project does, as the repository describes it. */
const DESCRIPTION =
  "A PDF library for Rust and TypeScript: create, edit and extract text from PDFs, in the browser, Node, Deno, Bun and Workers.";

/** The SPDX licence list's page for each licence identifier Lamina PDF may be offered under. */
const SPDX = "https://spdx.org/licenses/";

/**
 * The value of `key = "…"` in the `[table]` of a TOML file, such as `license` in
 * `[workspace.package]`.
 *
 * @param {string} toml
 * @param {string} table
 * @param {string} key
 * @returns {string}
 */
function tomlString(toml, table, key) {
  // A newline first, so a table on the file's first line is found too.
  const text = `\n${toml}`;
  const start = text.indexOf(`\n[${table}]\n`);
  if (start < 0) throw new Error(`Cargo.toml has no [${table}] table`);
  const body = text.slice(start + table.length + 4);
  const end = body.search(/^\[/m);
  const value = (end < 0 ? body : body.slice(0, end)).match(new RegExp(`^${key} = "([^"]+)"`, "m"))?.[1];
  if (value === undefined) throw new Error(`Cargo.toml's [${table}] has no ${key}`);
  return value;
}

/**
 * The facts both sites describe, read from the workspace's Cargo.toml.
 *
 * @param {string} cargoToml the text of the workspace's Cargo.toml
 * @returns {{ licence: string, licences: string[], repository: string, homepage: string }}
 */
export function projectFacts(cargoToml) {
  const licence = tomlString(cargoToml, "workspace.package", "license");
  // An SPDX expression of alternatives, such as `MIT OR Apache-2.0`: a page for each.
  const licences = licence.split(/\s+OR\s+/).map((id) => `${SPDX}${id.replace(/[()]/g, "")}.html`);
  return {
    licence,
    licences,
    repository: tomlString(cargoToml, "workspace.package", "repository"),
    homepage: tomlString(cargoToml, "workspace.package", "homepage"),
  };
}

/**
 * The JSON-LD for one site's home page: the publisher, the site and the source code, linked by
 * `@id`.
 *
 * @param {{ site: "website" | "docs", cargoToml: string }} options
 * @returns {Record<string, unknown>}
 */
export function structuredData({ site, cargoToml }) {
  const facts = projectFacts(cargoToml);
  const home = new URL("/", facts.homepage).href;
  const organization = `${home}#organization`;
  const siteUrl = site === "website" ? home : DOCS;
  return {
    "@context": "https://schema.org",
    "@graph": [
      {
        "@type": "Organization",
        "@id": organization,
        name: NAME,
        url: home,
        logo: new URL("/icon-512.png", home).href,
        sameAs: [facts.repository, ...REGISTRIES],
      },
      {
        "@type": "WebSite",
        "@id": `${siteUrl}#website`,
        name: site === "website" ? NAME : `${NAME} docs`,
        url: siteUrl,
        inLanguage: "en",
        publisher: { "@id": organization },
      },
      {
        "@type": "SoftwareSourceCode",
        "@id": `${facts.repository}#code`,
        name: NAME,
        description: DESCRIPTION,
        codeRepository: facts.repository,
        url: home,
        license: facts.licences,
        programmingLanguage: LANGUAGES,
        runtimePlatform: PLATFORMS,
        author: { "@id": organization },
        documentation: DOCS,
      },
    ],
  };
}

/** The properties schema.org's types need here, by type. */
const REQUIRED = {
  Organization: ["@id", "name", "url", "logo", "sameAs"],
  WebSite: ["@id", "name", "url", "publisher"],
  SoftwareSourceCode: ["@id", "name", "codeRepository", "license", "programmingLanguage", "author"],
};

/** Properties that claim what Lamina PDF doesn't have, which no node may carry. */
const FORBIDDEN = ["aggregateRating", "review", "offers"];

/**
 * What is wrong with a JSON-LD graph, as messages: a node of a type this module doesn't write, a
 * required property missing, a rating, review or offer anywhere, an address that isn't absolute
 * `https:`, or a reference to an `@id` the graph doesn't define. Shape checks only; nothing is
 * fetched.
 *
 * @param {Record<string, unknown>} data
 * @returns {string[]}
 */
export function structuredDataProblems(data) {
  const problems = [];
  if (data["@context"] !== "https://schema.org") problems.push("@context isn't https://schema.org");
  const graph = Array.isArray(data["@graph"]) ? data["@graph"] : [];
  const ids = new Set(graph.map((node) => node["@id"]));
  for (const node of graph) {
    const type = /** @type {keyof typeof REQUIRED} */ (node["@type"]);
    const required = REQUIRED[type];
    if (required === undefined) {
      problems.push(`unexpected type ${String(type)}`);
      continue;
    }
    for (const key of required) {
      if (node[key] === undefined) problems.push(`${type} has no ${key}`);
    }
  }
  walk(data, (key, value) => {
    if (FORBIDDEN.includes(key)) problems.push(`carries ${key}, which Lamina PDF doesn't have`);
    if (typeof value === "string" && /^[a-z]+:/i.test(value) && key !== "@context" && !value.startsWith("https://")) {
      problems.push(`${key} is ${value}, not an https: address`);
    }
    if (key === "@id" && typeof value === "string" && !ids.has(value)) {
      problems.push(`refers to ${value}, which the graph doesn't define`);
    }
  });
  return problems;
}

/**
 * Calls `visit` with every key and value in `value`, depth first.
 *
 * @param {unknown} value
 * @param {(key: string, value: unknown) => void} visit
 */
function walk(value, visit) {
  if (Array.isArray(value)) {
    for (const item of value) walk(item, visit);
  } else if (value !== null && typeof value === "object") {
    for (const [key, inner] of Object.entries(value)) {
      visit(key, inner);
      walk(inner, visit);
    }
  }
}

/**
 * `data` as the text of a `<script type="application/ld+json">` element: JSON with `<` escaped,
 * so no string in it can close the element.
 *
 * @param {Record<string, unknown>} data
 * @returns {string}
 */
export function jsonLd(data) {
  return JSON.stringify(data).replace(/</g, "\\u003c");
}
