<p align="right"><strong>English</strong> ｜ <a href="README.ja.md">日本語</a></p>

# Sentrith Usage Measurement

Sentrith does not assume usage improves. It measures it.

## Start here

You can begin before reading anything else:

```bash
sentrith hooks install
sentrith usage baseline start
```

Then **just work normally**. Nothing is recorded by hand.

When you have enough baseline tasks:

```bash
sentrith usage baseline stop
```

To see where you are and what to do next, at any time:

```bash
sentrith usage status
```

Once the data is comparable, `status` prints the commands to run.

---

## What those commands do

| Command | Effect |
|---|---|
| `hooks install` | Idempotently merges only Sentrith's hooks into `.claude/settings.json` / `.codex/hooks.json`. Your other settings, other hooks, and a custom statusLine are preserved. No manual JSON editing. |
| `usage baseline start` | Stashes the agent instruction files (`AGENTS.md` and friends) into `.sentrith-private/baseline-stash/` and records subsequent turns as `baseline`. Measurement hooks and data stay active. |
| `usage baseline stop` | Restores the stashed files and returns recording to `standard`. |

A baseline has to be measured with Sentrith inactive, which is why this one step is explicit.

After `baseline start` or `stop`, **start a new agent session**: the previous instructions are still in the old session's context.

---

## What gets recorded

Hooks append one row per turn to `.ai-usage/usage.csv`:

- tokens and model — parsed from the transcript
- cost and duration — from the statusLine (fallback)
- success — derived from commit and test evidence; `unknown` when undecidable
- head_sha — used to find task boundaries

No extra model calls are made.

---

## Reading order

If you want the details:

1. [`MEASUREMENT_ARCHITECTURE.en.md`](MEASUREMENT_ARCHITECTURE.en.md) — task boundaries and the success definition
2. [`AUTO_CAPTURE.en.md`](AUTO_CAPTURE.en.md) — per-agent capture
3. [`PROVIDER_ADAPTERS.en.md`](PROVIDER_ADAPTERS.en.md) — provider differences
4. [`BENCHMARK_GUIDE.en.md`](BENCHMARK_GUIDE.en.md) — comparison design and sample sizes
5. [`COMMUNITY_BENCHMARK.en.md`](COMMUNITY_BENCHMARK.en.md) — anonymized contributions
6. [`AUTO_PUBLISH.en.md`](AUTO_PUBLISH.en.md) — publishing to the README

---

## Where data lives

Private raw usage:

```text
.ai-usage/
```

Public community data:

```text
docs/metrics/contributions/
```

Raw usage is private and ignored by Git by default.

Only anonymized aggregate contribution files belong in the public benchmark dataset.
