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
- [ ] T011 — CI green on all platforms

T011 is the outstanding gate: no Rust toolchain exists on the author machine, so
`cargo test` has not been executed locally. `sentrith-ci.yml` runs it on
Linux/Windows/macOS when the branch is pushed.
