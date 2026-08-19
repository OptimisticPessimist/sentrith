# AI Project Memory Standard

Vendor-neutral repository memory for coding agents.

Designed for:

- OpenAI Codex
- Claude Code
- GitHub Copilot
- other agents that can read repository Markdown

## Design principle

The AI's own memory is not the project's memory.

The repository is the source of truth, and durable agent-facing knowledge is versioned in `docs/ai/`.

```text
Codex ─────── AGENTS.md ───────────┐
                                   │
Claude Code ─ CLAUDE.md ───────────┼──> docs/ai/
                                   │
Copilot ───── .github/              │
              copilot-instructions ┘
                         │
                         ├── PROJECT.md
                         ├── STATE.md
                         ├── DECISIONS.md
                         ├── KNOWN_ISSUES.md
                         ├── TASK_PROTOCOL.md
                         ├── MEMORY_POLICY.md
                         ├── TASK_CLOSEOUT.md
                         └── MEMORY_AUDIT.md
```

## What each layer does

### Adapter files

- `AGENTS.md`
- `CLAUDE.md`
- `.github/copilot-instructions.md`

These stay small.

They tell each agent where canonical project knowledge lives and how to behave.

Do not duplicate large amounts of project knowledge into all three files.

### `PROJECT.md`

Stable project facts:

- technology stack
- repository layout
- build/test commands
- architecture
- external systems
- repository conventions

### `STATE.md`

Current operational state:

- unfinished work
- blockers
- known failing checks
- pending migrations
- next useful actions

This is rewritten as reality changes.

It is not a work diary.

### `DECISIONS.md`

Durable engineering decisions:

- architecture choices
- API design
- dependency choices
- persistence strategy
- security boundaries
- compatibility policy
- deployment model

Record rationale and rejected alternatives when known.

### `KNOWN_ISSUES.md`

Reusable troubleshooting knowledge:

- exact symptoms
- root cause
- diagnosis
- confirmed fix/workaround
- ineffective fixes worth avoiding

### `TASK_PROTOCOL.md`

A common workflow for substantial engineering tasks.

## New repository

Copy the template into the repository and populate `PROJECT.md` as the architecture becomes real.

## Existing repository

Copy the template into the repository, then use `docs/ai/BOOTSTRAP.md`.

Review the first generated knowledge snapshot before committing it.

## Recommended commit

After bootstrap and human review:

```sh
git add AGENTS.md CLAUDE.md .github/copilot-instructions.md docs/ai README-AI-MEMORY.md
git commit -m "chore: add vendor-neutral AI project memory"
```

## Maintenance rule

Do not keep appending history forever.

Prefer:

```text
current truth
+ durable rationale
+ reusable troubleshooting knowledge
```

over:

```text
every task
+ every conversation
+ every failed attempt
+ every old state
```

Git already stores history.

The purpose of `docs/ai/` is to compress the repository into the minimum high-value context future agents need.


## Automatic knowledge capture

At the end of substantial work, agents should run `TASK_CLOSEOUT.md`.

This does not mean "always write memory."

The closeout protocol first runs a memory gate:

```text
task completed
     |
     v
stable fact changed? -------- yes --> PROJECT.md
     |
durable decision? ----------- yes --> DECISIONS.md
     |
recurring costly issue? ----- yes --> KNOWN_ISSUES.md
     |
current state changed? ------ yes --> STATE.md
     |
     no
     v
no project-memory write
```

This prevents project memory from becoming a second task history.

## Context / usage optimization

The template intentionally separates:

- high-frequency compact context: `PROJECT.md`, `STATE.md`
- low-frequency targeted knowledge: `DECISIONS.md`, `KNOWN_ISSUES.md`

Agents should not load low-frequency knowledge wholesale.

The goal is:

```text
less rediscovery
+ fewer repeated explanations
+ targeted file reads
- irrelevant historical context
= lower average context cost
```

This is an optimization strategy, not a guarantee that every task consumes fewer tokens or credits.


## v3 development layer

See `README-AI-DEVELOPMENT.md` and `docs/development/DEVELOPMENT_METHOD.md`.

v3 adds:

- Tiny / Normal / Significant task classification
- lightweight SDD
- test-first/regression-first guidance
- feature spec templates
- RFC template
- cross-agent skills
- Claude Code adapters
- GitHub Copilot IDE slash prompt files
