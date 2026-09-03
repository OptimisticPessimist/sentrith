<p align="right"><strong>English</strong> ｜ <a href="BENCHMARK_GUIDE.ja.md">日本語</a></p>

# Usage Benchmark Guide

Use this procedure to measure whether the standard actually reduces coding-agent usage without reducing engineering quality.

## Recommended experiment

Collect at least:

```text
baseline: 5–10+ tasks
standard: 5–10+ tasks
```

Use comparable task categories rather than repeating the exact same task when cache/history would bias the second run.

Record available metrics with:

```bash
sentrith usage record ...
```

With hooks enabled, capture is automatic. To measure a baseline, switch the contract off:

```bash
sentrith usage baseline start
```

Agent instruction files move to `.sentrith-private/baseline-stash/` and later turns record as `baseline`, while measurement keeps running. **Start a new agent session**, then work normally. When you have enough tasks:

```bash
sentrith usage baseline stop
```

Check progress at any time with `sentrith usage status`.

Phase precedence is `--phase` > `.ai-usage/phase` (the marker these commands write) > `SENTRITH_PHASE` > `standard`. The marker outranks the environment because hooks are spawned by the agent process: a variable exported after the agent started never reaches them.

Compare with:

```bash
sentrith usage report --compare
sentrith usage report --tasks
sentrith usage report --churn --days 14
```

`--tasks` aggregates captured turns into tasks; `--churn` computes the rework proxy from git history.

## Primary metrics

Prefer:

- credits per successful task
- credits per task
- input/cached/output tokens when available
- tool calls per task
- rework per task
- success rate
- duration when measured consistently

Do not call a workflow cheaper if token usage falls but success rate or rework becomes materially worse.

## Keep comparisons controlled

Whenever possible compare:

```text
same agent
same model
similar task mix
```

Keep different agents/models as separate datasets.

## Publishing results

When publishing benchmark results in the README, include:

- agent
- model
- sample size
- task mix/repository category
- measurement date
- baseline definition
- standard definition
- success rate

Use qualified language such as:

> In this benchmark...

Do not claim universal credit savings from a single repository or small sample.

See `README_UPDATE_GUIDE.en.md` for publication guidance.
