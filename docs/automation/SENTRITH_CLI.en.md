<p align="right"><strong>English</strong> ｜ <a href="SENTRITH_CLI.ja.md">日本語</a></p>

# Sentrith CLI

The Sentrith CLI is a small Rust utility for deterministic project checks and usage measurement.

It is intentionally **not an AI agent**.

## Goals

- no extra model calls for routine checks
- no Python runtime requirement for end users
- prebuilt binaries where practical
- deterministic local/CI behavior
- provider network access only when explicitly requested for usage snapshots

## Core commands

```bash
sentrith preflight
sentrith closeout-check
sentrith guard
sentrith review-hint
sentrith diff-budget
```

## Usage commands

Manual/import fallback:

```bash
sentrith usage record ...
```

Compare measured data:

```bash
sentrith hooks install
sentrith usage status
sentrith usage baseline start
sentrith usage baseline stop

sentrith usage report --compare
sentrith usage report --tasks
sentrith usage report --churn --days 14
```

Run machine-readable CLI adapters where supported:

```bash
sentrith usage run codex --task "fix login" -- "Fix login."
sentrith usage run copilot --task "add export" -- -p "Add CSV export."
```

Task-ledger measurement for IDE/Desktop workflows:

```bash
sentrith usage task start --agent copilot --task "login bug" ...
sentrith usage task stop --success yes
```

Provider snapshot:

```bash
sentrith usage snapshot copilot --github-user USER [--org ORG]
```

Community contribution:

```bash
sentrith usage contribute --agent copilot --model "<model>"
sentrith usage aggregate
sentrith usage aggregate --publish
```

## Network policy

Most Sentrith commands are local and deterministic.

A provider snapshot command may call a documented provider API when the user explicitly requests it. It should not silently send repository content, prompts, code, or transcripts.

## Private usage data

Local raw records belong under:

```text
.ai-usage/
```

This directory is ignored by Git by default.

Only anonymized aggregate contribution records belong in the public community benchmark directory.

## Success is not the same as exit code

A process exiting successfully is not sufficient evidence that an engineering task satisfied its acceptance criteria.

Usage measurement and task-quality evaluation remain separate concerns.
