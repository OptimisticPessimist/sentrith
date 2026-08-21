<p align="right"><strong>English</strong> ｜ <a href="DATA.ja.md">日本語</a></p>

# Data Science / Data Engineering Profile

This profile covers whether **results reproduce** and whether **data broke silently**.

## Applies when

```text
ETL/ELT pipelines, DAGs, schedulers
dataset generation, transformation, joins
analysis notebooks, aggregation logic, metric definitions
data schema changes, backfills
data quality checks, missing/duplicate handling
```

## Verification dimensions added

- **Reproducibility** — same input yields same output (random seeds, execution order, time zones)
- **Leakage check** — no future information or target variable leaking across train/eval splits
- **Schema contract** — upstream schema changes do not break downstream consumers
- **Row-level integrity** — row counts, grain, unintended fan-out from joins
- **Idempotency / backfill safety** — re-runs do not double-count
- **Provenance** — dataset version, generation time, and the code that produced it

## Technique gates

| Technique | Apply when | Skip when |
|---|---|---|
| **Data contract tests** | You do not control the upstream producer | A closed transformation inside one repo |
| **Quality assertions (great-expectations style)** | Production pipeline where degradation creeps in silently | One-off investigation |
| **Property-based testing** | Transformation functions with a wide input space | Representative cases suffice |
| **Statistical review** | Metric movement is hard to separate from noise | The change is structural and obvious |
| **Snapshot / golden dataset** | Output is large and diff review is impractical | Output is small enough to inspect |

## Definition of correct

```text
the pipeline finished
≠
the data is right
```

Make at least these observable:

- output row count and grain match expectations
- re-running produces the same result, or the reason it differs is stated
- missing values, duplicates, and type coercions are handled explicitly
- backfills do not corrupt existing correct rows

## Notebooks

Notebooks are good for exploration but weak for reproducibility, because **execution order carries hidden state**.

Move durable logic out of notebooks into modules where it can be verified.
