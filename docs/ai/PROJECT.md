# Project Knowledge

## Project summary

Sentrith is a vendor-neutral AI development workflow template plus a local Rust CLI. The current CLI provides deterministic repository checks, hook integration, and usage measurement. A local decision router is being added incrementally; Suggest routing records local metadata but does not execute an agent. Primary target is local developer work on Windows, with CI builds on supported platforms.

## Technology and layout

| Area | Current fact |
|---|---|
| CLI | Rust 2021 single binary, `tools/sentrith/src/main.rs` |
| Dependencies | existing guard/usage code uses the standard library; router uses `serde`, `toml`, `rusqlite` with bundled SQLite, and Unix-only `libc` for effective executable access, locked in `Cargo.lock` |
| Build/test | Cargo, inline unit tests in `main.rs` and `routing.rs` |
| CI | `.github/workflows/sentrith-ci.yml` and release workflows |
| Existing usage storage | ignored `.ai-usage/` CSV and task files |
| Router storage | ignored `.sentrith/config.toml` and `.sentrith/sentrith.db` (schema 1); compressed object store not implemented |

`tools/sentrith/src/routing.rs` owns the initial policy and rule-based route slice. `docs/architecture/LOCAL_ROUTER.md` records verified integration facts and proposed boundaries. `docs/ROADMAP.md` records implementation phases. `tools/sentrith/DECISIONS.md` records security-sensitive file-write decisions for existing settings and backup paths.

## Commands

```sh
cargo test --manifest-path tools/sentrith/Cargo.toml
cargo build --manifest-path tools/sentrith/Cargo.toml
cargo run --manifest-path tools/sentrith/Cargo.toml -- route "fix bug"
cargo run --manifest-path tools/sentrith/Cargo.toml -- config check
cargo run --manifest-path tools/sentrith/Cargo.toml -- db migrate
```

`cargo fmt --manifest-path tools/sentrith/Cargo.toml --check` currently reports pre-existing formatting differences in `main.rs`; format only touched modules until that unrelated diff is handled separately. Existing commands include `preflight`, `guard`, `closeout-check`, `review-hint`, `diff-budget`, `hooks`, and `usage`.

## Boundaries and conventions

Preserve existing CLI commands and `.ai-usage/` records. Router policy filters execution profiles before any Decision Engine; provider-specific CLI behavior belongs in adapters. Current `route` is local, rule-based, stores metadata in SQLite, and does not execute an agent. Do not infer subscription quota from CLI presence. No API-key billing fallback is authorized. Treat task text as data, not shell code.

New local `.sentrith/` data is Git-ignored. Existing settings/backup writes use hardened no-follow and permission-preserving helpers; read `tools/sentrith/DECISIONS.md` before adding similar writes.
