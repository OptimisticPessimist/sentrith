---
name: spec-feature
description: Create or refine a lightweight feature specification for Significant work, grounded in the current repository. Use for new capabilities, cross-cutting changes, migrations, public APIs, security-sensitive changes, or when the user asks for SDD/specification.
---

Read:

- `docs/development/DEVELOPMENT_METHOD.md`
- `docs/specs/_templates/SPEC.template.md`
- `docs/ai/PROJECT.md`
- `docs/ai/STATE.md`

Then inspect only repository areas relevant to the requested feature.

Create or update:

`docs/specs/<feature>/SPEC.md`

The specification must focus on observable intent:

- problem
- outcome
- requirements
- acceptance criteria
- non-goals
- constraints
- current repository evidence
- open questions only when materially unresolved

Do not invent requirements.

Do not duplicate implementation details into the spec unless they are actual constraints.

If the primary unresolved problem is architectural choice between meaningful alternatives, recommend an RFC before finalizing the spec.

If the specification is sufficiently clear and the user asked to proceed, continue to planning rather than asking unnecessary questions.
