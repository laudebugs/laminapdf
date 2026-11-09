# The e2e suite's pinned files

`web-manifest*.schema.json` are SchemaStore's JSON schemas for the web app manifest, fetched on
2026-11-09 from `https://json.schemastore.org/web-manifest-combined.json` and the three files it
refers to (`web-manifest.json`, `web-manifest-app-info.json`, `web-manifest-share-target.json`).
SchemaStore publishes them under the Apache License 2.0. `installability.e2e.test.ts` validates
the reader's manifest against them; refresh them by fetching the same four URLs again.
