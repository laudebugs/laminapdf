// The part of `bun:test` test/bun/examples.test.ts uses, and the `Bun.write` that
// examples/quick-start/bun.ts uses, for tsc (Bun doesn't type-check).
declare module "bun:test" {
  export function test(name: string, fn: () => Promise<void>, timeout?: number): void;
}
declare const Bun: { write(path: string, data: Uint8Array): Promise<number> };
