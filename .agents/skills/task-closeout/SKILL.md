---
name: task-closeout
description: Finish substantial engineering work: inspect final diff, run relevant verification, evaluate acceptance criteria, and update project memory only when the memory gate is satisfied.
---

Read:

- `docs/ai/TASK_CLOSEOUT.md`
- `docs/ai/MEMORY_POLICY.md`
- relevant active specification if one exists

Execute the closeout protocol.

Important:

- "No memory update required" is a valid and often preferred result.
- Update existing memory entries instead of duplicating them.
- Remove stale STATE entries when work resolves them.
- Create ADR entries only for durable decisions.
- Create known-issue entries only for expensive reusable troubleshooting knowledge.
- Do not store task transcripts or full logs.

Final report should be concise:

- changed
- verification actually run
- acceptance criteria status when applicable
- memory changes and why
- unresolved risks/manual steps
