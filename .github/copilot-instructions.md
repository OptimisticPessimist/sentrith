# GitHub Copilot Repository Instructions

This repository keeps vendor-neutral project memory under `docs/ai/`.

Use `AGENTS.md` as the shared engineering-agent policy.

Before substantial changes, consult:

- `docs/ai/PROJECT.md`
- `docs/ai/STATE.md`
- `docs/ai/PROFILE.md`

Consult when relevant:

- `docs/ai/DECISIONS.md`
- `docs/ai/KNOWN_ISSUES.md`
- `docs/ai/TASK_PROTOCOL.md`
- `docs/ai/MEMORY_POLICY.md`
- `docs/ai/TASK_CLOSEOUT.md`

Treat current source code, tests, configuration, and CI as authoritative over stale documentation.

Do not duplicate project facts in this file. This file is only a GitHub Copilot adapter.

When implementation changes invalidate durable project knowledge, update the corresponding file under `docs/ai/`.


## Development workflows

Use `docs/development/DEVELOPMENT_METHOD.md`.

Canonical reusable agent skills are stored in `.agents/skills/`.

In Copilot CLI, use the matching skill when relevant, for example:

- `Use /task-classify ...`
- `Use /spec-feature ...`
- `Use /implement-feature ...`
- `Use /debug-root-cause ...`
- `Use /review-change ...`
- `Use /task-closeout ...`
- `Use /memory-audit ...`
- `Use /project-bootstrap ...`

In supported IDEs, `.github/prompts/` provides reusable `/sdd-*` prompt-file shortcuts.


## Default usage

For ordinary coding requests, follow `.agents/skills/dev/SKILL.md` as the default end-to-end workflow.

Do not require the user to manually chain classification, specification, implementation, and closeout.

In Copilot CLI the explicit shortcut is `Use /dev ...`.
In supported IDEs, `/dev` is provided by `.github/prompts/dev.prompt.md`.
