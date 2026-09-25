# Local Decision Router and Context Compaction Roadmap

Status: active plan (2026-09-25). The existing `tools/sentrith` CLI remains supported.

The supplied 16-phase roadmap is grouped here into implementation gates. Its Policy phase is moved before model selection so an untrusted model never receives forbidden routes. Its first executor and validation phases are coupled so an agent exit code cannot be mistaken for success. Later multi-provider, evaluation, Assist, fallback, review, Auto, measured routing, VS Code, and optimization milestones remain separate acceptance gates inside Phase 4.

## Phase 0 — Foundation and boundaries (implemented locally)

- Goal: establish the current contract and verify external integration assumptions.
- Deliverables: repository inventory, architecture assessment, specification, ADRs, updated `AGENTS.md`, strict `.sentrith/config.toml`, `config init/check`, `status`, versioned SQLite schema with `db migrate`, and structured route events.
- Acceptance criteria: build/tests pass; config validation rejects unknown or unsafe values; migration is idempotent and rejects newer schema; existing commands remain intact; external uncertainties are recorded without a guessed Jev endpoint.
- Tests: Cargo tests, config validation cases, SQLite reopen/migration cases, and end-to-end CLI smoke in an ignored test directory.
- Risks: bundled SQLite adds a native C build to release targets; filesystem races and permissions need continued attention; LM Studio behavior remains unverified.
- Exit: local Phase 0 criteria pass on Windows. CI platform builds remain to be observed.

## Phase 1 — Policy and local Decision Engine (in progress)

- Goal: make policy-controlled routing observable without executing an agent.
- Deliverables: task features, provider status sources, execution profiles, override checks, `route`, explainable decisions, a swappable Decision Engine, and APUS/LM Studio integration after a real contract probe. Config, rule routing, metadata records, and `explain` are already present.
- Acceptance criteria: `sentrith route "task"` applies hard policy before scoring, never selects a disabled/exhausted/unavailable profile, identifies decision provenance, and persists a decision. Model ranking is enabled only after an APUS/LM Studio contract probe; a rule engine is an explicit provisional fallback.
- Tests: table-driven policy and override cases; mock Decision Engine rejecting out-of-candidate responses; local model contract/evaluation sample and about 100 representative routing cases.
- Risks: availability is not quota; model scores may be uncalibrated; persistence must not leak prompts or secrets.
- Exit: a user can audit the selection and retrieve its metadata, and local model behavior has representative quality evidence. Current implementation is deterministic Suggest routing with metadata records.

## Phase 2 — Limited execution and validation

- Goal: complete one task through a verified subscription CLI adapter.
- Deliverables: `run` in Assist mode for one provider, cancellation, error classification, bounded retries, validation commands, structured events, and task history.
- Acceptance criteria: no API-key or pay-as-you-go fallback; command arguments avoid a shell; workspace and security policy remain enforced; agent completion alone does not mean validation success.
- Tests: mocked adapter failures/quota changes, workspace traversal cases, command argument tests, and one opt-in end-to-end subscription CLI smoke run.
- Risks: provider CLI contracts and approval modes change; local tests cannot prove subscription entitlement.
- Exit: one adapter is exercised end to end with a real task and recorded validation.

## Phase 3 — Recoverable context

- Goal: reduce active context while preserving important original material.
- Deliverables: context classification, immutable keep set, SHA-256 addressed zstd objects, SQLite references, configurable retention, `context` and `gc`.
- Acceptance criteria: current request, policy, plan, active files/diff, unresolved failures/reviews, and relevant decisions stay active; cold objects recover byte-for-byte; conversation is not aggressively discarded.
- Tests: keep-set properties, round-trip/deduplication, retention and crash-safety tests, symlink/path boundary tests.
- Risks: sensitive content retention, object corruption, accidental deletion.
- Exit: restore and garbage collection are demonstrated on representative task records.

## Phase 4 — Controlled automation

- Goal: graduate Assist and Auto only on measured evidence.
- Deliverables: configurable priority/reserve policy, independent review, fallback and escalation, status/history/explain CLI, outcome metrics, and JSONL events.
- Acceptance criteria: max attempts, provider switches, and runtime are enforced; repeated failures reach Human Gate; unknown quota is shown as unknown; Auto remains opt-in until quality and safety gates pass.
- Tests: state-machine/property tests, quota/fallback scenarios, golden routing evaluation, adversarial security cases, and opt-in multi-provider smoke tests.
- Risks: quota estimates can mislead, recursive agents can spend subscription capacity, and review can share failure modes with implementation.
- Exit: documented golden set and operational results justify each mode's default; otherwise Suggest remains default.
