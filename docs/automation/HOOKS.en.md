<p align="right"><strong>English</strong> ｜ <a href="HOOKS.ja.md">日本語</a></p>

# Hooks

Hooks let Sentrith run deterministic checks around agent workflows without spawning additional model calls.

## Design rule

Use hooks for things code can decide reliably.

Good hook use:

- preflight validation
- guard checks
- diff-budget warnings
- task closeout checks
- local usage snapshots
- deterministic repository validation

Bad hook use:

- launching a second LLM for routine classification
- asking another model to summarize every task
- auto-approving destructive changes
- parsing unstable private UI formats when a documented provider surface exists

## Preflight

Typical preflight checks:

```bash
sentrith preflight
```

The goal is to detect obvious repository-state problems early without turning every task into a ceremony.

## Closeout

A closeout hook can verify that the agent has not forgotten required verification or durable memory updates.

A valid outcome can also be:

```text
No memory update required
```

Not every task should modify Project Memory.

## Safety / review hints

Hooks may run:

```bash
sentrith guard
sentrith review-hint
sentrith diff-budget
```

These commands provide deterministic evidence or warnings. They do not replace engineering judgment.

## Claude Code usage capture

Sentrith can combine Claude Code hooks with its documented status information.

Typical flow:

```text
UserPromptSubmit
→ start snapshot

Stop
→ end snapshot
→ record estimated usage delta
```

The provider's billing system remains the source of truth for actual charges.

## Codex usage capture

For machine-readable runs, prefer documented JSON usage surfaces.

Interactive hooks may use documented session/transcript locations as a best-effort fallback where applicable, but unstable transcript formats must not be treated as permanent contracts.

## Hook failure policy

A non-critical measurement hook should not corrupt or block unrelated engineering work.

A safety-critical hook may block only when the policy says independent evidence is required.

## Installing hooks

`sentrith hooks install` merges only Sentrith's own entries into the agent's settings file, idempotently:

```bash
sentrith hooks install                 # claude + codex
sentrith hooks install --agent claude
sentrith hooks install --dry-run
sentrith hooks status
```

Your other settings, other tools' hooks, and a custom statusLine are preserved. On Windows the commands are rewritten to `bin\sentrith.exe`. The previous file is kept as `*.json.sentrith-bak`.

Manual merging from the example files still works.

## Portability

Hook examples are adapters. The canonical project rules live in repository documents and `.agents/skills`, not in one vendor's hook syntax.

### Windows note

On native Windows, hook commands may be executed via `cmd.exe`, where `./bin/sentrith` does not resolve.

Use one of:

```text
bin\sentrith.exe preflight
```

or an absolute path. The binary name is `sentrith.exe` (see `bin/README.md`).

Under WSL or Git Bash, `./bin/sentrith` works as-is.
