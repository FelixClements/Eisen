# Domain Docs

How agent skills should consume this repo's domain documentation.

## Before exploring, read these

- **`CONTEXT.md`** at the repo root
- **`docs/adr/013-web-one-password-e2ee.md`** for auth, crypto, sync, and merge decisions

If either file is missing, proceed silently. Do not suggest creating them upfront.

## File structure

```
/
├── CONTEXT.md
├── docs/adr/
│   └── 013-web-one-password-e2ee.md
└── src/
```

## Use the glossary's vocabulary

When your output names a domain concept, use the term as defined in `CONTEXT.md`. Do not drift to synonyms the glossary explicitly avoids.

## Flag ADR conflicts

If your output contradicts ADR-013, say so explicitly rather than silently overriding.
