<p align="right"><strong>English</strong> ｜ <a href="QUICKSTART.ja.md">日本語</a></p>

# Sentrith Quickstart

Sentrith is designed so normal usage stays simple.

## Install first

For an existing repository, start with the repository contract:

```bash
./scripts/install.sh /path/to/your-project
```

or on Windows:

```powershell
./scripts/install.ps1 -Target C:\\path\\to\\your-project
```

See [Installation](INSTALLATION.en.md) for exactly what is copied.

## The default workflow

In ordinary work, ask your coding agent naturally:

```text
Fix this bug.
Add this feature.
Refactor this module.
Check whether RAG quality regressed.
```

The agent should use the repository's Sentrith rules and decide the appropriate engineering depth in the **same turn**.

You should not need to manually run a chain of classification/review/memory agents.

## If you are unsure, use `dev`

The canonical end-to-end workflow is the `dev` skill/router.

Vendor-facing shortcuts may differ, but the conceptual behavior is the same:

```text
user request
→ task/risk classification
→ Tiny / Normal / Significant
→ appropriate engineering
→ verification
→ memory closeout
```

## Codex

Use the repository's Codex adapter or `$dev` shortcut when available.

Normal natural-language requests should still work without explicitly naming the router.

## Claude Code

Use the repository's `/dev` adapter when you want an explicit end-to-end workflow.

## GitHub Copilot CLI / IDE

Use the provided prompt adapter where supported, or instruct Copilot to follow the repository's Sentrith development contract.

## First-time bootstrap

When adding Sentrith to an existing repository, bootstrap Project Memory once.

At minimum, establish:

```text
docs/ai/PROJECT.md
docs/ai/STATE.md
```

Then populate durable decisions and known issues only when they actually exist.

Do not copy full chat history into Project Memory.

## Development levels

### Tiny

Use for low-risk, local changes.

Typical flow:

```text
understand the narrow scope
→ implement directly
→ run a narrow verification
→ close out
```

Do not create a full SPEC just because Sentrith supports specifications.

### Normal

Use for ordinary features and bug fixes.

Typical flow:

```text
goal / acceptance criteria
→ useful test or check
→ implementation
→ verification
→ memory closeout if needed
```

### Significant

Use when the change affects architecture, compatibility, security, data, external services, or multiple subsystems.

Typical flow:

```text
SPEC
→ PLAN
→ TASKS
→ tests/checks
→ implementation
→ verification
→ decision/memory update
```

Examples that usually justify escalation:

- public API changes
- database migrations
- authentication / authorization
- security-sensitive behavior
- major external-service integration
- architecture changes
- broad cross-subsystem changes

## Specialized techniques

Sentrith may recommend additional techniques only when they fit the project:

- DDD Lite / Full
- Threat Modeling
- Property-Based Testing
- Ports & Adapters
- Statistical Review
- AI evaluation
- Visual / runtime verification
- Formal Methods
- CQRS / Event Sourcing in rare justified cases

The technique is never the goal.

## Automation boundary

Sentrith automates routine, deterministic work.

It does **not** automatically create extra LLM calls just to:

- classify every task
- perform routine review
- decide whether memory changed
- summarize the same work
- re-check ordinary safety classification

Use the current agent turn unless independence has real value.

## Hooks

Hooks are optional.

They are useful for deterministic checks such as:

```bash
sentrith preflight
sentrith guard
sentrith closeout-check
```

Keep them off or minimal if your environment does not need them.

## Hard gates

High-impact actions may require independent evidence.

Examples:

- destructive data/schema operations
- intentional compatibility breaks
- auth/security weakening
- weakening tests to make CI green
- unrelated major framework/runtime replacements
- major scope expansion from a narrow request

The agent cannot create a new document in the same task and use that document alone to authorize the dangerous action.

## Human review

Sentrith uses three levels:

```text
REVIEW-NOT-NEEDED
REVIEW-RECOMMENDED
REVIEW-REQUIRED
```

`REVIEW-REQUIRED` should stop only at the latest responsible moment. Safe preparatory work may continue.

## Credit policy

Optimization is subordinate to correctness and safety.

Sentrith avoids redundant model calls, giant context loads, and repeated repository archaeology, but it does not skip necessary verification to save usage.

## Measure usage

For local comparison:

```bash
sentrith usage report --compare
```

For IDE/Desktop workflows, use the Task Ledger:

```bash
sentrith usage task start ...
# work normally
sentrith usage task stop --success yes
```

For public community benchmarking:

```bash
sentrith usage contribute ...
```

Read the measurement docs before making public reduction claims.

## Recommended first day

1. Run `scripts/install.sh` or `scripts/install.ps1` against the target repository.
2. Ask the agent to read `docs/ai/BOOTSTRAP.md` and perform bootstrap without changing application behavior.
3. Review `PROJECT.md` and `STATE.md` once.
4. Use your agent normally.
5. Record a baseline before claiming any usage reduction.
