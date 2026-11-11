---
name: session-name
description: >
  Names an agent session after its agent and the work it holds: `<Name> - [<workItemId>:#<PR>, …]`,
  for example `Yanny - [D2d:#421, E1c:#440]`. Use it when a session starts work on a todo, opens or
  merges a PR, or when the owner asks a session to rename itself or say what it is working on.
---

# Session names

Several agent sessions work on Lamina PDF at once. The owner tells them apart by their names, so each name says who the agent is and which work items and PRs it holds right now.

## The format

```
<Name> - [<workItemId>:#<PR>, <workItemId>:#<PR>, …]
```

- **Name** is the agent's own name (Yanny, Alexi, Gaffer). It never changes during a session.
- **workItemId** is the todo's ID, the start of its file name in `docs/todos/` (`D2d`, `E1c`, `Q4f`), or a bug's ID from `docs/bugs/`.
- **PR** is the number of the open PR for that work item.

List only the work the session holds now:

| The session holds | Name |
| --- | --- |
| Nothing yet | `Yanny` |
| A claimed todo, PR not open yet | `Yanny - [D2d]` |
| One open PR | `Yanny - [D2d:#421]` |
| Two open PRs | `Yanny - [D2d:#421, E1c:#440]` |
| A PR with no todo (CI upkeep, a typo) | `Yanny - [D2d:#421, #449]` |

Drop an entry once its PR merges or closes, or once another session takes the work over.

## Renaming

An agent can't rename its own session: the name lives in the session's transcript, which agents must not edit. So when the name should change, work out the new one and ask the owner to run:

```
/rename Yanny - [D2d:#421, E1c:#440]
```

Say the new name in the same message that reports the change, such as the message announcing a new PR.

## PR titles carry the work-item ID

Every PR title names its work item right after the Conventional Commit prefix ([git workflow §5](../../../docs/guidelines/git-workflow.md#5-pull-requests)):

```
feat(desktop): D2d - stream engine tiles to the viewer and open a document
```

A PR with no todo or bug, such as CI upkeep or a typo fix, leaves the ID out: `ci: retry apt when a mirror times out`.
