# Engineering Safety Gates

These gates protect the repository from high-impact agent changes made from underspecified requests.

The purpose is not to block work.
The purpose is to require adequate evidence before high-impact actions.

---

## Core principle

For ordinary low-risk changes:

```text
request
→ inspect
→ implement
→ verify
```

For high-impact changes:

```text
request
→ identify high-impact action
→ check evidence
→ proceed only within supported scope
```

If evidence is insufficient, prefer the safest reversible implementation that still advances the task.

Do not invent product requirements or destructive intent.

---

# 1. Hard Gates

The following actions must not be performed merely because they make implementation easier.

They require explicit evidence described in the Evidence Gates section.

## Destructive data changes

Do not, without evidence:

- drop a table/collection
- drop a column/field containing potentially durable data
- delete user/customer data
- truncate production-like data
- rewrite irreversible migrations
- change migration history already expected to be applied
- silently discard unknown enum/state values
- disable backup/recovery behavior

Prefer additive or reversible migrations.

---

## Public compatibility changes

Do not, without evidence:

- remove or rename a public API endpoint
- remove a documented CLI flag
- change request/response semantics incompatibly
- change serialized formats incompatibly
- remove supported configuration keys
- raise minimum runtime/platform versions
- remove supported integrations
- alter externally consumed event/message schemas incompatibly

Prefer backwards-compatible transitions and deprecation paths.

---

## Authentication and authorization

Do not, without evidence:

- weaken authentication requirements
- broaden authorization
- bypass permission checks
- change trust boundaries
- disable CSRF/origin/session protections
- convert authenticated routes to public
- change credential/session semantics incompatibly
- log or persist credentials/tokens insecurely

When fixing auth failures, do not "solve" them by relaxing security.

---

## Secrets and cryptography

Do not, without evidence:

- hard-code secrets
- expose secret values to logs
- commit credentials
- weaken encryption or hashing parameters
- replace cryptographic randomness with non-cryptographic randomness
- disable certificate/TLS verification
- bypass signature validation
- store plaintext secrets where secure storage exists

---

## Verification and quality controls

Do not:

- delete a failing test merely to make the suite pass
- weaken an assertion merely to accommodate a bug
- add broad test skips without explaining scope and reason
- disable lint/type/security checks solely to get green CI
- hide an error with empty catch/exception swallowing
- mark a task complete when required verification was not run

Changing a test expectation is allowed only when there is evidence that intended behavior changed.

---

## Dependency / platform changes

Do not, without evidence:

- replace the primary framework
- replace the database/storage engine
- perform major-version upgrades unrelated to the request
- add a large dependency for trivial functionality
- change deployment/runtime model
- change package manager/build system
- introduce a new external service

Prefer existing project dependencies and patterns.

---

## Scope expansion

Do not expand a narrow request into a broad rewrite unless evidence shows the narrow fix cannot safely satisfy the task.

Examples of prohibited default behavior:

```text
"speed up this query"
→ rewrite persistence layer

"fix login error"
→ replace auth framework

"clean this component"
→ rewrite frontend architecture
```

---

# 2. Evidence Gates

A high-impact action is permitted when at least one strong evidence source supports it.

## Strong evidence

Acceptable sources include:

1. explicit user instruction in the current task
2. accepted feature specification
3. accepted RFC
4. existing ADR / durable project decision
5. executable contract or migration requirement in the repository
6. clearly established current project policy
7. unavoidable technical requirement demonstrated by repository evidence

When evidence sources conflict, surface the conflict.

---

## Weak evidence is not enough

The following alone are insufficient for destructive/high-impact changes:

- "this would be cleaner"
- common industry practice
- model preference
- prior chat memory
- a stale README
- an unrelated old branch
- convenience
- test failures caused by the proposed change
- "the library recommends it" without repository fit

---

# 3. Safe fallback behavior

When a high-impact action seems useful but evidence is insufficient:

## Prefer reversible changes

Examples:

- add before removing
- deprecate before deleting
- feature flag before permanent rollout
- parallel schema field before destructive migration
- compatibility adapter before breaking callers
- explicit validation failure before silent coercion

## Narrow scope

Implement the smallest safe subset that is supported by evidence.

## Preserve uncertainty

Record uncertainty in the task result or active spec rather than inventing the missing decision.

If the user explicitly asked the agent to complete the task without clarification, choose the safest reasonable interpretation that preserves data, compatibility, and security.

---

# 4. Evidence note for Significant work

For Level 2 Significant work, the plan should identify evidence for high-impact changes.

Example:

```md
## High-impact change evidence

- DB column removal: SPEC REQ-004 explicitly requires removal after migration.
- Public API deprecation: ADR-20260819-02 defines the compatibility window.
- Auth scope expansion: not permitted; no supporting requirement exists.
```

Do not create this section when no high-impact actions are involved.

---

# 5. Test integrity gate

Before modifying a failing test, determine which is wrong:

```text
implementation
or
test expectation
```

Evidence for changing a test may include:

- accepted specification
- public contract change
- explicit user requirement
- confirmed obsolete regression test
- intended behavior documented elsewhere and consistent with current design

Without that evidence, fix the implementation instead.

---

# 6. Migration gate

For schema/data migrations, evaluate:

- forward safety
- existing data preservation
- rollback/recovery
- deployment ordering
- old/new application compatibility during rollout
- idempotency where relevant
- production scale/locking risk where relevant

A migration file existing is not proof that it is safe.

---

# 7. Security-sensitive gate

Security-sensitive changes should explicitly preserve or improve:

- authentication
- authorization
- confidentiality
- integrity
- auditability where relevant
- secret handling
- validation boundaries

Do not trade these away silently for convenience or compatibility.

---

# 8. When to stop versus proceed

Do not ask for clarification merely because a decision is interesting.

Proceed automatically when:

- evidence is strong
- the safe interpretation is clear
- the action is reversible and low risk

Do not guess destructive intent.

If no supported safe implementation can satisfy the task, report the exact blocked decision and what evidence is missing.

---

# 9. Relationship to Vibe Coding

The user may give short, underspecified requests.

That is acceptable.

```text
user input:
"fix this"
```

can still trigger a disciplined internal process.

The user should not need to specify:

- task level
- test strategy
- ADR policy
- memory policy
- migration safety checks

Those are repository engineering responsibilities handled by the agent.

Short input is not permission for broad destructive interpretation.


# 10. Independent authorization

For high-impact actions, distinguish documentation from authorization.

A SPEC, ADR, PLAN, or test expectation created by the same agent during the same task may document a proposal, but it is not independent authorization for REVIEW-REQUIRED behavior.

Use `docs/development/HUMAN_REVIEW_POLICY.md`.

High-impact actions without independent authorization must either:

- use a safe reversible fallback, or
- stop at the latest responsible moment for precise human approval.
