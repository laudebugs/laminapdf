# Contributing

## Workflow

1. `main` is always the integration branch. Do not push directly to it after the initial commit.
2. Create a branch per change: `<type>/<short-description>` (e.g. `docs/pdf-research`, `feat/xref-parser`, `fix/flate-predictor`).
3. Open a pull request against `main`. Keep PRs focused on one thing.
4. Merge once review is done and checks pass.

## Commits

Use [Conventional Commits](https://www.conventionalcommits.org/) style: `type(scope): summary`.

Types: `feat`, `fix`, `docs`, `refactor`, `test`, `perf`, `build`, `ci`, `chore`.

## Documentation

- Anything worth remembering goes in `docs/` — see [`docs/README.md`](../README.md) for where.
- Significant technical decisions get an ADR in `docs/decisions/`.
