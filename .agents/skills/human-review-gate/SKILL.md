---
name: human-review-gate
description: Classify whether a proposed engineering change needs no human review, recommended review, or required approval, with special protection against an agent self-authorizing high-impact changes from artifacts it created in the same task.
---

Read:

- `docs/development/HUMAN_REVIEW_POLICY.md`
- `docs/development/SAFETY_GATES.md`

Evaluate the proposed change in the current agent turn.

Return one classification:

- REVIEW-NOT-NEEDED
- REVIEW-RECOMMENDED
- REVIEW-REQUIRED

For REVIEW-REQUIRED, identify the exact irreversible/high-impact action and whether independent authorization exists.

A SPEC/ADR/PLAN created by the same agent in the same task is not independent authorization for a high-impact action.

Do not spawn a separate model for this decision.

If review is required, complete safe preparatory work first when useful, then stop before the gated action.
