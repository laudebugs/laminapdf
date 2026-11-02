// The package version the worker handshake compares (L2a): the client and the worker it starts
// must be one build. `cargo xtask version sync` writes it from package.json (ADR 0017), and
// test/version.test.ts checks the two agree.

/** `@laminapdf/sdk`'s version. */
export const VERSION = "0.2.0-alpha.1";
