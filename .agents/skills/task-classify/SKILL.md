---
name: task-classify
description: Classify an engineering request as Tiny, Normal, or Significant and select the lightest safe development workflow. Use before substantial implementation when scope or process level is unclear.
---

Read `docs/development/DEVELOPMENT_METHOD.md`.

Inspect enough repository evidence to understand likely scope and risk.

Classify the task:

- Level 0 — Tiny
- Level 1 — Normal
- Level 2 — Significant

Do not classify by lines of code alone.

Escalate for public API, schema/migration, authentication/security, compatibility, deployment, external-service, architecture, or multi-subsystem changes.

Return a concise classification containing:

- level
- reason
- minimum required artifacts
- verification expectation
- whether an ADR/RFC is potentially warranted

If implementation is also requested, proceed using that workflow rather than stopping after classification.
