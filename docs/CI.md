# CI and Local Checks

This document describes the continuous-integration pipeline and local commands used to verify the Eisen project.

## Pipeline

The CI pipeline is defined in `.github/workflows/ci.yml` and runs on every push and pull request to `main`.

| Job | Purpose | Blocking |
|---|---|---|
| `structure` | Verify repository boundaries from P0.01 | yes |
| `markdown-lint` | Lint Markdown files | yes |
| `secret-scan` | Scan for committed secrets with TruffleHog | yes |
| `license-check` | Verify `LICENSE` is present | yes |
| `android-lint` | Run Android lint on `clients/android` | yes |
| `android-unit-tests` | Run Android unit tests | yes |
| `dependency-scan` | Run OSV scanner on supported lockfiles | yes (when lockfiles exist) |
| `sbom` | Generate an SPDX SBOM artifact | yes (generation must succeed) |
| `protocol-stubs` | Verify `protocol/`, `tests/`, and `core/` boundaries | yes (will run vector runner in P1) |

## Local commands

Run all local checks:

```bash
./ops/run-local-checks.sh
```

Run checks individually:

```bash
./tools/verify-structure.sh
./tools/verify-protocol-stubs.sh
(cd clients/android && ./gradlew lint)
(cd clients/android && ./gradlew testDebugUnitTest)
```

## Pending additions

The following checks depend on P0.03 stack decisions and P1 core/protocol implementation:

- **Schema compatibility**: add `tools/verify-schema.sh` once canonical schemas are frozen in P0.
- **Protocol vectors**: replace `tools/verify-protocol-stubs.sh` with the P1.18 vector runner.
- **Reference-model tests**: replace the stub with property/convergence tests once `core/` and `tests/` are populated.
- **SBOM/dependency enforcement**: the `dependency-scan` job is currently skipped until lockfiles (e.g. `gradle.lockfile`) are present. Once lockfiles are committed, OSV scanning will run automatically.
