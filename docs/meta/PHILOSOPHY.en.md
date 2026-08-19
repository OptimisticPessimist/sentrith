<p align="right"><strong>English</strong> ｜ <a href="PHILOSOPHY.ja.md">日本語</a></p>

# Sentrith — Design Philosophy

This document explains why Sentrith is designed the way it is.

It is not normal task context. Keep it under `docs/meta/` so ordinary engineering agents do not waste context on template history.

# 1. Starting point

AI coding is already fast.

The recurring problem is not lack of generation speed. It is that every new session or agent tends to re-discover:

- what the project is
- why the architecture looks this way
- which errors were already solved
- which constraints must not be violated
- what “done” means in this repository

Sentrith therefore focuses on **durable project-owned engineering context**, not on making one model remember more.

# 2. Project memory, not AI memory

Core principle:

> **Do not make the AI remember your project. Make your project remember itself.**

Model memory, chat history, and vendor-specific state are weak foundations for long-lived software.

Repository-owned knowledge is inspectable, reviewable, versioned, and portable across agents.

# 3. Split memory by purpose

A single giant “memory file” becomes expensive and ambiguous.

Sentrith uses four main memory surfaces.

## PROJECT.md

Stable project facts.

Examples:

- purpose
- major architecture
- supported platforms
- non-negotiable constraints
- important commands
- repository conventions

## STATE.md

Current operational state.

Examples:

- current milestone
- active migration
- known temporary constraint
- current rollout status

STATE should remain short and current.

## DECISIONS.md

Durable rationale.

Use it for decisions that future engineers/agents would otherwise need to re-litigate.

Do not log every minor implementation choice.

## KNOWN_ISSUES.md

Expensive-to-rediscover problems.

A useful entry explains:

```text
symptom
root cause
working fix/workaround
verification
relevant version/context
```

# 4. Context is a budget

More context is not automatically better.

Large context:

- costs more
- hides relevant evidence
- increases contradiction risk
- encourages superficial reading

Sentrith uses progressive disclosure: load the smallest evidence set that can answer the task, then expand when uncertainty requires it.

# 5. Where usage savings should come from

The desired savings are structural:

- stop repeated repository archaeology
- stop re-explaining durable decisions
- avoid loading all project memory for every task
- avoid spawning routine subagents
- use deterministic code for deterministic checks
- preserve known fixes and verification knowledge

Savings should **not** come from skipping required verification.

# 6. Do not make “another agent” the default automation primitive

It is easy to design a workflow like:

```text
classifier agent
→ implementation agent
→ review agent
→ memory agent
→ summary agent
```

That can be useful in high-risk work, but as a default it multiplies cost and coordination failure.

Sentrith therefore performs routine:

- classification
- routing
- safety/review classification
- memory closeout
- summary

inside the current agent turn when independence is not materially valuable.

# 7. Do not reject Vibe Coding

Natural-language requests are a good interface.

The problem is not:

```text
"Fix this bug."
```

The problem is when that lightweight input implies lightweight engineering.

Sentrith keeps the interface simple while moving rigor behind it:

> **Vibe Coding UI + Structured Engineering Backend**

# 8. SDD is adaptive

A full specification for every change becomes bureaucracy.

Sentrith uses three default levels.

## Tiny

Direct change + narrow verification.

## Normal

Goal / acceptance criteria + useful test/check + implementation + verification.

## Significant

```text
SPEC
→ PLAN
→ TASKS
→ tests/checks
→ implementation
→ verification
→ ADR/memory
```

Architecture, security, data migration, public API, external-service, or multi-subsystem changes usually justify escalation.

# 9. TDD is adaptive too

Tests are evidence, not religion.

TDD works well when behavior is:

- deterministic
- executable
- economically testable

Other domains may require:

- visual verification
- runtime validation
- device/platform matrices
- AI evaluation sets
- statistical checks
- manual evidence

Sentrith asks: **what evidence best establishes correctness here?**

# 10. Hard Gates / Evidence Gates

A dangerous operation should not be authorized by optimistic prose.

High-impact examples include:

- destructive data/schema operations
- intentional public compatibility breaks
- auth/security weakening
- secret/crypto weakening
- deleting/weakening checks just to get green
- unrelated major runtime/framework/database replacement
- broad scope expansion from a narrow request

These actions may require independent evidence.

# 11. Reject same-task self-authorization

An agent must not:

1. create a new SPEC/ADR/test in the same task,
2. write that the dangerous action is allowed,
3. use that newly created artifact as the only authorization.

That is circular evidence.

# 12. Human review should be minimal, not absent

Review levels:

```text
REVIEW-NOT-NEEDED
REVIEW-RECOMMENDED
REVIEW-REQUIRED
```

The goal is to preserve developer flow.

When review is required, continue safe preparatory work and stop only before the irreversible/high-impact action—the **latest responsible moment**.

# 13. Prefer executable evidence over prompts

Approximate authority order:

```text
current source / executable config
> tests
> CI / build config
> docs/ai
> other docs
> Git history
> chat / model memory
```

A prompt saying “do not break X” is weaker than an executable invariant that fails when X breaks.

# 14. Repository safety is not whole-system safety

The repository cannot directly govern:

- cloud consoles
- identity providers
- secrets managers
- production data
- billing systems
- third-party SaaS control planes

External safety still requires external controls.

# 15. Do not endlessly add rules

Every new policy has a maintenance and context cost.

Prefer:

- consolidation
- executable checks
- stable invariants
- source-of-truth configuration
- narrow domain profiles

Sentrith should get simpler as patterns become better understood.

# 16. End goal

The goal is not a giant AI-development framework.

The goal is:

> **a project that explains itself well enough that different agents can make safe, verifiable changes without repeatedly rediscovering the same engineering context.**

# 17. Minimize runtime dependencies for local automation

A project standard should not require every contributor to install a large automation stack.

Sentrith therefore favors:

- one small Rust CLI
- prebuilt binaries
- std-only/deterministic utilities where practical
- GitHub Actions for portable CI verification

# 18. Vibe Coding positioning

Vibe Coding is a user-experience description, not Sentrith's permanent technical identity.

Sentrith's durable definition is:

```text
vendor-neutral project memory
+ adaptive engineering workflow
+ guardrails
+ verification
+ measurement
```

If the phrase “Vibe Coding” becomes unfashionable, the engineering value remains.

The user-facing promise stays simple:

> **Ask freely. Build reliably.**
