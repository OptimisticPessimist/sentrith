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

Besides the explicit ledger written by `usage task start` / `stop`, hook-based capture **derives** the work boundary automatically:

```text
1 turn = one usage.csv row
1 task = the turns of a session up to a head_sha change
```

`sentrith usage report --tasks` aggregates turns into tasks using that rule.

## Success semantics (automatic capture)

For hook-captured rows, success comes from **repository evidence, not human judgment**:

```text
commit reached + last test run green -> yes
commit reached + last test run red   -> no
otherwise (nothing to decide on)     -> unknown
```

Evidence used:

- **commit reached**: HEAD at `Stop` differs from HEAD at `UserPromptSubmit`
- **test outcome**: success/failure of test-runner commands (`cargo test`, `pytest`, `npm test`, …) executed during the turn

`unknown` is not failure. It is **excluded from the success-rate denominator**:

```text
success rate = yes / (yes + no)
```

This prevents automatic capture from deflating the rate. The unknown count is always reported alongside it.

The definition is reproducible, but it is not the same as "did a human consider this successful". Correct work that ended without a commit is recorded as unknown.

## Rework (churn proxy)

Rework is not instrumented at capture time; it is **computed retroactively from git history**:

```bash
sentrith usage report --churn --days 14
```

For each recorded commit it reports the share of files touched by that commit which were modified again within N days.

This is a file-level proxy, not line-level attribution.

## Phase (baseline / standard)

Switch phases with the dedicated commands:

```bash
sentrith usage baseline start
sentrith usage baseline stop
```

Precedence:

```text
--phase > .ai-usage/phase (marker) > SENTRITH_PHASE > standard
```

The marker outranks the environment variable because hooks are spawned by the agent process: a variable exported after the agent started never reaches them, while the marker always can be read.

Baseline measurement requires running without the Sentrith contract, so switching is an explicit step.

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
