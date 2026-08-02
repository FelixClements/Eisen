---
labels: wayfinder:task
status: open
---

## Question

Which `phasing-plan.md` and ADR changes are required to reflect the Leptos PWA + Cloudflare stack, and when should they be made?

## Context

- ADR-001 currently records Windows (C#/WinUI 3) + Android (Kotlin/Jetpack Compose).
- `phasing-plan.md` P2 is titled "Local-only native product" and lists Windows and Android tasks.
- ADR-007 and ADR-008 need browser-storage notes.
- Updates should happen after the build toolchain, core WASM, and storage adapter decisions are firm.

## Deliverable

A checklist of exact edits to `phasing-plan.md`, `docs/adr/001-native-stack.md`, `docs/adr/007-local-at-rest-coverage.md`, and `docs/adr/008-fail-closed-counter-policy.md`.
