# Current Project State

Last reviewed: 2026-09-26

## Current status

- Windows Rust CLI tests: 110 passed, including failed/concurrent config publication, destructive-request variants, and concurrent first-use routing regressions. Unix-specific symlink, permission, and effective-user executable-access cases run in CI.
- Phase 0 now has validated TOML config (`config init/check`), a versioned SQLite schema (`db migrate`), structured route events, and `status` alongside the assessment documents.
- Phase 1 has a rule-based `route` command with policy containment tests and SQLite decision metadata retrievable by `explain`. APUS scoring, provider execution, context objects, and GC are not implemented.

## Verification gaps and blockers

- The local LM Studio daemon is not running; `lms ls`, `lms server start`, and `lms daemon up` timed out. APUS Q8_0 presence, prompt tokenization, complete label logprobs, and routing quality need on-device verification.
- Codex CLI is present, but authentication and quota status are unknown. Claude CLI is absent. Installed Antigravity CLI exposes editor commands rather than verified headless agent execution.
- Current `route` checks configured enabled profiles and CLI presence; exhausted status is a placeholder for later provider status sources. It never executes an agent. The local metadata database was exercised in an ignored test directory.

## Next useful actions

1. Start LM Studio with the APUS Q8_0 GGUF and verify the official prompt and full candidate distribution. If the required distribution is unavailable, keep an explicit rule engine or verify the bundled llama-server path.
2. Add verified provider status sources, then a Decision Engine adapter and golden routing cases.
3. Gate one subscription CLI adapter and Assist execution on provider contract and sandbox verification.
