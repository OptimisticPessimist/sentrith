# AI-Native Development Method

This repository uses a lightweight, evidence-grounded development method designed for human + coding-agent collaboration.

The method combines:

- lightweight Spec-Driven Development (SDD)
- test-first / Test-Driven Development (TDD) where it adds value
- acceptance criteria
- Architecture Decision Records (ADR)
- targeted project memory
- repository evidence as the final source of truth

The goal is not maximum documentation.

The goal is to give an agent enough durable intent and verification criteria to implement changes correctly without repeatedly rediscovering the project.

---

## Task classification

Classify work before substantial implementation.

### Level 0 — Tiny

Typical examples:

- typo
- formatting
- small CSS/layout adjustment
- obvious localized bug with low risk
- mechanical rename
- trivial refactor with unchanged behavior

Default process:

```text
inspect
→ change
→ narrow verification
→ memory gate
```

No specification document is required.

Do not create process artifacts merely to satisfy the method.

---

### Level 1 — Normal

Typical examples:

- localized feature
- ordinary bug fix
- behavior change affecting a small subsystem
- several related files
- new validation or error handling
- refactor with meaningful behavioral risk

Default process:

```text
goal
→ acceptance criteria
→ targeted evidence
→ failing/regression test when practical
→ implementation
→ verification
→ memory gate
```

A full specification directory is optional.

At minimum, make the desired observable behavior and acceptance criteria explicit before implementation.

For an issue-driven workflow, the issue itself may be the specification.

---

### Level 2 — Significant

Use when one or more apply:

- new product capability
- public API change
- database/schema migration
- authentication/authorization change
- security boundary change
- new external service
- cross-cutting architecture change
- compatibility policy change
- deployment/runtime model change
- multi-subsystem work
- work whose intent is likely to be disputed later

Default process:

```text
SPEC
→ PLAN
→ TASKS
→ tests / executable checks
→ implementation
→ verification
→ ADR if durable decision exists
→ memory gate
```

Create:

```text
docs/specs/<feature>/
├── SPEC.md
├── PLAN.md
└── TASKS.md
```

Use `docs/specs/_templates/`.

If the task requires choosing among substantial architectural alternatives before the feature specification can be stable, create an RFC under `docs/rfcs/` first.

---

## Escalation rule

Start with the lightest process that is safe.

Escalate when investigation reveals greater scope or risk.

Examples:

- Tiny bug touches a public API → Level 2
- Normal feature requires a schema migration → Level 2
- Significant task turns out to be a one-line config correction → the existing spec may remain, but do not invent additional process work

Task level is a risk/context decision, not a measure of lines changed.

---

## SDD rule

Specifications describe observable intent, constraints, and acceptance criteria.

Specifications must not become a duplicate implementation.

Prefer:

```text
what must be true
why it matters
constraints
acceptance criteria
non-goals
```

Avoid prescribing low-level code structure unless that structure is itself a requirement.

The current repository remains evidence of what exists.
The specification describes what should exist after the change.

When they conflict, do not silently reinterpret the specification.
Surface the conflict.

---

## TDD / test-first rule

Use test-first development when behavior can be expressed reliably in automated tests.

Especially prefer it for:

- bug fixes
- parsers/serializers
- business logic
- validation
- API behavior
- state transitions
- calculations
- regressions

For a bug:

```text
reproduce
→ regression test fails
→ minimal root-cause fix
→ regression test passes
```

Do not force unit-test TDD where the real acceptance criterion is not representable by a meaningful unit test.

Examples where other verification may be better:

- visual layout
- graphics/game-engine behavior
- deployment configuration
- hardware integration
- exploratory prototypes

In those cases, define an executable or observable verification procedure instead.

Never write a test that merely encodes the current implementation to satisfy a process requirement.

---

## Acceptance criteria rule

Acceptance criteria should be externally observable when practical.

Good:

- expired session returns 401
- valid session retains current behavior
- migration preserves existing rows
- command exits non-zero on invalid input

Weak:

- create a helper method
- use three classes
- refactor the service

Implementation structure belongs in the plan only when needed.

---

## ADR rule

Record a decision in `docs/ai/DECISIONS.md` when future maintainers are likely to ask:

> Why is the system intentionally this way?

Typical ADR-worthy decisions:

- architecture
- public API policy
- database strategy
- dependency selection/rejection
- security boundary
- compatibility rule
- concurrency model
- deployment model
- non-obvious workaround intentionally retained

Do not record routine implementation details.

The feature spec says **what** must be achieved.
The ADR explains **why a durable design choice was made**.

---

## RFC rule

Use an RFC before implementation when the primary uncertainty is architectural rather than feature-level.

An RFC should compare real alternatives.

Do not write an RFC when the decision has effectively already been made and no meaningful tradeoff exists.

After a decision:

- the RFC may remain as historical design analysis
- the durable conclusion should be compressed into `docs/ai/DECISIONS.md`

---

## Repository evidence rule

Agents must ground implementation in the current repository.

Prefer:

1. current source/configuration
2. executable tests
3. build/CI configuration
4. canonical `docs/ai/` knowledge
5. feature specifications
6. other documentation
7. Git history
8. chat/model memory

This priority does not mean a current bug is "correct" because it exists in code.

It means claims about the existing system must be verified against repository evidence.

Specifications and acceptance criteria define intended changes.

---

## Context-efficiency rule

Do not load every development artifact on every task.

### Default high-frequency context

- `AGENTS.md`
- `docs/ai/PROJECT.md`
- `docs/ai/STATE.md`

### Load when relevant

- current feature `SPEC.md`
- current `PLAN.md`
- current `TASKS.md`
- matching ADR entries
- matching known issues
- relevant tests/source/configuration

### Avoid by default

- unrelated specs
- all historical RFCs
- all ADRs
- all known issues
- old completed task lists
- full Git history

Search first, then read the smallest relevant section.

---

## Engineering Safety Gates

Read `docs/development/SAFETY_GATES.md` when a task may involve high-impact actions.

High-impact actions include:

- destructive data/schema changes
- public compatibility breaks
- authentication/authorization/security changes
- secret/cryptography changes
- disabling or weakening verification
- major dependency/framework/platform changes
- broad scope expansion beyond the request

These actions require supporting evidence.

When evidence is insufficient, prefer a reversible/narrow fallback rather than inventing destructive intent.

## Definition of done

A task is done only when:

- requested behavior is implemented
- relevant acceptance criteria are satisfied
- relevant verification has actually run, or inability to run it is explicitly reported
- unintended changes are absent
- required migrations/manual steps are documented
- durable project knowledge is updated only when the memory gate says it should be

"Code was generated" is not a definition of done.
