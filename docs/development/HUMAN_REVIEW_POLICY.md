# Human Review Policy

The default goal is autonomous engineering with selective escalation.

Human review should be requested only when the expected cost of a wrong autonomous decision is materially higher than the interruption cost.

---

## Review levels

### REVIEW-NOT-NEEDED

Use when:

- change is local and reversible
- behavior is well specified by repository evidence
- no security/data/public compatibility boundary changes
- verification is strong
- rollback is trivial

Examples:

- typo
- small UI adjustment
- localized bug with regression test
- internal refactor with preserved tests/contracts

The agent may proceed and finish autonomously.

---

### REVIEW-RECOMMENDED

Use when:

- moderate behavioral change
- new feature with clear acceptance criteria
- non-destructive schema addition
- external integration with established contract
- significant refactor with good test coverage
- performance change with measurable benchmark

The agent may proceed autonomously unless repository policy says otherwise.

Before finalizing, surface:

- what changed
- key assumptions
- verification
- rollback/migration notes when relevant

Do not block solely because review is recommended.

---

### REVIEW-REQUIRED

Use when any of the following apply and the change is not already explicitly authorized by independent evidence:

- irreversible or destructive data migration
- production data deletion
- permission broadening
- authentication weakening
- secret/cryptographic weakening
- public breaking API/CLI/schema change
- disabling a security/quality control
- legal/compliance-sensitive behavior change
- payment/billing semantics with material financial impact
- production infrastructure action with material outage risk
- irreversible external side effect
- replacing a primary framework/database/runtime
- unresolved conflict between accepted requirements and current repository evidence

When review is required:

1. do all safe preparatory work
2. stop before the irreversible/high-impact step
3. present the exact decision needing approval
4. state the evidence and risk
5. offer the safest default/reversible option

Do not ask for review earlier than necessary.

---

# Independent evidence rule

An agent must not bootstrap its own authorization.

## Not independent

The following is **not sufficient by itself**:

```text
agent writes SPEC saying "drop column"
→ agent cites that same newly-written SPEC
→ agent drops column
```

Likewise:

- an ADR created by the same agent in the same task
- a plan text invented solely to justify the desired implementation
- a modified test expectation written solely to match the new behavior

These may document the proposal, but they do not independently authorize high-impact behavior.

## Independent evidence examples

Strong independent evidence includes:

- explicit user instruction in the current request
- pre-existing accepted SPEC/RFC/ADR
- pre-existing public contract
- existing migration/deprecation policy
- external requirement supplied by the user
- existing repository policy
- unavoidable technical constraint demonstrated by current repository evidence

## Same-task artifacts

A same-task SPEC/PLAN/ADR may be used to:

- clarify scope
- organize implementation
- record rationale
- express acceptance criteria

But for REVIEW-REQUIRED actions, it cannot be the only authorization source.

---

# Authorization inheritance

If the user explicitly requests a high-impact outcome, do not repeatedly ask for permission for each implementation detail that is a necessary, foreseeable part of that outcome.

Example:

```text
user:
"Remove legacy v1 API even though it is breaking."

→ breaking removal is authorized
→ routine supporting edits do not require repeated approval
```

But do not expand authorization beyond the stated scope.

Example:

```text
authorized:
remove v1 API

not automatically authorized:
delete unrelated customer data
replace auth system
```

---

# Safe preparation before review

When REVIEW-REQUIRED applies, the agent should still complete safe work where useful:

- inspect/reproduce
- write tests
- prepare additive migration
- prepare compatibility adapter
- update docs/spec proposal
- calculate impact
- identify callers
- create rollback plan
- stage code that does not execute the irreversible action

The goal is to ask the human one precise question at the latest responsible moment.

---

# Review payload

When escalation is necessary, report only:

## Decision

What exact action needs approval?

## Why

Why is it required?

## Evidence

What independent evidence exists and what is missing?

## Risk

What could go wrong?

## Safe default

What reversible/non-destructive option can be used instead?

Avoid dumping the entire reasoning transcript.

---

# Credit discipline

Human-review classification must be performed inside the current agent turn.

Do not spawn a separate reviewer model merely to decide REVIEW-NOT-NEEDED / RECOMMENDED / REQUIRED.

Deterministic local scripts may provide hints, but model judgment remains in the current task.
