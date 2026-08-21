# Usage Measurement v2 Specification

Status: accepted
Owner: OptimisticPessimist
Last updated: 2026-08-22

## Problem

The v1 automatic capture path cannot compute the headline metric "usage per successful task":

- Claude Code rows record cost via statusLine snapshot deltas, which race the statusLine refresh and miss token counts.
- `success` and `rework_count` are always empty for hook-captured rows, so success rate degenerates to 0%.
- Granularity is one row per turn; a multi-turn task skews per-task averages.
- Baseline labeling requires editing hook configs.

## Outcome

Fully automatic, deterministic (no model calls, no network) capture that yields:

- accurate per-turn tokens and model from the provider transcript
- an objective success signal per task (commit reached + last test run green)
- task-level aggregation at report time
- rework computed retroactively from git churn
- baseline labeling via one command, with a marker hooks can always read

## Users / actors

- Claude Code / Codex hooks invoking `sentrith usage hook <agent>`
- Humans running `sentrith usage report`

## Requirements

### Functional

- REQ-001: `usage hook claude` parses the turn's window of the transcript JSONL and records summed input/cache_read/cache_creation/output tokens and model, deduplicated by message id.
- REQ-002: The turn window is delimited by the transcript line count captured at `UserPromptSubmit`.
- REQ-003: Test-like Bash commands inside the window are detected; the matching `tool_result`'s `is_error` flag determines pass/fail. The latest verification state is carried per session.
- REQ-004: HEAD is captured at `UserPromptSubmit` and compared at `Stop`; a change marks the turn as the commit-closing row, and `head_sha` records the new SHA only in that case, so a non-empty value always denotes a task boundary.
- REQ-005: Success semantics: commit reached AND last verification pass → `yes`; commit reached AND last verification fail → `no`; otherwise `unknown`.
- REQ-006: The usage CSV gains `head_sha` and `verification` columns (schema v2). Existing v1 files are migrated in place deterministically (header rewrite + column padding).
- REQ-007: Phase precedence is `--phase` > the `.ai-usage/phase` marker written by `usage baseline start` > `SENTRITH_PHASE` > `standard`. The marker outranks the environment because hooks are spawned by the agent process and never see a variable exported afterwards.
- REQ-008: `usage report --tasks` groups turn rows into tasks by session, closing a task on any recorded `head_sha` (or a decided outcome, for manual ledger rows), and reports task-level stats; success rate counts `yes/(yes+no)` and reports the unknown share separately.
- REQ-009: `usage report --churn [--days N]` computes file-level churn for recorded commits from git history.
- REQ-010: `usage hook codex` applies the same HEAD tracking and best-effort test detection over the Codex transcript (`command` fields plus `exit_code` when present).
- REQ-011: statusLine cost capture remains as fallback for cost_usd; transcript parse failure must not error the hook.

### Non-functional

- No new runtime dependencies; line-oriented parsing consistent with the existing zero-dependency style.
- Hooks must stay fast (single file scan per Stop) and must never block or corrupt the session on malformed input.
- Transcript formats are not stable vendor contracts; parsers must degrade gracefully and record the degradation in `notes`.

## Acceptance criteria

- [x] AC-001: Given a fixture transcript window with duplicated message ids, token sums count each message id once (unit test).
- [x] AC-002: A fixture window containing a failing test command yields `verification=fail`; a passing one yields `pass` (unit test).
- [x] AC-003: A v1 CSV is migrated to v2 with padded columns and all v1 data preserved (unit test).
- [x] AC-004: Phase resolution honors `--phase` > marker > `SENTRITH_PHASE` > default (unit test).
- [x] AC-005: Task grouping closes a task on any recorded `head_sha`, and success rate excludes `unknown` from the denominator (unit test).
- [x] AC-006: `cargo test` passes in CI on Linux/Windows/macOS.

## Non-goals

- Real-time credit metering or provider billing reconciliation.
- LLM-based success judgment.
- Interactive Copilot capture (unchanged v1 behavior).
- Line-level churn attribution (file-level proxy only).

## Constraints

- Claude Code Bash `toolUseResult` has no exit code; failure is only observable via `is_error` on the tool_result block.
- One transcript line exists per content block; assistant usage repeats per line for the same message id.

## Existing behavior and evidence

- `tools/sentrith/src/main.rs`: `usage_hook_claude`, `usage_hook_codex`, `usage_claude_status`, `USAGE_HEADER`, report/publish/contribute paths.
- `.claude/settings.hooks.example.json`, `.codex/hooks.example.json`.

## Open questions

None blocking; Codex transcript field names are best-effort and may need adjustment against real rollout files.

## Related decisions

- Success is defined by objective repository evidence (commit + verification), not human judgment; documented in `docs/metrics/MEASUREMENT_ARCHITECTURE.*`.
- PostToolUse hooks are intentionally not required: pull-based transcript scanning at Stop keeps hook configs vendor-portable.
