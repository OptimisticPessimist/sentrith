---
name: implement-feature
description: Implement a Normal or Significant engineering change using acceptance criteria, targeted repository evidence, test-first/regression-first verification when meaningful, and final memory closeout.
---

Read:

- `AGENTS.md`
- `docs/development/DEVELOPMENT_METHOD.md`
- `docs/ai/PROJECT.md`
- `docs/ai/STATE.md`
- `docs/ai/PROFILE.md`

Read a `docs/profiles/` document only when the change matches a trigger recorded in `PROFILE.md`.

If a current feature spec exists, also read its `SPEC.md`, `PLAN.md`, and active `TASKS.md` as needed.

Workflow:

1. Establish the requested observable outcome.
2. Confirm the smallest responsible subsystem.
3. Inspect existing tests and conventions.
4. For bugs and testable behavior, create or identify a failing/regression check first when practical.
5. Make the smallest coherent implementation.
6. Run narrow verification first.
7. Broaden verification only when justified by risk.
8. Check acceptance criteria.
9. Inspect the final diff.
10. Run the `task-closeout` workflow.

Do not force unit-test TDD for behavior that is better verified visually, operationally, or through integration checks.

Do not claim checks passed unless actually run.
