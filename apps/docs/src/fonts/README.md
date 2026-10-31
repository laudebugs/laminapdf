# Fonts of the docs site

The [Paper design language](../../../../docs/design/paper/design-language.md#3-typography)'s two UI families, self-hosted. The site's CSP allows fonts from `'self'` only, and the product makes no third-party requests (UI/UX plan principle 7). `src/styles/paper.css` loads them, and Astro bundles them into `/_astro/` with hashed names.

| File | Family | Axes | Range | From | SHA-256 |
| --- | --- | --- | --- | --- | --- |
| `newsreader-latin-standard-normal.woff2` | Newsreader | `opsz` 6–72, `wght` 200–800 | Latin | `@fontsource-variable/newsreader@5.3.0` `files/` | `6e4f2958c3a7c4a80acde4e5a679abe7e01bc1e30b92be3c7a8b696ef401d101` |
| `newsreader-latin-ext-standard-normal.woff2` | Newsreader | same | Latin Extended | same | `45683de03de37187604102316c0b42c0cb2d8dc9c4140a20ad471c3148cc1278` |
| `newsreader-latin-standard-italic.woff2` | Newsreader Italic | same | Latin | same | `5dfcd10d24af8c82927ba57f7983dd997f7b200594c09c35a1b1ed9fc4597506` |
| `source-sans-3-latin-wght-normal.woff2` | Source Sans 3 | `wght` 200–900 | Latin | `@fontsource-variable/source-sans-3@5.3.1` `files/` | `19143dca075972bc84e3fd9eab7416dd40cc565dccb2435a044bdfabe060c7d5` |
| `source-sans-3-latin-ext-wght-normal.woff2` | Source Sans 3 | same | Latin Extended | same | `a85a7459bdb3cdc1136751e151a506bae653fc29ada3ca86237477df6f1b59e6` |
| `source-sans-3-latin-wght-italic.woff2` | Source Sans 3 Italic | same | Latin | same | `5fbc14d56cea35e60596c72ca969b349443c6a536f74e11748eed8b3a396e61d` |

Both families are under the SIL Open Font License 1.1: [`OFL-Newsreader.txt`](OFL-Newsreader.txt) and [`OFL-SourceSans3.txt`](OFL-SourceSans3.txt), each with its copyright notice. The files are committed rather than installed as npm dependencies, like the OFL fonts in [`fixtures/fonts/`](../../../../fixtures/fonts/). They are unmodified, so the OFL's Reserved Font Name terms don't apply.

To update them, `npm pack` the packages above, copy the same files from `files/` and the `LICENSE`, and update this table.
