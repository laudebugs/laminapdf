# lamina-build data

Files the crate embeds with `include_bytes!`.

## `sRGB-v4.icc`

The sRGB profile that PDF/A documents embed in their output intent (`src/profile/pdfa.rs`). It is 480 bytes: an ICC v4.2 display-class profile with the sRGB primaries, white point and tone curve, small enough to sit in every build, the WebAssembly modules included.

- Source: Clinton Ingram's [Compact ICC Profiles](https://github.com/saucecontrol/Compact-ICC-Profiles), `profiles/sRGB-v4.icc` at commit `bdd84663061bc4ae95ca70decff54f581e27f702` (2023-06-07).
- Licence: [CC0-1.0](https://github.com/saucecontrol/Compact-ICC-Profiles/blob/bdd84663061bc4ae95ca70decff54f581e27f702/license), a public-domain dedication. It asks for no attribution; this file records the source.
- SHA-256: `c56e1685d888f5edb92fe07f2750f387f8fe8e91b32ff8fb0b56bfbbb9458353`.
- Checked by `profile::pdfa`'s unit test (a valid RGB profile of 480 bytes) and by veraPDF (`cargo xtask verapdf`), which accepts it as a PDF/A-2 and PDF/A-3 output intent.

Replace it only with a profile under a licence at least as permissive as CC0-1.0, and update this file and the test.
