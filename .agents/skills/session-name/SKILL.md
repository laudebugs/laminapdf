---
name: session-name
description: >
  Names an agent session after its agent and the PRs it holds: `<Name> - [<workItemId>:#<PR>, …]`,
  for example `Yanny - [D2d:#421, E1c:#440]`. Use it when a session opens or merges a PR, hands
  one over, or when the owner asks a session to rename itself.
---

# Session names

Several agent sessions work on Lamina PDF at once. The owner tells them apart by name, so each name says who the agent is and which PRs it holds.

```
<Name> - [<workItemId>:#<PR>, …]
```

- **Name** is the agent's own name (Yanny, Alexi, Gaffer).
- **workItemId** is the todo's ID, the start of its file name in `docs/todos/` (`D2d`, `E1c`).
- **PR** is the number of that todo's open PR.

For example: `Yanny` (no open PR), `Yanny - [D2d:#421]`, `Yanny - [D2d:#421, E1c:#440]`. A PR with no todo, such as CI upkeep, is just its number: `Yanny - [D2d:#421, #449]`. Drop an entry once its PR merges, closes or passes to another session.

An agent can't rename its own session, so whenever the name changes, ask the owner to run `/rename` with the new name, in the message that reports the change:

```
/rename Yanny - [D2d:#421, E1c:#440]
```
