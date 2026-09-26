# Local router: evidence, proposed boundaries, and risks

Status: Phase 0 assessment, 2026-09-25. Proposed components below are not all implemented.

## Repository inventory and current architecture

`tools/sentrith` is a single Rust 2021 binary at version 0.2.0. `src/main.rs` contains deterministic preflight/guard/hook checks and usage capture/reporting. The initial router uses `routing.rs`, strict TOML settings use `config.rs`, and SQLite metadata uses `storage.rs`. CI runs Cargo tests and release builds. Usage records remain under ignored `.ai-usage/`; router config and metadata are under ignored `.sentrith/`. Shell and PowerShell installation scripts and prebuilt binaries are supported. There is no model caller, context object store, or agent orchestration path yet. `docs/automation/SENTRITH_CLI.en.md` describes the pre-router CLI as non-agent; the new Suggest command still does not execute one.

Existing settings/backup writes have hardened no-follow and permission-preserving helpers documented in `tools/sentrith/DECISIONS.md`. New `.sentrith/` persistence must receive equivalent security review. The initial worktree had untracked `.claude/settings.json` and `.codex/hooks.json`; they were left untouched.

## External feasibility

| Surface | Evidence | Status |
|---|---|---|
| Rust | Cargo/rustc 1.98 installed; 91 baseline tests and 104 current tests pass | verified locally |
| Codex | `codex-cli 0.121.0`; `codex exec` supports noninteractive prompts, `--cd`, and sandbox options | CLI verified, entitlement/quota unknown |
| Claude Code | no `claude` command on PATH | unavailable locally |
| Antigravity | 1.107.0 CLI help exposes editor/window commands, no headless agent run | agent adapter unavailable locally |
| LM Studio | `lms` installed, daemon stopped; `lms ls`, server start, and daemon start timed out | inference/model presence unverified locally |
| APUS-OpenJev-v1-4B-GGUF | official model card lists Q8_0 and a 2–16 candidate A–P decision protocol, with exact distribution via bundled llama-server client | published contract verified; LM Studio behavior unverified |

Sources: [APUS GGUF model card](https://huggingface.co/apus-ailab/APUS-OpenJev-v1-4B-GGUF), [APUS prompt contract](https://huggingface.co/apus-ailab/APUS-OpenJev-v1-4B-GGUF/blob/main/openjev_contracts.py), [LM Studio REST API](https://lmstudio.ai/docs/developer/rest), [LM Studio compatibility API](https://lmstudio.ai/docs/developer/openai-compat), [LM Studio structured output](https://lmstudio.ai/docs/developer/openai-compat/structured-output).

APUS is a label-scoring decision model, not a general chat assistant. The GGUF card says low-depth effort is unavailable and probabilities are not calibrated. The bundled contract uses a strict prompt and next-label distribution. LM Studio documents `/v1/chat/completions` and schema-constrained text, but that alone does not prove the required complete label-logprob distribution or matching tokenizer boundary. Do not implement `/v1/systemone` or claim exact APUS scores through LM Studio without a real Q8_0 probe. A rule-based Decision Engine may serve as an explicitly identified fallback; it must not manufacture probabilities.

## Proposed boundaries

```text
CLI / config
  -> task features
  -> deterministic policy + provider status sources
  -> allowed ExecutionProfile IDs
  -> DecisionEngine (APUS adapter after probe, rule fallback)
  -> explanation + metadata record
  -> ProviderAdapter execution (later, Assist only)
  -> validation / review / bounded fallback (later)

Context classifier -> immutable keep set -> active context / compressed object store
                                       -> SQLite metadata and retention index
```

Policy owns provider bans, explicit overrides within hard limits, billing mode, CLI availability, exhausted status, concurrency, security, and reserve. Decision Engine receives only allowed profile IDs and cannot invent a route. Provider adapters own CLI syntax, output/error parsing, and cancellation. Unknown quota is a first-class status. Task text is data, never a shell command. The current `route` uses config, local CLI discovery, and rule-based selection; it stores metadata and structured route events in SQLite without storing the prompt or executing an agent. Quota, concurrency, and reserve enforcement await verified status sources.

## Risk register

| Risk | Effect | Mitigation / gate |
|---|---|---|
| Model API mismatch | wrong route or fabricated certainty | official prompt contract plus on-device logprob/token tests before APUS adapter |
| Billing escape | unintended charges | no API key fallback; adapters limited to verified subscription/local modes |
| Partial provider availability | failed or misrouted jobs | separate CLI presence, authentication, quota, and concurrency states; fail closed on execution |
| Prompt injection or destructive instructions | unwanted commands | data/command boundary, workspace sandbox, hard policy and Human Gate; no auto-approve |
| Context loss or leakage | unrecoverable work or exposed secrets | immutable keep set, masked logs, content hashes, local object store, restore tests |
| Persistence migration | damage to existing usage data | separate `.sentrith/` store and additive schema; never rewrite `.ai-usage/` |
| Quality drift | poor choices despite valid syntax | small golden routing set, outcome metrics, explicit model/version provenance |
| Scope collision with existing CLI | regressions | additive commands and baseline test suite |
