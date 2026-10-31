// The two Deno APIs test/deno/examples.test.ts uses, for tsc (Deno checks the file with its own
// types and never loads this one).
declare namespace Deno {
  function test(name: string, fn: () => Promise<void>): void;
  function readFile(path: URL): Promise<Uint8Array<ArrayBuffer>>;
}
