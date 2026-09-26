# Local Router Specification

Status: draft; Phase 0/1 initial slice in progress
Last updated: 2026-09-25

## Problem and outcome

The current Sentrith CLI checks repository work and measures usage, but cannot safely select or run a coding agent. A user should eventually be able to submit a task once and receive a policy-compliant, explainable route, then optionally execute it. This specification covers the staged design; current code only begins Suggest routing.

## Requirements

- REQ-001: Suggest is the default mode and never executes an agent.
- REQ-002: deterministic policy filters profiles before any model ranking. Security, billing, provider disablement, exhaustion, concurrency, reserve, and unavailable CLI are hard constraints.
- REQ-003: explicit human agent/effort override takes priority among allowed profiles but cannot bypass hard security or billing policy.
- REQ-004: Decision Engine ranks only allowed ExecutionProfile IDs. Rule-based output is labelled as such; model confidence is never fabricated.
- REQ-005: route explanations include the selected profile, alternatives, policy exclusions, provenance, and unknown availability signals.
- REQ-006: local APUS integration uses the published GGUF prompt/score contract only after runtime verification. Model failure has a bounded, visible fallback.
- REQ-007: future Assist execution uses a provider adapter without shell interpolation or automatic API-key billing fallback and requires validation evidence for success.
- REQ-008: future context compaction keeps current request, policy, AGENTS.md, plan, active files/diff, unresolved errors/tests/reviews, and relevant decisions active and stores recoverable original content locally.
- REQ-009: storage records routing metadata in SQLite; later outcome metadata and large original content use deduplicated compressed objects with configurable retention.
- REQ-010: automatic retry has limits for attempts, provider switches, and total runtime and escalates repeat failures.

## Acceptance criteria

- [x] The initial `route` command does not execute an agent or change tracked files, reports a policy-controlled provisional route, and retains existing CLI commands.
- [ ] The final Phase 1 `route` command uses config and provider status sources, invokes a verified local Decision Engine or a declared rule fallback, and records the decision in SQLite. Config and recording are implemented; model and status sources remain.
- [ ] A forbidden, exhausted, unavailable, or security-blocked profile is never selected, even on override or malicious model output.
- [ ] `explain <task-id>` retrieves recorded features, exclusions, scores when available, and final decision without private chain-of-thought. Rule route metadata is implemented; model scores remain.
- [ ] `run` processes a task through at least one verified subscription CLI adapter and records external validation results.
- [ ] Retained context restores byte-for-byte and GC respects the keep set and retention policy.

## Non-goals for the initial slice

Auto mode, VS Code extension, all-provider orchestration, public hosted API, and global quota percentage estimation.

## Constraints and existing evidence

Preserve `tools/sentrith` commands and the existing `.ai-usage/` format. The existing guard/usage paths use the standard library; new TOML/SQLite dependencies passed `docs/development/DEPENDENCY_POLICY.md` as recorded in `PLAN.md`. Any future zstd dependency needs its own review. Current behavior and integration findings are in `docs/architecture/LOCAL_ROUTER.md`.

## Open questions

- Does the installed LM Studio version expose every APUS candidate label logprob with a verified prompt/token boundary?
- Which subscription CLIs can report authentication, rate limits, and cancellation reliably on this machine?
- What are the golden routing cases and acceptable regression thresholds for the user's task mix?
