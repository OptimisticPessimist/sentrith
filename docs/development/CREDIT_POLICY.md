# AI Credit / Context Budget Policy

This repository optimizes for engineering correctness per unit of model usage.

Cost reduction must not override correctness or safety.

---

## Default rule

Do not create additional model calls solely for:

- task classification
- workflow routing
- memory classification
- routine summarization
- routine code review
- safety-gate classification
- human-review classification

Perform these inside the current agent turn.

---

## Subagents / additional model calls

Use additional agents only when the expected value exceeds the cost.

Examples where they may be justified:

- genuinely independent security review
- large architectural comparison
- parallel research across independent subsystems
- user explicitly requests multi-agent review
- failure cost materially exceeds model cost

Do not use subagents merely because the platform supports them.

---

## Hooks

Default hooks must be deterministic local commands.

Allowed by default:

```text
Python
shell
git diff checks
file-size checks
static pattern checks
local linters/tests
```

Not enabled by default:

```text
LLM hook
Agent hook
automatic review model
automatic summarization model
automatic memory-writing model
```

---

## Context loading

High-frequency:

- `AGENTS.md`
- `docs/ai/PROJECT.md`
- `docs/ai/STATE.md`

Load other policy/knowledge only when relevant.

Search before reading large files.

---

## Output discipline

Do not generate long process narration unless requested.

Routine final report should contain only:

- changed
- verification
- unresolved risks/manual steps
- memory update when relevant

Reasoning transcripts are not project artifacts.

---

## Verification exception

Do not skip meaningful verification to save model credit.

Local test/lint/build execution usually costs machine time, not model credit, and often reduces later Agent investigation.

---

## Safety exception

Do not omit necessary context for:

- destructive migration
- security/auth
- payment/billing
- public compatibility
- production infrastructure

A few thousand extra tokens are cheaper than a high-impact failure.

---

## Optimization target

Do not minimize tokens per task blindly.

Optimize:

```text
total engineering cost
=
model usage
+
human review
+
rediscovery
+
rework
+
failure risk
```


## Measurement

Do not assume this policy reduces cost in every repository.

When practical, measure with:

- `sentrith usage record`
- `sentrith usage report`

Prefer credits per successful task over raw token minimization.
