<p align="right"><strong>English</strong> ｜ <a href="AUTO_PUBLISH.ja.md">日本語</a></p>

# Automatic benchmark aggregation and README publishing

`sentrith usage publish` aggregates recorded usage data and updates both README languages without any extra model/API call.

Preview:

```bash
sentrith usage publish \
  --agent codex \
  --model "<model>" \
  --task-mix "bugfix + small feature" \
  --date YYYY-MM-DD \
  --dry-run
```

Publish:

```bash
sentrith usage publish \
  --agent codex \
  --model "<model>" \
  --task-mix "bugfix + small feature" \
  --date YYYY-MM-DD
```

By default publication requires at least 5 baseline and 5 standard tasks. Change the threshold with:

```bash
--min-samples 10
```

`--force` bypasses the threshold but automatically adds a small-sample warning.

Only content between these markers is modified:

```text
<!-- SENTRITH-USAGE-BENCHMARK:BEGIN -->
<!-- SENTRITH-USAGE-BENCHMARK:END -->
```

The rest of the README is preserved.

Vendor-specific credit/token retrieval is intentionally not automatic because there is no single portable cross-agent source. Aggregation, threshold checks, bilingual rendering, and README updates are automatic.
