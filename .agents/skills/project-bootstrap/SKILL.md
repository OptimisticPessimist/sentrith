---
name: project-bootstrap
description: Initialize or repair the vendor-neutral project knowledge for an existing repository without changing application behavior. Use when adopting this template, onboarding an unfamiliar repository, or project memory is missing.
---

Read:

- `AGENTS.md`
- `docs/ai/BOOTSTRAP.md`
- `docs/ai/MEMORY_POLICY.md`

Then execute the bootstrap procedure.

Requirements:

1. Treat the current repository as the primary evidence source.
2. Do not modify application behavior during bootstrap.
3. Populate stable facts in `docs/ai/PROJECT.md`.
4. Replace bootstrap placeholders in `docs/ai/STATE.md`.
5. Add decisions/issues only when repository evidence supports them.
6. Run only safe baseline verification commands.
7. Ask the profile questions in `BOOTSTRAP.md` once and populate `docs/ai/PROFILE.md`.
8. Report remaining unknowns and contradictions explicitly.

Profile rules:

- Propose each answer from repository evidence; ask the user only to confirm or correct.
- Enable every matching profile. Profiles are additive overlays, not exclusive modes.
- Record triggers as real paths/change kinds in this repository.
- Enable a technique only when its gate condition in `docs/profiles/` is met; otherwise record why not.
- Keep `PROFILE.md` an index (<= 100 lines). Do not copy technique explanations into it.
