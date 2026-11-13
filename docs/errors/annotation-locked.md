# `annotation-locked`

- Type: error
- Kind: `state` (the operation isn't valid in the current state)

## What happened

The annotation's flags forbid the change (ISO 32000-2 §12.5.3, Table 167). The `Locked` flag forbids deleting the annotation, moving it or changing its properties, though its contents may still change. The `LockedContents` flag forbids changing its contents. A delete also takes the annotation's pop-up, its replies and the rest of its group, so a lock on any of those refuses it too.

## Why

The file's author locked the annotation so that readers can't change it by accident, as some review and form tools do.

## How to fix

An update that changes only the annotation's flags is always allowed, so clearing the lock first lets the next edit through. Clear it only when you are allowed to change the annotation.

## Example

```json
{ "op": "updateAnnot", "args": { "id": "a12.0", "translate": { "dx": 10, "dy": 0 } } }
```

The annotation has `/F 132` (Print and Locked), so it can't be moved.

## Data fields

None. The `location` is the annotation's `object` and `id`.

## For contributors

The decisions and todos behind this code:

- [E4b](../todos/done/E4b-annotation-model-and-overlay.md): the annotation model and the annotation ops' core
