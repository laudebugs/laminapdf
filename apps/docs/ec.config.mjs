// Expressive Code's options for every code block (Starlight's and the `<Code>` component's), in
// their own file because a plugin can't be serialised into the component's settings.
import { defineEcConfig } from "@astrojs/starlight/expressive-code";
import { codeKinds } from "./plugins/code-kinds.mjs";

export default defineEcConfig({
  // Every block is built to wrap with its indentation kept (`wrap` and `preserveIndent`), so a
  // wrapped line continues under its own indentation; paper.css keeps the lines unwrapped until
  // the reader presses "Wrap long lines" (CodeWrap.astro, todo Q4r).
  defaultProps: { wrap: true, preserveIndent: true },
  // Tokens are coloured in the five Paper code kinds (plugins/code-kinds.mjs, todo Q4u), as the
  // "Edit and run" editor colours them, on the editor's surface and text colour, so Shiki's own
  // colouring is off.
  shiki: false,
  plugins: [codeKinds()],
  styleOverrides: { codeBackground: "var(--paper-raised)", codeForeground: "var(--paper-text)" },
});
