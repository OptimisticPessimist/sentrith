# Sentrith v3

Vendor-neutral engineering workflow for:

- OpenAI Codex
- Claude Code
- GitHub Copilot

It combines:

- project memory
- lightweight 3-level Spec-Driven Development
- test-first development where meaningful
- acceptance criteria
- ADR/RFC discipline
- reusable agent skills
- context-cost control

## Architecture

```text
                           repository truth
                                  |
                    +-------------+-------------+
                    |                           |
                docs/ai/              docs/development/
             durable memory             method/rules
                    |                           |
                    +-------------+-------------+
                                  |
                          .agents/skills/
                         canonical workflows
                         /       |        \
                        /        |         \
                    Codex     Copilot      Claude adapter
                      |          |              |
                 $skill     /skill in CLI   /skill-name
                 /skills    /sdd-* in IDE
```

## Task levels

```text
Level 0 Tiny
inspect → change → verify → memory gate

Level 1 Normal
goal → acceptance criteria → test/check → implement → verify → memory gate

Level 2 Significant
SPEC → PLAN → TASKS → tests/checks → implement → verify → ADR if needed → memory gate
```

## Shared skills

Canonical definitions:

`.agents/skills/<skill>/SKILL.md`

Default workflow:

- `dev` — automatically routes the entire engineering task

Specialized workflows:

- `project-bootstrap`
- `task-classify`
- `spec-feature`
- `implement-feature`
- `debug-root-cause`
- `review-change`
- `task-closeout`
- `memory-audit`

## Codex

Codex reads repository skills from `.agents/skills/`.

Explicit invocation:

```text
/skills
```

then choose a skill, or mention it directly:

```text
$task-classify
$spec-feature
$implement-feature
$debug-root-cause
$review-change
$task-closeout
$memory-audit
$project-bootstrap
```

Codex custom slash prompts are intentionally not used by this template because custom prompts are deprecated in favor of skills and are local-user configuration rather than repository-shared configuration.

## Claude Code

Claude Code project skills live under `.claude/skills/`.

This template keeps those files as thin adapters to avoid maintaining two copies of the workflow.

Invoke directly:

```text
/project-bootstrap
/task-classify
/spec-feature
/implement-feature
/debug-root-cause
/review-change
/task-closeout
/memory-audit
```

## GitHub Copilot CLI

Copilot can load the canonical `.agents/skills/` directly.

Explicitly request a skill:

```text
Use /task-classify to classify this request.
Use /spec-feature to specify the new authentication flow.
Use /debug-root-cause to investigate this error.
```

## GitHub Copilot IDE prompt shortcuts

For VS Code, Visual Studio, and JetBrains environments that support Copilot prompt files, `.github/prompts/` adds:

```text
/sdd-classify
/sdd-spec
/sdd-implement
/sdd-debug
/sdd-review
/sdd-closeout
/sdd-memory-audit
/sdd-bootstrap
```

Prompt files are adapters; the canonical workflow remains `.agents/skills/`.

## Existing repository adoption

1. Copy this template into the repository.
2. Run the appropriate bootstrap workflow.
3. Review the first generated `docs/ai/` snapshot.
4. Commit it.
5. Classify future non-trivial work before implementation.

Examples:

Codex:

```text
$project-bootstrap
```

Claude Code:

```text
/project-bootstrap
```

Copilot CLI:

```text
Use /project-bootstrap to initialize this repository.
```

Copilot IDE:

```text
/sdd-bootstrap
```

## New repository

Start with project knowledge mostly empty.

Do not invent architecture up front just to fill the template.

Let verified stable facts accumulate as the project becomes real.

## Key rule

Do not optimize for producing more specifications, tests, ADRs, or memory.

Optimize for:

```text
clear intent
+ executable evidence
+ minimal rediscovery
+ minimal irrelevant context
```


## Sentrith pre-release prototype simplified usage

The user no longer needs to remember the workflow chain.

Normal usage is simply a normal engineering request.

The agent automatically applies the `dev` workflow.

Explicit shortcuts:

```text
Codex:          $dev
Claude Code:    /dev
Copilot CLI:    Use /dev ...
Copilot IDE:    /dev
```

Japanese quick start:

`docs/guide/QUICKSTART.ja.md`

Optional deterministic hooks:

`docs/automation/HOOKS.ja.md`


## Sentrith pre-release prototype additions

- explicit credit/context budget policy
- dependency supply-chain gate
- diff budget
- verification policy
- design philosophy and design-history documentation

The default workflow still avoids extra model calls for routine classification, review, memory handling, and summarization.
