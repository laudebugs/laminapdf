// The `@laminapdf/ui/testing` subpath: what the gallery (apps/gallery) shares with this package's
// tests, so every state is written once. Workspace-only: the package is private, and this file is
// TypeScript source the consumer compiles. It also lends the reader's e2e suite the properties form's
// rules, which that suite holds to the SDK's (apps/web/e2e/properties-warnings.e2e.test.ts): this package's
// own tests can't load the SDK, which they run before the WebAssembly is built.

export {
  type EditableField,
  fieldWarnings,
  formFrom,
  type MetadataField,
} from "../src/reader/properties-form.ts";
export { FakeViewer, fakeFile, fakePlatform } from "./fakes.ts";
export { EXAMPLES, type Example } from "./states.tsx";
