// The `@laminapdf/ui/testing` subpath: what the gallery (apps/gallery) shares with this package's
// tests, so every state is written once. Workspace-only: the package is private, and this file is
// TypeScript source the consumer compiles.

export { FakeViewer, fakeFile, fakePlatform } from "./fakes.ts";
export { EXAMPLES, type Example } from "./states.tsx";
