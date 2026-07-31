# Domain Docs

How the engineering skills should consume this repo's domain documentation.

## Before exploring, read these

- `CONTEXT.md` at the repo root.
- Relevant ADRs under `docs/adr/`.

If these files do not exist, proceed silently. The domain-modeling skill creates them lazily when terminology or decisions are resolved.

## File structure

This repo uses a single-context layout:

```text
/
|-- CONTEXT.md
|-- docs/adr/
`-- packages/
```

## Use the glossary's vocabulary

Use terms as defined in `CONTEXT.md`. Avoid synonyms that the glossary explicitly rejects. If a needed concept is absent, reconsider the terminology or note the gap for domain modeling.

## Flag ADR conflicts

If proposed work contradicts an existing ADR, surface the conflict explicitly instead of silently overriding the decision.
