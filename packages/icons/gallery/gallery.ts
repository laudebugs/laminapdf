// The icon gallery: every icon at each size the design language renders, in `text-3` and in
// `accent`, on the light and the dark theme. `pnpm --filter @laminapdf/icons gallery` serves it;
// the browser test checks it with axe, and the Node test that it lists every icon.

import { ICONS, type IconName, type IconSize, iconSvg } from "../src/index.ts";

const SIZES: IconSize[] = [14, 16, 18, 20, 22];
const THEMES = [
  ["light", "Paper light"],
  ["dark", "Paper dark"],
] as const;

function row(name: IconName): string {
  const cells = SIZES.map((size) => `<td>${iconSvg(name, { size })}</td>`).join("");
  const accent = `<td class="accent">${iconSvg(name, { size: 18 })}</td>`;
  return `<tr data-icon="${name}"><th scope="row"><code>${name}</code></th>${cells}${accent}</tr>`;
}

function table(theme: string, title: string): string {
  const head = SIZES.map((s) => `<th scope="col">${s} px</th>`).join("");
  const rows = (Object.keys(ICONS) as IconName[]).map(row).join("\n");
  return `<section data-theme="${theme}" aria-labelledby="t-${theme}">
<h2 id="t-${theme}">${title}</h2>
<table>
<caption>Every icon in <code>text-3</code> at each size, then in <code>accent</code> at 18 px</caption>
<thead><tr><th scope="col">Name</th>${head}<th scope="col">Accent</th></tr></thead>
<tbody>
${rows}
</tbody>
</table>
</section>`;
}

/** The gallery's markup: a heading, the label rule, and one table per theme. */
export function galleryHtml(): string {
  return `<main>
<h1>Paper icons</h1>
<p>Every icon is decorative (<code>aria-hidden</code>) inside a button that has a visible label or an
<code>aria-label</code> naming the action as a verb phrase, such as "Find in document". An icon that stands
alone takes a label of its own and becomes <code>role="img"</code>.</p>
${THEMES.map(([theme, title]) => table(theme, title)).join("\n")}
</main>`;
}

/** The gallery's styles: the tokens' colours, type and spacing. */
export const GALLERY_CSS = `
body { margin: 0; background: var(--paper-ground); color: var(--paper-text); font: var(--paper-type-body); }
main { padding: var(--paper-space-24); }
h1, h2 { font: var(--paper-type-title); }
section { background: var(--paper-surface); color: var(--paper-text); padding: var(--paper-space-16); margin-block: var(--paper-space-24); border: var(--paper-border-width) solid var(--paper-hairline); border-radius: var(--paper-radius-6); }
table { border-collapse: collapse; }
caption { text-align: start; font: var(--paper-type-caption); color: var(--paper-muted); padding-block-end: var(--paper-space-8); }
th, td { padding: var(--paper-space-6) var(--paper-space-12); text-align: center; color: var(--paper-text-3); }
th[scope="row"] { text-align: start; color: var(--paper-text); font: var(--paper-type-control); }
th[scope="col"] { font: var(--paper-type-label); color: var(--paper-text-3); }
td.accent { color: var(--paper-accent); }
`;
