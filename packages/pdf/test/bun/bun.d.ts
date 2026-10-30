// The part of `bun:test` test/bun/examples.test.ts uses, for tsc (Bun doesn't type-check).
declare module "bun:test" {
  export function test(name: string, fn: () => Promise<void>, timeout?: number): void;
}
