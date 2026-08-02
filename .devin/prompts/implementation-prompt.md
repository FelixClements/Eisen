# Implementation Prompt for Eisen PWA Build Tracker

Use this prompt when asking an AI to implement the next open issue from the ordered build tracker.

- Replace `[ISSUE_NUMBER]` with a specific GitHub issue number if you know it.
- Or leave the placeholder in place and the AI will pick the next unblocked, lowest-order open issue from the issue comments.

---

**Implement the next issue from the ordered build tracker**

You are a senior full-stack developer. The repo is `FelixClements/Eisen` and the main spec is `phasing-plan.md`. The human is not a developer, so you do all the coding. Do not ask them to write code, but do ask them one focused question at a time if a decision is unclear.

Task:

1. **Pick the issue**
   - List open issues with `gh issue list --state open --label build --limit 100`.
   - Use the "Implementation order: N/79" comments to find the next unblocked, lowest-order issue.
   - If `[ISSUE_NUMBER]` is given, open that issue directly.
   - Read the issue title and body to understand what needs to be built.

2. **Read the spec and context**
   - Read `phasing-plan.md`.
   - Read the closed wayfinder map `#8` for the original decision context.
   - Read relevant ADRs in `docs/adr/` (especially `001-native-stack.md`, `007-local-at-rest-coverage.md`, `008-fail-closed-counter-policy.md`, and `012-server-stack.md`).
   - Refer to the throwaway prototype branches if useful (`prototype/opfs-storage`, `prototype/pwa-shell`, `prototype/cloudflare-deploy`) — do not merge them, but use them as design reference.
   - Read any code files that are clearly relevant (`core/`, `clients/pwa/`, `cloudflare-deploy/`).

3. **Design before coding**
   - If the issue is small and clear, just implement it.
   - If it touches architecture or could have multiple approaches, briefly explain your plan and wait for the human's "ok" before continuing.

4. **Implement**
   - Follow the existing code style and conventions in the repo.
   - Use the same languages/frameworks already in use: Rust (core + Leptos CSR/WASM), `trunk`, `wrangler`, Cloudflare Workers + Static Assets, browser OPFS, and Web Crypto.
   - Do not introduce new languages, frameworks, or platforms unless the issue explicitly requires it.
   - Write tests if the repo has test infrastructure (`core/tests`, `clients/pwa` can be checked with `cargo check`); if not, add minimal verification steps.
   - Do not edit `phasing-plan.md` or the ADRs unless the issue is specifically a documentation update.

5. **Verify**
   - Run `cargo check --target wasm32-unknown-unknown` in `core/` after any core changes.
   - Run `cargo test` in `core/` if tests are relevant.
   - For `clients/pwa`: `cd clients/pwa && cargo check --target wasm32-unknown-unknown`. If `trunk` is installed, also run `trunk build --release`.
   - For Cloudflare/deployment work: `cd clients/pwa && npx wrangler deploy --dry-run` (or use `cloudflare-deploy/` if the PWA branch is not yet on `main`).
   - For browser-specific behavior, describe the manual verification steps.

6. **Commit**
   - Write a concise commit message in the repo's existing style.
   - Include "Generated with [Devin](https://devin.ai)" if you use Devin.
   - Do **not** push to `origin/main`.

7. **Report back**
   - Summarize what changed.
   - Tell the human the commit hash.
   - Explain how to manually test it.
   - Ask if they want you to push, open a PR, or continue with the next issue in the ordered build tracker.

---

## Quick example

> Implement the next ordered issue in `FelixClements/Eisen`. Read the issue, `phasing-plan.md`, and closed wayfinder map `#8`. Follow the repo's existing conventions, verify with `cargo check --target wasm32-unknown-unknown` and `cargo test`, commit, and report back. Do not push.

Or for a specific issue:

> Implement issue `#21` in `FelixClements/Eisen`. Read the issue, `phasing-plan.md`, and wayfinder map `#8`. Follow the repo's existing conventions, verify with `cd clients/pwa && cargo check --target wasm32-unknown-unknown` and `trunk build --release` if trunk is available, commit, and report back. Do not push.
