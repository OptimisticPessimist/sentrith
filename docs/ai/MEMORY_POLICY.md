# AI Project Memory Policy

> The goal is not to remember everything.
> The goal is to minimize future rediscovery cost while keeping routine context small.

## Core rule

Do not record an event merely because it happened.

Persist information only when at least one of these is true:

1. a future agent is likely to need it again
2. rediscovering it would require meaningful investigation
3. forgetting it could cause a regression, security issue, incompatible change, or repeated failed approach
4. it explains a non-obvious architectural constraint
5. it materially changes the current operational state

Otherwise, leave it to Git history, issue tracking, or the current task transcript.

## Storage classifier

At the end of substantial work, classify reusable information:

### PROJECT.md

Store only stable facts, such as:

- repository layout
- supported platforms
- build/test commands
- architectural boundaries
- configuration mechanisms
- established conventions
- external systems

Do not store:

- today's task
- temporary debugging notes
- one-off command output
- speculative future architecture

### STATE.md

Store only current state, such as:

- unfinished work
- current blockers
- currently failing checks
- pending migrations/manual actions
- the next useful action

Replace stale state rather than appending history.

### PROFILE.md

Store only which engineering profiles are enabled, what triggers them in this
repository, and which techniques are actually in force with the condition that
activated them.

Do not store technique explanations; those live in `docs/profiles/`.

Update it when the repository gains or loses a domain (for example a data
pipeline or model-driven behavior appears), not per task.

### DECISIONS.md

Store only durable decisions where rationale matters.

Use when the answer to "why is it this way?" would otherwise require archaeology.

Examples:

- choosing one database strategy over another
- keeping a compatibility layer
- rejecting a dependency
- defining an API contract
- choosing a security boundary

Do not create ADRs for routine implementation details.

### KNOWN_ISSUES.md

Store only expensive-to-rediscover troubleshooting knowledge.

A good entry normally has:

- recognizable symptoms
- a confirmed or explicitly unknown root cause
- a diagnostic method
- a confirmed fix/workaround
- failed approaches worth avoiding

Do not duplicate every ordinary bug.

## End-of-task memory gate

Before finishing a substantial task, ask:

1. Did stable project knowledge change?
2. Did current project state materially change?
3. Was a durable engineering decision made?
4. Was a recurring or expensive failure understood?
5. Would any new entry save more future context than it costs to keep?

If all answers are no, do not modify `docs/ai/`.

## Context-cost discipline

Treat project memory as a context budget, not an archive.

### Default read set

For a normal substantial task:

- `AGENTS.md`
- `docs/ai/PROJECT.md`
- `docs/ai/STATE.md`
- `docs/ai/PROFILE.md`

Read other memory files only when relevant.

Read a `docs/profiles/` document only when the task matches a trigger recorded in `PROFILE.md`.

### Targeted reads

Prefer searching headings, identifiers, subsystem names, errors, and paths.

Do not read every ADR or every known issue by default.

### Size discipline

Keep high-frequency files concise.

Recommended soft targets:

- `PROJECT.md`: <= 400 lines
- `STATE.md`: <= 120 lines
- `PROFILE.md`: <= 100 lines
- `DECISIONS.md`: no fixed total, but retrieve relevant entries rather than reading all
- `KNOWN_ISSUES.md`: no fixed total, but retrieve relevant entries rather than reading all

These are maintenance targets, not correctness limits.

If a high-frequency file grows beyond its target:

- remove obsolete information
- collapse repeated details
- move low-frequency detail into focused documents
- preserve references to source paths/tests instead of copying source content

## Anti-patterns

Avoid:

- one giant `MEMORY.md`
- copying chat transcripts
- copying large source snippets
- storing full logs
- recording every failed attempt
- duplicating README content without adding agent value
- writing the same fact into multiple adapter files
- keeping resolved transient state forever
- reading all memory files before every task

## Information compression rule

Prefer:

> Fact + reason + location + consequence

over narrative history.

Example:

Bad:

> We spent several hours trying package A, then package B, then changed several files...

Good:

> Use package B for image decoding because package A corrupts alpha on Windows.
> Regression: `tests/image/test_alpha.py`.
> Affected adapter: `src/image/windows.py`.

## Cleanup trigger

Run a memory audit when:

- `PROJECT.md` exceeds its soft target
- `STATE.md` contains resolved work
- entries contradict current repository evidence
- multiple entries describe the same constraint
- agents repeatedly load irrelevant memory
- a major architecture migration completes

Use `MEMORY_AUDIT.md` for the cleanup procedure.


## Meta documentation exclusion

`docs/meta/` is human/template-maintenance reference material.

Do not load or search it during normal application development.
It is not part of routine Project Memory.
