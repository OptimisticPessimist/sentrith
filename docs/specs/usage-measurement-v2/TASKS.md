# Usage Measurement v2 Tasks

Specification: `SPEC.md`
Plan: `PLAN.md`

- [x] T001 — Schema v2 constants + in-place v1→v2 migration + tests
- [x] T002 — Phase resolution helper (`--phase` > `SENTRITH_PHASE` > default) + tests
- [x] T003 — Claude transcript window parser (usage sums, model, msg-id dedupe) + tests
- [x] T004 — Verification detection (test-command patterns + `is_error` matching) + tests
- [x] T005 — HEAD tracking in claude hook; success finalization on commit-closing rows
- [x] T006 — Codex hook: HEAD tracking + best-effort command/exit_code scan
- [x] T007 — `usage report --tasks` grouping + success-rate denominator change + tests
- [x] T008 — `usage report --churn` (file-level, from `git log --numstat`) + parsing tests
- [x] T009 — Update hook example configs (drop `--phase standard`)
- [x] T010 — Update metrics/automation docs (ja/en)
- [x] T011 — CI green on all platforms

All tasks complete. A Rust toolchain was installed locally, so `cargo test` and
`cargo build --release` now run before every push; CI confirms them on
Linux/Windows/macOS.

Follow-up work from code review is recorded in the PR, not here.
