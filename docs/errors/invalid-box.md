# `invalid-box`

- Type: warning
- Kind: `input` (the input is malformed beyond what tolerance repairs)

## What happened

A page boundary box is not four numbers; it reads as absent (ADR 0027 §3 for the fallback).

## Why

Writer bugs.

## How to fix

Nothing; the page uses the next box in the chain.

## Example

To be added ([L1b](../todos/ready/L1b-error-contract-codegen.md)).

## Data fields

`key` (which box, from lamina's fixed list).
