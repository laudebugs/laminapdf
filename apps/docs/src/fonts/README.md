# Fonts of the docs site

This folder self-hosts Source Sans 3, the one family the docs site uses for all its text. The site dropped the Paper design language's Newsreader titles for one sans-serif font ([quality plan §5](../../../../docs/plans/quality.md#5-the-font)). The site's CSP allows fonts from `'self'` only, and the product makes no third-party requests (UI/UX plan principle 7). `src/styles/paper.css` loads the files, and Astro bundles them into `/_astro/` with hashed names.

| File | Family | Axes | Range | From | SHA-256 |
| --- | --- | --- | --- | --- | --- |
| `source-sans-3-latin-wght-normal.woff2` | Source Sans 3 | `wght` 200–900 | Latin | `@fontsource-variable/source-sans-3@5.3.1` `files/` | `19143dca075972bc84e3fd9eab7416dd40cc565dccb2435a044bdfabe060c7d5` |
| `source-sans-3-latin-ext-wght-normal.woff2` | Source Sans 3 | same | Latin Extended | same | `a85a7459bdb3cdc1136751e151a506bae653fc29ada3ca86237477df6f1b59e6` |
| `source-sans-3-latin-wght-italic.woff2` | Source Sans 3 Italic | same | Latin | same | `5fbc14d56cea35e60596c72ca969b349443c6a536f74e11748eed8b3a396e61d` |
| `source-sans-3-latin-ext-wght-italic.woff2` | Source Sans 3 Italic | same | Latin Extended | same | `6fe10a66ec416f77a26309b645471d032ad8175063bb605b886e9b7e57efaaa8` |

Source Sans 3 is under the SIL Open Font License 1.1: [`OFL-SourceSans3.txt`](OFL-SourceSans3.txt), with its copyright notice. The files are committed rather than installed as npm dependencies, like the OFL fonts in [`fixtures/fonts/`](../../../../fixtures/fonts/). The files are Google Fonts' Latin and Latin Extended subsets of the upstream font (Modified Versions under the OFL), unchanged since; the licence declares no Reserved Font Name, so the subsets may keep the family name. ADR 0019 allows bundled OFL-1.1 fonts; they go in the notices file with the first release.

To update them, `npm pack` the package above, copy the same files from `files/` and the `LICENSE`, and update this table.

These files are an interim copy for the docs site ([U6g](../../../../docs/todos/done/U6g-docs-site-paper.md)). [U6b](../../../../docs/todos/ready/U6b-self-hosted-fonts.md)'s `@laminapdf/fonts` (fonts subsetted from upstream sources, fallback metrics, a preload) replaces them, and the docs site then imports its Source Sans 3 from that package and deletes this folder.
