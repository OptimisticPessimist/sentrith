---
name: memory-audit
description: Audit and compact docs/ai project memory to remove stale, duplicate, contradictory, or low-value context without changing application behavior. Use when memory grows noisy or after major migrations.
---

Read:

- `docs/ai/MEMORY_AUDIT.md`
- `docs/ai/MEMORY_POLICY.md`

Compare `docs/ai/` against current repository evidence.

Do not change application behavior.

Perform:

- stale fact removal/correction
- resolved STATE cleanup
- duplicate consolidation
- superseded decision marking
- obsolete known-issue removal
- copied log/source compression
- adapter duplication checks

Keep high-frequency context compact.

Report material corrections/removals and any ambiguity requiring human confirmation.
