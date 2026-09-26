# Local Router Implementation Plan

Specification: `SPEC.md`
Last updated: 2026-09-25

## Approach

Extend the existing binary additively. Policy, strict config, and metadata recording now support a Suggest route that does not execute an agent. Model adapter, execution, and context storage follow in separately verified steps. Existing guard and usage paths remain operational.

## Affected areas

| Area | Intended change |
|---|---|
| `tools/sentrith/src/main.rs` | add CLI entry points only |
| `tools/sentrith/src/routing.rs` | task features, policy, profile selection, explanation |
| `tools/sentrith/src/config.rs`, `storage.rs` | strict settings, schema, decision metadata |
| later `tools/sentrith/src/*` | model, provider, context modules as required |
| `.sentrith/` | ignored local config, metadata, objects, logs |
| `docs/ai/*`, `AGENTS.md`, `docs/ROADMAP.md` | durable decisions and working contract |

## Verification

| Criterion | Evidence |
|---|---|
| initial Suggest route | unit tests and CLI smoke run; no agent executed or tracked files changed |
| policy containment | table cases including unavailable CLI, override, and Human Gate |
| model ranking | mock engine validation plus on-device APUS protocol test |
| records | SQLite reopen and schema compatibility tests |
| execution | mocked failure cases and opt-in real CLI smoke run |
| context | recovery, deduplication, retention, and boundary tests |

## High-impact change evidence and rollout

The current request authorizes a new local router but not weakening existing hooks, billing controls, or security boundaries. Additive CLI commands are reversible. New storage is isolated from `.ai-usage/`. Agent execution stays opt-in until provider contract and sandbox behavior are verified. No destructive migration is planned.

## Dependency review

Routing metadata and strict TOML config need maintained parsers rather than bespoke SQL/TOML implementations. `rusqlite` 0.40.2 (MIT) uses bundled SQLite so release binaries need no system SQLite installation; this adds a native C build to each CI target. `toml` 1.1.6 and `serde` 1.0.229 are MIT/Apache-2.0. `Cargo.lock` pins their transitive tree. Existing guard/usage paths continue to use the standard library. The dependency cost is accepted for schema integrity and strict configuration validation; CI builds across the release matrix remain a follow-up verification gate.

Unix executable discovery additionally uses `libc` 0.2.189 (MIT/Apache-2.0), maintained by the Rust project, for `faccessat` with `AT_EACCESS`. Rust's standard library does not expose this effective-ID/ACL check; mode-bit inference is incorrect, and hand-coded FFI constants differ across platforms. The target-specific dependency adds one locked package with no enabled transitive dependencies. Its build script selects compiler/platform cfg values, without adding a native-library build. Sources: [libc](https://github.com/rust-lang/libc) and the downloaded, checksum-locked package manifest/build script.

## Risks

See `docs/architecture/LOCAL_ROUTER.md`. The main Phase 1 gate is verified APUS logprob behavior, not merely successful text generation.
