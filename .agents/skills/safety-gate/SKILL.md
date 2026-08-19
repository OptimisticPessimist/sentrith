---
name: safety-gate
description: Evaluate proposed high-impact engineering changes against repository Hard Gates and Evidence Gates. Use for destructive migrations, public API breaks, auth/security changes, major dependency/platform changes, test weakening, or when a short user request could be interpreted broadly.
---

Read:

- `docs/development/SAFETY_GATES.md`
- `docs/ai/PROJECT.md`
- `docs/ai/DECISIONS.md` only as targeted/relevant
- active feature specification/RFC when relevant

For each proposed high-impact action:

1. identify the action
2. identify supporting evidence
3. determine whether the evidence is strong enough
4. if yes, proceed within that scope
5. if no, choose a reversible/narrow fallback when one can still satisfy the task
6. never infer destructive intent from convenience

Do not turn this into a generic security review.

Do not invoke another model solely to evaluate the gate.

Keep the result short unless the user explicitly asks for gate analysis.
