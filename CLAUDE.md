# Claude Code Project Instructions

The canonical project memory for this repository is vendor-neutral and lives under `docs/ai/`.

Read and follow `AGENTS.md` as the shared project operating policy.

Before substantial work, consult:

- `docs/ai/PROJECT.md`
- `docs/ai/STATE.md`
- `docs/ai/PROFILE.md`

When relevant, consult:

- `docs/ai/DECISIONS.md`
- `docs/ai/KNOWN_ISSUES.md`
- `docs/ai/TASK_PROTOCOL.md`
- `docs/ai/MEMORY_POLICY.md`
- `docs/ai/TASK_CLOSEOUT.md`

Do not treat Claude Auto Memory, conversation history, or prior session assumptions as more authoritative than the repository and `docs/ai/`.

If Claude-specific memory conflicts with repository evidence, repository evidence wins.

When work changes stable facts, current state, durable decisions, or reusable troubleshooting knowledge, update the appropriate file under `docs/ai/`.

Keep this file thin. Shared project knowledge belongs in `docs/ai/`, not here.


## Development workflows

Use `docs/development/DEVELOPMENT_METHOD.md`.

Project skills are available under `.claude/skills/` and are invoked as:

- `/project-bootstrap`
- `/task-classify`
- `/spec-feature`
- `/implement-feature`
- `/debug-root-cause`
- `/review-change`
- `/task-closeout`
- `/memory-audit`

The `.claude/skills/` files are thin adapters.
Canonical cross-agent workflow content lives in `.agents/skills/`.


## Default usage

For normal coding requests, automatically follow the canonical `dev` workflow even when `/dev` is not explicitly typed.

The user should not need to manually chain task classification, specification, implementation, and closeout.

Explicit shortcut:

```text
/dev <task>
```
