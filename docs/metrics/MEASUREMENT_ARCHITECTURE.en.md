<p align="right"><strong>English</strong> ｜ <a href="MEASUREMENT_ARCHITECTURE.ja.md">日本語</a></p>

# Measurement Architecture

Sentrith treats usage measurement as a **UI-independent model**, not as a CLI-wrapper feature.

## Core model

```text
Task Ledger
+
Provider Usage
+
Success / Rework
=
Usage per successful task
```

Whether the developer uses an IDE, desktop app, CLI, or web UI is secondary.

## Task Ledger

Sentrith records the engineering work boundary:

- task start / stop
- baseline / Sentrith phase
- agent / model
- task category
- success
- rework
- duration

This lets provider consumption be attributed to meaningful engineering work.

## Provider Usage

Prefer the provider's documented usage/billing surface as source of truth.

Examples:

- GitHub Copilot → AI Credits API / export / manual snapshot
- Claude Code → documented status/hook cost data
- Codex → documented hooks / machine-readable usage
- Gemini → documented usage/billing/export surfaces

Avoid making private APIs, network interception, OCR, or UI scraping the default strategy.

## Native metrics

Keep provider-native units:

```text
Copilot -> AI Credits
Claude  -> estimated USD / tokens
Codex   -> tokens / provider-native usage
Gemini  -> tokens / provider-native cost
```

Do not invent a universal credit.

## Cross-provider comparison

Normalize each environment against its own baseline:

```text
baseline usage / successful task = 100
```

Then compare the relative change after Sentrith.

Community aggregation uses the median baseline-relative change.

## Why “successful task”?

A lower `usage / task` can be misleading if failure/rework increases.

The primary cost metric is therefore:

> **usage / successful task**

Always inspect success rate, rework, and relevant quality metrics alongside it.

## Measurement is evidence, not causality

An observed improvement does not prove Sentrith alone caused it.

Task mix, model changes, cache behavior, developer learning, provider changes, and repository evolution can all affect results.

Publish the measurement conditions with the result.
