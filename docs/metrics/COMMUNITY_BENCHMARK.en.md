<p align="right"><strong>English</strong> ｜ <a href="COMMUNITY_BENCHMARK.ja.md">日本語</a></p>

# Community Benchmark

## Goal

Sentrith should not rely only on maintainer claims about usage savings.

Community measurements let real users contribute comparable, anonymized results.

## Contribution

Generate a public contribution:

```bash
sentrith usage contribute --agent copilot --model "<model>"
```

Default qualification:

- baseline >= 10 tasks
- Sentrith >= 10 tasks
- task success outcomes recorded
- the same native usage metric exists in both phases

Smaller samples may be exported as experimental data, but they are excluded from the default qualified aggregate.

## Privacy

A contribution contains aggregate benchmark values only.

It must not contain:

- task labels
- prompts
- repository names
- customer/company names
- file paths
- source code
- transcripts
- session IDs
- Git remotes

Raw `.ai-usage/usage.csv` remains local/private.

## Aggregation

```bash
sentrith usage aggregate
sentrith usage aggregate --publish
```

Default aggregation includes qualified contributions only.

Different provider units are **not** added directly.

Instead, each environment computes its baseline-relative change in:

```text
usage / successful task
```

The Community Benchmark uses the median normalized change.

## README publication

At minimum, publish:

- number of contributions
- baseline task count
- Sentrith task count
- median normalized usage / successful task
- success-rate context
- explicit “community-reported” labeling

Keep maintainer-controlled benchmarks separate from community-reported benchmarks.
