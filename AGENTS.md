# AGENTS.md

This repository uses vendor-neutral project memory stored under `docs/ai/`.

## Source of truth

Treat the current repository as authoritative.

Durable AI-facing project knowledge lives in:

- `docs/ai/PROJECT.md` — stable project facts
- `docs/ai/STATE.md` — current working state
- `docs/ai/DECISIONS.md` — durable engineering decisions
- `docs/ai/KNOWN_ISSUES.md` — recurring expensive-to-rediscover problems
- `docs/ai/TASK_PROTOCOL.md` — protocol for substantial tasks
- `docs/ai/MEMORY_POLICY.md` — rules for what should and should not become durable memory
- `docs/ai/TASK_CLOSEOUT.md` — end-of-task memory classification and cleanup gate

Do not treat chat history or model-specific memory as the canonical source of project facts.

## Start of task

Before substantial work:

1. Inspect Git status and the relevant repository area.
2. Read `docs/ai/PROJECT.md`.
3. Read `docs/ai/STATE.md`.
4. Read `docs/ai/DECISIONS.md` when the task may affect architecture, APIs, persistence, dependencies, security, compatibility, deployment, or established design choices.
5. Read `docs/ai/KNOWN_ISSUES.md` when debugging or touching a related subsystem.
6. Read only source files, tests, configuration, and documentation relevant to the task.

Use progressive disclosure. Do not load the whole repository merely because it is available.

Do not read all of `DECISIONS.md` or `KNOWN_ISSUES.md` by default when targeted search can identify relevant entries.

Treat context as a budget. Repository memory should reduce rediscovery, not become another large prompt.

## Authority order

When sources disagree, prefer:

1. current source code and executable configuration
2. tests that describe current behavior
3. current CI/build configuration
4. `docs/ai/`
5. other repository documentation
6. Git history
7. chat history or model-specific memory

If a lower-priority source reveals likely intent that conflicts with current code, flag the conflict rather than silently choosing one.

## Default automatic workflow

For ordinary engineering requests, do not require the user to manually invoke a sequence of workflow skills.

Automatically perform the logic of `.agents/skills/dev/SKILL.md` within the current task:

1. classify Tiny / Normal / Significant internally
2. use the lightest safe development process
3. create specification artifacts only when warranted
4. use test-first/regression-first verification when meaningful
5. implement and verify
6. run the memory gate before finishing

Do not ask the user which workflow level to use unless the process choice itself is the subject of the request.

Do not launch extra model calls or subagents solely for classification, process routing, or memory closeout.

Specialized skills remain available when the user explicitly wants only one stage.

## Development method

Use `docs/development/DEVELOPMENT_METHOD.md` for engineering workflow.

Before substantial implementation, choose the lightest safe task level:

- Level 0 — Tiny: direct change + narrow verification
- Level 1 — Normal: explicit goal + acceptance criteria + test/regression-first when practical
- Level 2 — Significant: `SPEC.md` + `PLAN.md` + `TASKS.md`, verification, and ADR/RFC when warranted

Do not force full SDD onto trivial changes.

Repository-scoped reusable workflows live canonically in `.agents/skills/`.
Use them when the task matches their descriptions.

## Hard Gates and Evidence Gates

For high-impact engineering actions, follow `docs/development/SAFETY_GATES.md`.

Never perform the following merely for convenience:

- destructive data/schema changes
- incompatible public API/contract changes
- authentication/authorization weakening
- secret exposure or cryptographic weakening
- deleting/weakening tests to obtain green results
- disabling CI/lint/type/security checks to bypass failures
- major framework/database/runtime replacement unrelated to the request
- broad rewrites when a narrow safe change is available

Such changes require strong evidence from the current user request, accepted specification/RFC/ADR, executable repository contract, or unavoidable repository evidence.

When evidence is insufficient, choose the safest reversible/narrow interpretation that advances the task.

Short or vague user requests are not permission for destructive scope expansion.

## Human Review Escalation

Use `docs/development/HUMAN_REVIEW_POLICY.md`.

Default to autonomous completion.

Classify material changes as:

- REVIEW-NOT-NEEDED
- REVIEW-RECOMMENDED
- REVIEW-REQUIRED

Do not block for REVIEW-RECOMMENDED.

For REVIEW-REQUIRED actions, do safe preparatory work and stop only before the gated high-impact step if independent authorization is missing.

An artifact created by the same agent in the same task (SPEC/ADR/PLAN/test expectation) cannot by itself authorize destructive, security-weakening, or breaking behavior.

## Meta-document exclusion

`docs/meta/` contains human-facing history and design philosophy about this AI development template.

For ordinary application feature, bugfix, refactor, review, or debugging work:

- do not search `docs/meta/`
- do not load `docs/meta/` into context
- do not use `docs/meta/` as project-behavior evidence

Read it only when changing/evaluating the development template, context/credit strategy, or process design.

## Cost-aware execution

Follow `docs/development/CREDIT_POLICY.md`.

Do not spawn extra model calls for routine workflow stages when the current agent can perform them.

When relevant:

- dependency change → `docs/development/DEPENDENCY_POLICY.md`
- large diff → `docs/development/DIFF_BUDGET.md`
- verification planning → `docs/development/VERIFICATION_POLICY.md`

Do not load these detailed policies when they are irrelevant.

## Engineering behavior

- Make the smallest coherent change that satisfies the task.
- Follow existing repository conventions before inventing new ones.
- Avoid speculative refactors.
- Do not introduce dependencies without concrete benefit.
- Do not weaken validation, security, error handling, or test coverage silently.
- Keep generated files generated.
- Do not claim a command passed unless it was actually run.
- Mark unknowns as unknown; do not invent architecture or rationale.

## Debugging

When debugging:

1. capture the exact symptom
2. reproduce when practical
3. identify the smallest responsible subsystem
4. form a falsifiable hypothesis
5. test the hypothesis
6. fix the root cause with the smallest coherent change
7. add regression coverage when practical
8. preserve reusable findings in `docs/ai/KNOWN_ISSUES.md` when rediscovery would be costly

Avoid random fix sequences without updating the hypothesis.

## Verification

Run the narrowest relevant checks first, then broaden when justified.

Typical order:

1. affected tests
2. affected integration tests
3. type checks
4. lint/static analysis
5. build
6. broader suite

Use the repository's actual commands documented in `docs/ai/PROJECT.md`.

## Project-memory maintenance

Project memory is part of the implementation.

Update `docs/ai/PROJECT.md` when stable project facts change.

Update `docs/ai/STATE.md` when current status, blockers, failing checks, migrations, or next actions materially change.

Update `docs/ai/DECISIONS.md` only for durable decisions whose rationale would otherwise be expensive to rediscover.

Update `docs/ai/KNOWN_ISSUES.md` only for recurring or costly troubleshooting knowledge.

Correct stale entries instead of appending contradictory history.

Git is the chronological history. `docs/ai/` is compressed current knowledge.

## End of task

Before finishing:

1. inspect the final diff
2. verify unintended files were not changed
3. run relevant checks
4. run `docs/ai/TASK_CLOSEOUT.md` and update `docs/ai/` only if the memory gate is satisfied
5. summarize:
   - what changed
   - why
   - verification actually performed
   - remaining risks or unresolved work
