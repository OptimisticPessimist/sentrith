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

Compare with:

```bash
sentrith usage report --compare
```

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
