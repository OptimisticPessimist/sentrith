---
name: dev
description: Default end-to-end engineering workflow. Use for ordinary coding requests when the user wants a bug fixed, feature added, refactor performed, or implementation completed without manually selecting task-classify/spec/implement/closeout skills. Internally choose the lightest safe process and complete the task end-to-end.
---

This is the default router for engineering work.

Read:

- `AGENTS.md`
- `docs/development/DEVELOPMENT_METHOD.md`
- `docs/ai/PROJECT.md`
- `docs/ai/STATE.md`

Then perform the requested engineering task in one continuous workflow.

## 1. Classify internally

Choose Level 0 Tiny, Level 1 Normal, or Level 2 Significant.

Do not stop merely to report the classification unless the user asked for classification only.

Do not invoke a separate model/subagent only to classify the task.

## 2. Use the lightest safe workflow

### Tiny

Inspect → implement → narrow verification → memory gate.

Do not create specification artifacts.

### Normal

Establish goal and acceptance criteria internally.

Use regression/test-first development when meaningful.

Implement → verify → memory gate.

Create a durable spec only if the work turns out to need one.

### Significant

Create/update:

`docs/specs/<feature>/SPEC.md`
`docs/specs/<feature>/PLAN.md`
`docs/specs/<feature>/TASKS.md`

Then implement against acceptance criteria.

Use an RFC only when genuine architectural alternatives need resolution.

Create/update ADR information only when a durable decision exists.

## 3. Apply safety gates automatically

If the task may involve destructive data changes, public compatibility breaks, auth/security changes, verification weakening, major dependency/platform changes, or broad scope expansion:

- read `docs/development/SAFETY_GATES.md`
- evaluate evidence in the same agent turn
- proceed only within supported scope
- prefer reversible/narrow fallbacks when evidence is insufficient

Do not invoke a separate model solely for this gate.

## 4. Escalate automatically

Escalate to Significant for material:

- public API changes
- schema/database migrations
- authentication/authorization
- security boundaries
- external services
- compatibility policy
- deployment/runtime model
- architecture changes
- multi-subsystem changes

Do not ask the user to select a process level.

## 5. Classify human review automatically

For material or high-impact changes, apply `docs/development/HUMAN_REVIEW_POLICY.md` in the same task.

Classify:

- REVIEW-NOT-NEEDED
- REVIEW-RECOMMENDED
- REVIEW-REQUIRED

Do not interrupt for REVIEW-RECOMMENDED.

For REVIEW-REQUIRED:

- complete safe preparatory work first
- do not execute the irreversible/high-impact step without independent authorization
- same-task SPEC/ADR/PLAN text is not independent authorization
- ask only for the exact approval needed at the latest responsible moment

Do not spawn a reviewer model solely for this classification.

## 6. Verify

Run the narrowest meaningful verification first.

Do not claim checks passed unless actually run.

## 7. Close out automatically

Run the logic in:

- `docs/ai/TASK_CLOSEOUT.md`
- `docs/ai/MEMORY_POLICY.md`

within this same workflow.

Do not launch a second model or subagent merely for closeout.

"No project-memory update required" is valid.

## 8. Keep context lean

Do not read all specs, ADRs, or known issues.

Search for relevant entries first.

The purpose of this workflow is to reduce user interaction and rediscovery, not increase process overhead.


## Credit discipline

Follow `docs/development/CREDIT_POLICY.md`.

Do not spawn additional agents solely for routing, routine review, summarization, safety classification, or memory closeout.

## Dependency changes

If adding/replacing/major-upgrading a dependency, apply `docs/development/DEPENDENCY_POLICY.md` in the same turn.

## Diff budget

Keep changes to the smallest coherent scope.

If the final diff is large, apply `docs/development/DIFF_BUDGET.md`.

## Verification policy

Use `docs/development/VERIFICATION_POLICY.md`.

Executable evidence is stronger than prompt confidence.
