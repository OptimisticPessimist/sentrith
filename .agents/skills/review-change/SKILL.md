---
name: review-change
description: Review current changes against repository conventions, acceptance criteria, tests, security, compatibility, and unintended behavior. Use before merge, after agent implementation, or when asked for code review.
---

Read:

- `AGENTS.md`
- `docs/ai/PROJECT.md`
- `docs/ai/PROFILE.md`
- relevant feature spec/acceptance criteria if present

Inspect the actual diff and relevant surrounding code/tests.

When the diff matches a trigger in `PROFILE.md`, check the matched profile's verification dimensions as part of "missing verification".

Prioritize findings:

1. correctness/regressions
2. security/privacy
3. data loss/migration risk
4. API/compatibility breakage
5. concurrency/reliability
6. missing verification
7. maintainability only when material

Do not manufacture stylistic findings to fill a review.

For each material finding include:

- severity
- evidence/location
- why it matters
- smallest reasonable correction

If no material findings exist, say so and state what was actually inspected.
