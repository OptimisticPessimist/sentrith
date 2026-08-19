<p align="right"><strong>English</strong> ｜ <a href="README_UPDATE_GUIDE.ja.md">日本語</a></p>

# Updating the README with measured results

Do not replace the illustrative README sample with measured claims until you have a meaningful sample.

Recommended minimum:

```text
5 baseline + 5 standard tasks
```

Prefer 10 + 10 or more.

When publishing, include a compact condition block:

```md
Measured with:

- Agent: OpenAI Codex
- Model: <model>
- Baseline: 10 tasks
- Standard: 10 tasks
- Task mix: 6 bug fixes / 4 small features
- Date: YYYY-MM-DD
```

Then use a compact table:

```md
| Metric | Baseline | Standard | Change |
|---|---:|---:|---:|
| Credits / task | 14.2 | 9.8 | -31.0% |
| Tool calls / task | 20.8 | 15.0 | -27.9% |
| Rework / task | 0.9 | 0.5 | -44.4% |
| Success rate | 90% | 100% | +10pp |
```

Checklist:

- sample size shown
- agent/model shown
- measurement date shown
- baseline/standard definitions shown
- success rate included
- Japanese and English numbers match
- no confidential task or repository information leaked
- claims are scoped to the measured benchmark

Prefer:

> In this benchmark...

Avoid:

> This always reduces credits by 30%.
