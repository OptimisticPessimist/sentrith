# Task Closeout Protocol

Use this at the end of substantial engineering tasks.

## A. Verify implementation

- inspect the final diff
- ensure unintended files did not change
- run the narrowest relevant checks
- broaden verification when justified
- record only checks actually executed

## B. Run the memory gate

Evaluate the completed task against `MEMORY_POLICY.md`.

### Stable fact changed?

If yes, update `PROJECT.md`.

### Current state changed materially?

If yes, update `STATE.md`.

Remove stale state while editing.

### Durable decision made?

If yes, update `DECISIONS.md`.

Only record decisions whose rationale will matter later.

### Expensive recurring problem learned?

If yes, update `KNOWN_ISSUES.md`.

Record symptoms, diagnosis, root cause, and confirmed fix.

### Nothing worth preserving?

Do not modify project memory.

"No memory update required" is a valid outcome.

## C. Prevent memory inflation

Before adding text:

1. search for an existing entry
2. update it instead of duplicating it
3. remove contradicted/stale statements
4. reference source paths/tests instead of copying large content
5. keep the new entry to the minimum information needed for rediscovery

## D. Final report

Report:

- implementation changes
- verification actually performed
- project-memory files changed, if any
- why each memory update passed the memory gate
- remaining risks or unresolved work

Do not claim that project memory was updated when it was not.
