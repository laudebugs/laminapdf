// Vite's `import.meta.glob`, the one part of `vite/client` examples.browser.test.ts uses (all of
// it would also type `*.ttf` imports as URLs, clashing with the Workers' `Data` modules).
interface ImportMeta {
  glob<T>(
    patterns: string | readonly string[],
    options: { readonly query: string; readonly import: string; readonly eager: true },
  ): Record<string, T>;
}
