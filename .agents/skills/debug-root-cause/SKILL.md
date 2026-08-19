---
name: debug-root-cause
description: Debug a failure systematically: reproduce, form a falsifiable hypothesis, identify root cause, add regression evidence when practical, fix minimally, and preserve expensive reusable troubleshooting knowledge.
---

Read:

- `AGENTS.md`
- `docs/ai/PROJECT.md`
- `docs/ai/STATE.md`

Search `docs/ai/KNOWN_ISSUES.md` for matching symptoms before broad investigation.

Workflow:

1. Capture the exact symptom/error.
2. Reproduce when practical.
3. Trace the smallest failing path.
4. Separate symptom from suspected cause.
5. Form one falsifiable hypothesis at a time.
6. Test the hypothesis with the cheapest meaningful check.
7. Add a regression test/check first when practical.
8. Apply the smallest root-cause fix.
9. Verify the failure is gone without regressing related behavior.
10. Run `task-closeout`.

Do not apply a sequence of unrelated speculative fixes.

Record a known issue only if the knowledge is likely to prevent meaningful future rediscovery.
