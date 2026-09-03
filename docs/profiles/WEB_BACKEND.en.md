<p align="right"><strong>English</strong> ｜ <a href="WEB_BACKEND.ja.md">日本語</a></p>

# Web / Backend Engineering Profile

This profile covers changes where breaking a **contract, a state transition, or a permission** is observable from outside the system.

## Applies when

Read this profile only for tasks touching:

```text
HTTP/RPC/GraphQL endpoints or schemas
database schema / migrations
authentication, authorization, sessions
external service integrations
background jobs / queues / schedulers
irreversible state transitions (billing, inventory, ...)
```

Do not load it for unrelated work such as copy edits or internal refactors.

## Verification dimensions added

- **Contract verification** — request/response schema, status codes, error shape, backward compatibility
- **State verification** — invariants before/after migration, rollback viability
- **AuthZ verification** — expected response per permission boundary (200 / 401 / 403 / 404)
- **Concurrency verification** — conflicting updates, idempotency, retry safety
- **Boundary verification** — input validation, boundary values, error paths

Unit tests are rarely sufficient here; prefer **contract and integration tests**.

## Technique gates

Techniques are not goals. Apply one only when its condition holds.

| Technique | Apply when | Skip when |
|---|---|---|
| **DDD Lite** (bounded context, shared vocabulary) | Domain language and code have drifted apart, or one concept has several names | CRUD maps cleanly to the domain |
| **DDD Full** (explicit aggregates and invariants) | Invariants span multiple entities and violating them is business-critical | A single table constraint expresses the invariant |
| **Ports & Adapters** | External dependencies must be substitutable in tests | One dependency, no mocking needed |
| **CQRS** | Read and write requirements genuinely diverge in scale or model shape | One model serves both |
| **Event Sourcing** | The history of *why* the state changed is itself a requirement | An audit log suffices |
| **Threat modeling** | Auth, secrets, billing, or PII is involved | Impact stays internal |
| **Property-based testing** | Parsers, serializers, calculations, or conversions with a wide input space | Representative cases give real coverage |

When the call is unclear, **do not apply** the technique; escalate later if evidence appears.

Over-applying techniques is exactly the process overhead Sentrith tries to avoid.

## Definition of correct

```text
the code runs
≠
the contract holds
```

At minimum, state these observably:

- existing clients keep working, or the break is declared as a breaking change
- an unauthorized principal cannot perform a privileged operation
- the migration preserves existing rows
- a failure leaves no partially applied state

## Relationship to Safety Gates

This profile's scope overlaps the high-impact actions in `docs/development/SAFETY_GATES.md`.

Profiles only **add** verification; they never **relax** a Hard Gate.

When they appear to conflict, follow the stricter rule.
